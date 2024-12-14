use std::{fmt::Display, str::FromStr, sync::Arc};

use flax::{component, Component, Entity, EntityRef};
use futures::{stream::BoxStream, StreamExt};
use futures_signals::signal::Mutable;
use glam::Vec2;
use palette::Srgba;
use winit::event::ElementState;

use super::input::TextInput;
use crate::{
    components::{offset, padding, rect},
    input::{focusable, on_cursor_move, on_mouse_input},
    layout::Align,
    state::{State, StateDuplex, StateSink, StateStream},
    style::{interactive_active, interactive_passive, spacing_small, SizeExt},
    to_owned,
    unit::Unit,
    utils::zip_latest,
    widget::{row, Float, Positioned, Rectangle, Stack, StreamWidget, Text},
    Scope, StreamEffect, Widget,
};

#[derive(Debug, Clone, Copy)]
pub struct SliderStyle {
    pub track_color: Component<Srgba>,
    pub handle_color: Component<Srgba>,
    pub track_size: Unit<Vec2>,
    pub handle_size: Unit<Vec2>,
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            track_color: interactive_passive(),
            handle_color: interactive_active(),
            track_size: Unit::px2(256.0, 4.0),
            handle_size: Unit::px2(4.0, 16.0),
        }
    }
}

pub struct Slider<V> {
    style: SliderStyle,
    value: Arc<dyn Send + Sync + StateDuplex<Item = V>>,
    min: V,
    max: V,
    transform: Option<Box<dyn Send + Sync + Fn(V) -> V>>,
    scrub_mode: bool,
}

impl<V> Slider<V> {
    pub fn new(value: impl 'static + Send + Sync + StateDuplex<Item = V>, min: V, max: V) -> Self
    where
        V: Copy,
    {
        Self {
            value: Arc::new(value),
            min,
            max,
            style: Default::default(),
            transform: None,
            scrub_mode: false,
        }
    }

    /// Set the style
    pub fn with_style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the transform
    pub fn with_transform(mut self, transform: impl 'static + Send + Sync + Fn(V) -> V) -> Self {
        self.transform = Some(Box::new(transform));
        self
    }

    pub fn with_scrub_mode(mut self, scrub_mode: bool) -> Self {
        self.scrub_mode = scrub_mode;
        self
    }
}

impl<V: SliderValue> Widget for Slider<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let track_color = stylesheet
            .get_copy(self.style.track_color)
            .unwrap_or_default();
        let handle_color = stylesheet
            .get_copy(self.style.handle_color)
            .unwrap_or_default();

        let handle_size = self.style.handle_size;
        let track_size = self.style.track_size;

        let track = scope.attach(
            Rectangle::new(track_color)
                .with_min_size(track_size)
                .with_size(track_size),
        );

        let min = self.min.to_progress();
        let max = self.max.to_progress();

        fn get_progress_value<V: SliderValue>(
            entity: &EntityRef,
            track_pos: f32,
            min: f32,
            max: f32,
        ) -> V {
            let rect = entity.get_copy(rect()).unwrap();
            let padding = entity.get_copy(padding()).unwrap_or_default();

            let value = ((track_pos - padding.left) / (rect.size().x - padding.size().x))
                .clamp(0.0, 1.0)
                * (max - min)
                + min;

            V::from_progress(value)
        }

        fn get_slider_position(entity: &EntityRef, progress: f32, min: f32, max: f32) -> f32 {
            let rect = entity.get_copy(rect()).unwrap();
            let padding = entity.get_copy(padding()).unwrap_or_default();
            let size = rect.size().x - padding.size().x;

            (progress - min) * size / (max - min) + padding.left
        }
        fn update_scrubbed<V: SliderValue>(
            entity: &EntityRef,
            drag_distance: f32,
            start_value: f32,
            min: f32,
            max: f32,
        ) -> V {
            let rect = entity.get_copy(rect()).unwrap();
            let padding = entity.get_copy(padding()).unwrap_or_default();

            let value = (drag_distance / (rect.size().x - padding.size().x)) * (max - min) + min;

            V::from_progress((start_value + value).clamp(min, max))
        }

        let handle = SliderHandle {
            value: self.value.stream(),
            min,
            max,
            rect_id: track,
            handle_color,
            handle_size,
        };

        let value = Arc::new(self.value.map(
            |v| v,
            move |v| self.transform.as_ref().map(|f| f(v)).unwrap_or(v),
        ));

        let drag_start = Mutable::new(None as Option<(f32, f32)>);

        scope.spawn_stream(value.stream(), |scope, value| {
            scope.set(current_value(), value.to_progress());
        });

        scope
            .set(focusable(), ())
            .on_event(on_mouse_input(), {
                to_owned![value, drag_start];
                move |scope, input| {
                    if input.state == ElementState::Pressed {
                        let progress =
                            get_progress_value(scope, input.cursor.local_pos.x, min, max);
                        if let Ok(current_value) = scope.get(current_value()) {
                            let pos = get_slider_position(scope, *current_value, min, max);

                            if (pos - input.cursor.local_pos.x).abs() < 16.0 {
                                drag_start.set(Some((input.cursor.local_pos.x, *current_value)));
                                return;
                            }
                        }

                        value.send(progress);
                    }
                }
            })
            .on_event(on_cursor_move(), {
                to_owned![value];
                move |scope, input| {
                    let drag_start = &mut *drag_start.lock_mut();
                    let progress = if let Some((start, start_value)) = drag_start {
                        update_scrubbed(scope, input.local_pos.x - *start, *start_value, min, max)
                    } else {
                        get_progress_value(scope, input.local_pos.x, min, max)
                    };

                    value.send(progress);
                }
            });

