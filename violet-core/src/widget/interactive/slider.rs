use std::{fmt::Display, str::FromStr, sync::Arc};

use flax::{component, Entity, EntityRef};
use futures::{stream::BoxStream, StreamExt};
use futures_signals::signal::Mutable;
use glam::Vec2;
use palette::Srgba;
use winit::event::ElementState;

use crate::{
    components::{anchor, min_size, offset, padding, rect},
    input::{interactive, on_cursor_move, on_mouse_input},
    layout::Align,
    state::{StateDuplex, StateExt, StateSink, StateStream},
    style::{
        default_corner_radius, element_accent, spacing_small, surface_interactive, ResolvableStyle,
        SizeExt, ValueOrRef,
    },
    to_owned,
    unit::Unit,
    utils::zip_latest,
    widget::{row, Float, InputBox, Rectangle, Stack, StreamWidget, Text},
    Edges, Scope, StreamEffect, Widget,
};

#[derive(Debug, Clone, Copy)]
pub struct SliderStyle {
    pub track_color: ValueOrRef<Srgba>,
    pub fill_color: ValueOrRef<Srgba>,
    pub handle_color: ValueOrRef<Srgba>,
    pub track_size: Unit<Vec2>,
    pub handle_size: Unit<Vec2>,
    pub handle_corner_radius: Unit<f32>,
    pub fill: bool,
}

impl SliderStyle {
    fn with_fill_color(mut self, color: impl Into<ValueOrRef<Srgba>>) -> Self {
        let color = color.into();
        self.fill_color = color;
        self.handle_color = color;
        self
    }
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            track_color: surface_interactive().into(),
            handle_color: element_accent().into(),
            fill_color: element_accent().into(),
            track_size: Unit::px2(256.0, 4.0),
            handle_size: Unit::px2(12.0, 12.0),
            handle_corner_radius: Unit::rel(1.0),
            fill: true,
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

    pub fn with_fill_color(mut self, color: impl Into<ValueOrRef<Srgba>>) -> Self {
        self.style = self.style.with_fill_color(color);
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

        let track_color = self.style.track_color.resolve(stylesheet);
        let fill_color = self.style.fill_color.resolve(stylesheet);
        let handle_color = self.style.handle_color.resolve(stylesheet);

        let handle_size = self.style.handle_size;
        let track_size = self.style.track_size;

        let track = scope.attach(
            Rectangle::new(track_color)
                .with_exact_size(track_size)
                .with_corner_radius(default_corner_radius()),
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

        if self.style.fill {
            let fill = SliderFill {
                value: self.value.stream(),
                min,
                max,
                rect_id: track,
                height: self.style.track_size.px.y,
                color: fill_color,
            };
            scope.attach(Float::new(fill));
        }

        let handle = SliderHandle {
            value: self.value.stream(),
            min,
            max,
            rect_id: track,
            corner_radius: self.style.handle_corner_radius,
            handle_color,
            handle_size,
        };

        scope.attach(Float::new(handle));

        let value = Arc::new(self.value.map_value(
            |v| v,
            move |v| self.transform.as_ref().map(|f| f(v)).unwrap_or(v),
        ));

        let drag_start = Mutable::new(None as Option<(f32, f32)>);

        scope.spawn_stream(value.stream(), |scope, value| {
            scope.set(current_value(), value.to_progress());
        });

        scope
            .set(interactive(), ())
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
                                return None;
                            }
                        }

                        value.send(progress);
                    }

                    None
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

                    None
                }
            });

        let handle_size = self.style.handle_size.px;

        Stack::new(())
            .with_min_size(Unit::px(handle_size))
            .with_vertical_alignment(Align::Center)
            .with_padding(
                Edges::new(
                    handle_size.x / 2.0,
                    handle_size.x / 2.0,
                    handle_size.y / 2.0,
                    handle_size.y / 2.0,
                )
                .max(
                    scope
                        .stylesheet()
                        .get_copy(spacing_small())
                        .unwrap_or_default(),
                ),
            )
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
    corner_radius: Unit<f32>,
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
                    let pos = (value.to_progress().clamp(self.min, self.max) - self.min) * size.x
                        / (self.max - self.min);

                    scope.entity().update_dedup(offset(), Unit::px2(pos, 0.0));
                }
            }
        }));

        scope
            .set_default(offset())
            .set(anchor(), Unit::rel2(0.5, 0.5));

        Rectangle::new(self.handle_color)
            .with_min_size(self.handle_size)
            .with_corner_radius(self.corner_radius)
            .mount(scope)
    }
}

