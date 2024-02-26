use cosmic_text::Wrap;
use flax::{Component, Entity, EntityRef};
use futures_signals::{
    map_ref,
    signal::{Mutable, MutableSignalRef, SignalExt},
};
use glam::{IVec2, Vec2};
use palette::Srgba;
use winit::event::ElementState;

use crate::{
    components::{offset, rect},
    input::{focusable, on_cursor_move, on_mouse_input, CursorMove},
    layout::Alignment,
    style::{get_stylesheet, interactive_active, interactive_inactive, spacing, StyleExt},
    text::TextSegment,
    unit::Unit,
    widget::{BoxSized, ContainerStyle, List, Positioned, Rectangle, Signal, Stack, Text},
    Edges, Scope, StreamEffect, Widget,
};

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
            handle_size: Unit::px2i(1, 5),
        }
    }
}

pub struct Slider<V, T = V> {
    style: SliderStyle,
    value: Mutable<T>,
    min: V,
    max: V,
    lower: fn(&T) -> V,
    update: fn(&mut T, V),
}

impl<V> Slider<V> {
    pub fn new(value: Mutable<V>, min: V, max: V) -> Self
    where
        V: Copy,
    {
        Self {
            value,
            min,
            max,
            style: Default::default(),
            lower: |v| *v,
            update: |dst, src| {
                *dst = src;
            },
        }
    }
}

impl<V, T> Slider<V, T> {
    pub fn new_with_transform(
        value: Mutable<T>,
        min: V,
        max: V,
        lower: fn(&T) -> V,
        update: fn(&mut T, V),
    ) -> Self {
        Self {
            value,
            min,
            max,
            style: Default::default(),
            lower,
            update,
        }
    }

    /// Set the style
    pub fn with_style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
    }
}

impl<V: SliderValue, T: 'static + Send + Sync> Widget for Slider<V, T> {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = get_stylesheet(scope);

        let track_color = stylesheet
            .get_copy(self.style.track_color)
            .unwrap_or_default();
        let handle_color = stylesheet
            .get_copy(self.style.handle_color)
            .unwrap_or_default();

        let spacing = stylesheet.get_copy(spacing()).unwrap_or_default();

        let handle_size = spacing.resolve_unit(self.style.handle_size);
        let track_size = spacing.resolve_unit(self.style.track_size);

        let track = scope.attach(BoxSized::new(Rectangle::new(track_color)).with_size(track_size));

        let min = self.min.to_progress();
        let max = self.max.to_progress();

        fn update<V: SliderValue, T>(
            entity: &EntityRef,
            input: CursorMove,
            min: f32,
            max: f32,
            dst: &Mutable<T>,
            update: fn(&mut T, V),
        ) {
            let rect = entity.get_copy(rect()).unwrap();
            let value = (input.local_pos.x / rect.size().x).clamp(0.0, 1.0) * (max - min) + min;
            update(&mut dst.lock_mut(), V::from_progress(value));
        }

        let handle = SliderHandle {
            value: self.value.signal_ref(self.lower),
            min,
            max,
            rect_id: track,
            handle_color,
            handle_size,
        };

        scope
            .set(focusable(), ())
            .on_event(on_mouse_input(), {
                let value = self.value.clone();
                move |_, entity, input| {
                    if input.state == ElementState::Pressed {
                        update(entity, input.cursor, min, max, &value, self.update);
                    }
                }
            })
            .on_event(on_cursor_move(), move |_, entity, input| {
                update(entity, input, min, max, &self.value, self.update)
            });

        Stack::new(handle)
            .with_vertical_alignment(Alignment::Center)
            .with_style(ContainerStyle {
                margin: Edges::even(10.0),
                ..Default::default()
            })
            .mount(scope)
    }
}

struct SliderHandle<V, T> {
    value: MutableSignalRef<T, fn(&T) -> V>,
    handle_color: Srgba,
    handle_size: Unit<Vec2>,
    min: f32,
    max: f32,
    rect_id: Entity,
}

impl<V: SliderValue, T: 'static + Send + Sync> Widget for SliderHandle<V, T> {
    fn mount(self, scope: &mut Scope<'_>) {
        let rect_size = Mutable::new(None);

        let update_signal = map_ref! {
            let value = self.value,
            let size = rect_size.signal() =>
                (value.to_progress(), *size)
        };

        scope.frame_mut().monitor(self.rect_id, rect(), move |v| {
            rect_size.set(v.map(|v| v.size()));
        });

        scope.spawn_effect(StreamEffect::new(update_signal.to_stream(), {
            move |scope: &mut Scope<'_>, (value, size): (f32, Option<Vec2>)| {
                if let Some(size) = size {
                    let pos = (value - self.min) * size.x / (self.max - self.min);

                    scope.entity().update_dedup(offset(), Unit::px2(pos, 0.0));
                }
            }
        }));

        Positioned::new(
            BoxSized::new(Rectangle::new(self.handle_color)).with_size(self.handle_size),
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

pub struct SliderWithLabel<V, T = V> {
    slider: Slider<V, T>,
}

impl<V> SliderWithLabel<V> {
    pub fn new(value: Mutable<V>, min: V, max: V) -> Self
    where
        V: Copy,
    {
        Self {
            slider: Slider::new(value, min, max),
        }
    }
}
impl<V, T> SliderWithLabel<V, T> {
    pub fn new_with_transform(
        value: Mutable<T>,
        min: V,
        max: V,
        lower: fn(&T) -> V,
        update: fn(&mut T, V),
    ) -> Self {
        Self {
            slider: Slider::new_with_transform(value, min, max, lower, update),
        }
    }

    /// Set the style
    pub fn with_style(mut self, style: SliderStyle) -> Self {
        self.slider = self.slider.with_style(style);
        self
    }
}

impl<V: SliderValue, T: 'static + Send + Sync> Widget for SliderWithLabel<V, T> {
    fn mount(self, scope: &mut Scope<'_>) {
        let lower = self.slider.lower;
        let label =
            Signal(self.slider.value.signal_ref(lower).map(|v| {
                Text::rich([TextSegment::new(format!("{:>4.2}", v))]).with_wrap(Wrap::None)
            }));

        List::new((self.slider, label)).mount(scope)
    }
}
