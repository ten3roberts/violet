use std::{fmt::Display, str::FromStr, sync::Arc};

use cosmic_text::Wrap;
use flax::{Component, Entity, EntityRef};
use futures::{stream::BoxStream, StreamExt};
use futures_signals::signal::Mutable;
use glam::{IVec2, Vec2};
use palette::{num::Recip, Srgba};
use winit::event::ElementState;

use crate::{
    components::{offset, rect},
    input::{focusable, on_cursor_move, on_mouse_input, CursorMove},
    layout::Alignment,
    state::{Dedup, FilterDuplex, Map, StateDuplex, StateStream},
    style::{get_stylesheet, interactive_active, interactive_inactive, spacing, StyleExt},
    text::TextSegment,
    to_owned,
    unit::Unit,
    utils::zip_latest,
    widget::{
        row, BoxSized, ContainerStyle, Positioned, Rectangle, Stack, StreamWidget, Text, WidgetExt,
    },
    Edges, Scope, StreamEffect, Widget,
};

use super::input::TextInput;

#[derive(Debug, Clone, Copy)]
pub struct SliderStyle {
    pub track_color: Component<Srgba>,
    pub handle_color: Component<Srgba>,
    pub track_size: Unit<IVec2>,
    pub handle_size: Unit<IVec2>,
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            track_color: interactive_inactive(),
            handle_color: interactive_active(),
            track_size: Unit::px2i(64, 1),
            handle_size: Unit::px2i(1, 4),
        }
    }
}

pub struct Slider<V> {
    style: SliderStyle,
    value: Arc<dyn Send + Sync + StateDuplex<Item = V>>,
    min: V,
    max: V,
    transform: Option<Box<dyn Send + Sync + Fn(V) -> V>>,
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
}

impl<V: SliderValue> Widget for Slider<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = get_stylesheet(scope);

        let track_color = stylesheet
            .get_copy(self.style.track_color)
            .unwrap_or_default();
        let handle_color = stylesheet
            .get_copy(self.style.handle_color)
            .unwrap_or_default();

        let spacing = stylesheet.get_copy(spacing()).unwrap_or_default();

        let handle_size = spacing.size(self.style.handle_size);
        let track_size = spacing.size(self.style.track_size);

        let track = scope.attach(BoxSized::new(Rectangle::new(track_color)).with_size(track_size));

        let min = self.min.to_progress();
        let max = self.max.to_progress();

        fn update<V: SliderValue>(
            entity: &EntityRef,
            input: CursorMove,
            min: f32,
            max: f32,
            dst: &dyn StateDuplex<Item = V>,
        ) {
            let rect = entity.get_copy(rect()).unwrap();
            let value = (input.local_pos.x / rect.size().x).clamp(0.0, 1.0) * (max - min) + min;
            dst.send(V::from_progress(value));
        }

        let handle = SliderHandle {
            value: self.value.stream(),
            min,
            max,
            rect_id: track,
            handle_color,
            handle_size,
        };

        let value = Arc::new(Map::new(
            self.value,
            |v| v,
            move |v| {
                if let Some(transform) = &self.transform {
                    transform(v)
                } else {
                    v
                }
            },
        ));

        scope
            .set(focusable(), ())
            .on_event(on_mouse_input(), {
                to_owned![value];
                move |_, entity, input| {
                    if input.state == ElementState::Pressed {
                        update(entity, input.cursor, min, max, &*value);
                    }
                }
            })
            .on_event(on_cursor_move(), {
                to_owned![value];
                move |_, entity, input| update(entity, input, min, max, &*value)
            });

        let slider = Stack::new(handle)
            .with_vertical_alignment(Alignment::Center)
            .with_style(ContainerStyle {
                margin: Edges::even(5.0),
                ..Default::default()
            });

        slider.mount(scope)
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

        Positioned::new(
            BoxSized::new(Rectangle::new(self.handle_color)).with_min_size(self.handle_size),
        )
        .with_anchor(Unit::rel2(0.5, 0.0))
        .mount(scope)
    }
}

pub trait SliderValue: 'static + Send + Sync + Copy + std::fmt::Display {
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
    text_value: Box<dyn 'static + Send + Sync + StateDuplex<Item = String>>,
    slider: Slider<V>,
    editable: bool,
}

impl<V: SliderValue + FromStr + Display + Default + PartialOrd> SliderWithLabel<V> {
    pub fn new(value: impl 'static + Send + Sync + StateDuplex<Item = V>, min: V, max: V) -> Self
    where
        V: Copy,
    {
        // Wrap in dedup to prevent updating equal numeric values like `0` and `0.` etc when typing
        let value = Arc::new(value);

        let text_value = Box::new(FilterDuplex::new(
            Dedup::new(value.clone()),
            |v| Some(format!("{v}")),
            move |v: String| {
                let v = v.parse::<V>().ok()?;
                let v = if v < min {
                    min
                } else if v > max {
                    max
                } else {
                    v
                };

                Some(v)
            },
        ));

        Self {
            text_value,
            slider: Slider {
                style: Default::default(),
                value,
                min,
                max,
                transform: None,
            },
            editable: false,
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

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }
}

impl SliderWithLabel<f32> {
    pub fn round(mut self, round: f32) -> Self {
        let recip = round.recip();
        self.slider.transform = Some(Box::new(move |v| (v * recip).round() / recip));
        self
    }
}

impl<V: SliderValue> Widget for SliderWithLabel<V> {
    fn mount(self, scope: &mut Scope<'_>) {
        if self.editable {
            row((self.slider, TextInput::new(self.text_value)))
                .with_cross_align(Alignment::Center)
                .mount(scope)
        } else {
            row((
                self.slider,
                StreamWidget(self.text_value.stream().map(Text::new)),
            ))
            .with_cross_align(Alignment::Center)
            .mount(scope)
        }
    }
}