        Stack::new(Float::new(handle))
            .with_min_size(handle_size)
            .with_vertical_alignment(Align::Center)
            .with_padding(spacing_small())
            .with_margin(spacing_small())
            .mount(scope)
    }
}

struct SliderHandle<V> {
    value: BoxStream<'static, V>,
    handle_color: Srgba,
    handle_size: Unit<Vec2>,
    min: f32,
    max: f32,
    rect_id: Entity,
}

impl<V: SliderValue> Widget for SliderHandle<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        let rect_size = Mutable::new(None);

        let update = zip_latest(self.value, rect_size.stream());

        scope.frame_mut().monitor(self.rect_id, rect(), move |v| {
            rect_size.set(v.map(|v| v.size()));
        });

        scope.spawn_effect(StreamEffect::new(update, {
            move |scope: &mut Scope<'_>, (value, size): (V, Option<Vec2>)| {
                if let Some(size) = size {
                    let pos = (value.to_progress() - self.min) * size.x / (self.max - self.min);

                    scope.entity().update_dedup(offset(), Unit::px2(pos, 0.0));
                }
            }
        }));

        Positioned::new(Rectangle::new(self.handle_color).with_min_size(self.handle_size))
            .with_offset(Unit::px2(0.0, 0.0))
            .with_anchor(Unit::rel2(0.5, 0.5))
            .mount(scope)
    }
}

pub trait SliderValue:
    'static + Send + Sync + Copy + std::fmt::Display + PartialEq + PartialOrd
{
    fn from_progress(v: f32) -> Self;
    fn to_progress(&self) -> f32;
}

impl SliderValue for f32 {
    fn from_progress(v: f32) -> Self {
        v
    }

    fn to_progress(&self) -> f32 {
        *self
    }
}

macro_rules! num_impl {
    ($ty: ty) => {
        impl SliderValue for $ty {
            fn from_progress(v: f32) -> Self {
                v.round() as $ty
            }

            fn to_progress(&self) -> f32 {
                *self as f32
            }
        }
    };
}

num_impl!(i8);
num_impl!(u8);
num_impl!(i16);
num_impl!(u16);
num_impl!(i32);
num_impl!(u32);
num_impl!(i64);
num_impl!(u64);
num_impl!(isize);
num_impl!(usize);

/// A slider with label displaying the value
pub struct SliderWithLabel<V> {
    slider: Slider<V>,
    editable: bool,
    rounding: Option<f32>,
    min: V,
    max: V,
    value: Arc<dyn Send + Sync + StateDuplex<Item = V>>,
}

impl<V: SliderValue + FromStr + Display + Default + PartialOrd> SliderWithLabel<V> {
    pub fn new(value: impl 'static + Send + Sync + StateDuplex<Item = V>, min: V, max: V) -> Self
    where
        V: Copy,
    {
        // Wrap in dedup to prevent updating equal numeric values like `0` and `0.` etc when typing
        let value = Arc::new(value);

        Self {
            value: value.clone(),
            min,
            max,
            slider: Slider {
                style: Default::default(),
                value,
                min,
                max,
                transform: None,
                scrub_mode: false,
            },
            editable: false,
            rounding: None,
        }
    }

    /// Set the style
    pub fn with_style(mut self, style: SliderStyle) -> Self {
        self.slider = self.slider.with_style(style);
        self
    }

    pub fn with_transform(mut self, transform: impl 'static + Send + Sync + Fn(V) -> V) -> Self {
        self.slider.transform = Some(Box::new(transform));
        self
    }

    pub fn with_scrub_mode(mut self, scrub_mode: bool) -> Self {
        self.slider.scrub_mode = scrub_mode;
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }
}

impl SliderWithLabel<f32> {
    pub fn round(mut self, round: f32) -> Self {
        let recip = round.recip();
        self.rounding = Some(round);
        let x = move |v: f32| (v * recip).round() / recip;

        self.slider.transform = Some(Box::new(x));
        self.value = Arc::new(self.value.map(x, |v| v));
        self
    }

    pub fn round_digits(mut self, round: u32) -> Self {
        self.rounding = Some(10i32.pow(round) as f32);
        let x = move |v: f32| (v * 10i32.pow(round) as f32).round() / 10i32.pow(round) as f32;

        self.slider.transform = Some(Box::new(x));
        self.value = Arc::new(self.value.map(x, |v| v));
        self
    }
}

impl<V: SliderValue + FromStr + Display + Default + PartialOrd + Copy> Widget
    for SliderWithLabel<V>
{
    fn mount(self, scope: &mut Scope<'_>) {
        let text_value = Box::new(self.value.clone().dedup().prevent_feedback().filter_map(
            move |v: V| Some(format!("{v}")),
            move |v| {
                v.parse::<V>().ok().map(|v| {
                    if v < self.min {
                        self.min
                    } else if v > self.max {
                        self.max
                    } else {
                        v
                    }
                })
            },
        ));

        if self.editable {
            row((self.slider, TextInput::new(text_value)))
                .with_cross_align(Align::Center)
                .mount(scope)
        } else {
            row((
                self.slider,
                StreamWidget(text_value.stream().map(Text::new)),
            ))
            .with_cross_align(Align::Center)
            .mount(scope)
        }
    }
}

component! {
    current_value: f32,
    scrubbing: bool,
}