struct SliderFill<V> {
    value: BoxStream<'static, V>,
    height: f32,
    color: Srgba,
    min: f32,
    max: f32,
    rect_id: Entity,
}

impl<V: SliderValue> Widget for SliderFill<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        let rect_size = Mutable::new(None);

        let update = zip_latest(self.value, rect_size.stream());

        scope.frame_mut().monitor(self.rect_id, rect(), move |v| {
            rect_size.set(v.map(|v| v.size()));
        });

        scope.spawn_effect(StreamEffect::new(update, {
            move |scope: &mut Scope<'_>, (value, outer_size): (V, Option<Vec2>)| {
                if let Some(outer_size) = outer_size {
                    let pos = (value.to_progress().clamp(self.min, self.max) - self.min)
                        * outer_size.x
                        / (self.max - self.min);

                    let entity = scope.entity();
                    entity.update_dedup(min_size(), Unit::px2(pos, self.height));
                    // scope.entity().update_dedup(offset, Unit::px2(pos, 0.0));
                }
            }
        }));

        scope
            .set_default(offset())
            .set(anchor(), Unit::rel2(0.0, 0.5));

        Rectangle::new(self.color)
            .with_min_size(Unit::px2(10.0, self.height))
            .with_corner_radius(default_corner_radius())
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

impl SliderValue for f64 {
    fn from_progress(v: f32) -> Self {
        v as f64
    }

    fn to_progress(&self) -> f32 {
        *self as f32
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
pub struct LabeledSlider<V> {
    slider: Slider<V>,
    editable: bool,
    rounding: Option<f32>,
    min: V,
    max: V,
    value: Arc<dyn Send + Sync + StateDuplex<Item = V>>,
}

impl<V: SliderValue + FromStr + Display + Default + PartialOrd> LabeledSlider<V> {
    pub fn input(value: impl 'static + Send + Sync + StateDuplex<Item = V>, min: V, max: V) -> Self
    where
        V: Copy,
    {
        Self::new(value, min, max).editable(true)
    }

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

    /// Enable scrub mode
    ///
    /// Scrub mode allows dragging the slider handle anywhere on the screen to change the value
    pub fn with_scrub_mode(mut self, scrub_mode: bool) -> Self {
        self.slider.scrub_mode = scrub_mode;
        self
    }

    pub fn with_fill_color(mut self, color: impl Into<ValueOrRef<Srgba>>) -> Self {
        self.slider = self.slider.with_fill_color(color);
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }
}

impl LabeledSlider<f32> {
    pub fn precision(mut self, round: u32) -> Self {
        self.rounding = Some(10i32.pow(round) as f32);
        let x = move |v: f32| (v * 10i32.pow(round) as f32).round() / 10i32.pow(round) as f32;

        self.slider.transform = Some(Box::new(x));
        self.value = Arc::new(self.value.map_value(x, |v| v));
        self
    }
}

impl<V> Widget for LabeledSlider<V>
where
    V: SliderValue + FromStr + Display + Default + PartialOrd + Copy,
    V::Err: 'static + std::fmt::Display,
{
    fn mount(self, scope: &mut Scope<'_>) {
        let text_value = Box::new(self.value.clone().prevent_feedback().map_value(
            move |v: V| v,
            move |v| {
                if v < self.min {
                    self.min
                } else if v > self.max {
                    self.max
                } else {
                    v
                }
            },
        ));

        if self.editable {
            row((self.slider, InputBox::<V>::new(text_value)))
                .with_cross_align(Align::Center)
                .mount(scope)
        } else {
            row((
                self.slider,
                StreamWidget(self.value.stream().map(|v| Text::new(v.to_string()))),
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
