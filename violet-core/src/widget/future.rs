use std::f32::consts::TAU;

use futures::{Future, Stream};
use futures_signals::signal::{self, SignalExt};
use glam::vec2;

use crate::{
    components::{on_animation_frame, rotation, transform_origin, translation},
    effect::Effect,
    layout::Align,
    style::{icon_spinner, spacing_medium, text_small, SizeExt},
    tweens::tweens,
    unit::Unit,
    widget::{bold, row, Text},
    FutureEffect, Scope, StreamEffect, Widget,
};

pub struct SignalWidget<S>(pub S);

impl<S> SignalWidget<S> {
    pub fn new(signal: S) -> Self
    where
        S: 'static + signal::Signal,
        <S as signal::Signal>::Item: Widget,
    {
        Self(signal)
    }
}

impl<S> Widget for SignalWidget<S>
where
    S: 'static + signal::Signal,
    S::Item: Widget,
{
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let mut child = None;
        let stream = self.0.to_stream();
        let label = std::any::type_name::<S::Item>();

        scope.spawn_effect(
            StreamEffect::new(stream, move |scope: &mut Scope<'_>, v| {
                if let Some(child) = child {
                    scope.detach(child);
                }

                child = Some(scope.attach(v));
            })
            .with_label(label),
        );
    }
}

pub struct StreamWidget<S>(pub S)
where
    S: Stream,
    S::Item: Widget;

impl<S> StreamWidget<S>
where
    S: 'static + Stream,
    S::Item: Widget,
{
    pub fn new(widget: S) -> Self {
        Self(widget)
    }
}

impl<S> Widget for StreamWidget<S>
where
    S: 'static + Stream,
    S::Item: Widget,
{
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let mut child = None;

        let label = std::any::type_name::<S::Item>();

        scope.spawn_effect(
            StreamEffect::new(self.0, move |scope: &mut Scope<'_>, v| {
                puffin::profile_scope!("StreamWidget::mount", "update child widget");
                if let Some(child) = child {
                    puffin::profile_scope!("detach");
                    scope.detach(child);
                }

                {
                    puffin::profile_scope!("attach");
                    child = Some(scope.attach(v));
                }
            })
            .with_label(label),
        );
    }
}

/// Defer a widget until the future resolves
pub struct FutureWidget<S>(S);

impl<F> FutureWidget<F>
where
    F: 'static + Future,
    F::Output: Widget,
{
    pub fn new(future: F) -> Self {
        Self(future)
    }
}

impl<S> Widget for FutureWidget<S>
where
    S: 'static + Future,
    S::Output: Widget,
{
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let label = std::any::type_name::<S::Output>();
        scope.spawn_effect(
            FutureEffect::new(self.0, move |scope: &mut Scope<'_>, v| {
                scope.attach(v);
            })
            .with_label(label),
        );
    }
}

/// Defer a widget until the future resolves
pub struct SuspenseWidget<T, F> {
    placeholder: T,
    future: F,
}

impl<T, F> SuspenseWidget<T, F>
where
    T: Widget,
    F: 'static + Future,
    F::Output: Widget,
{
    pub fn new(placeholder: T, future: F) -> Self {
        Self {
            placeholder,
            future,
        }
    }
}

impl<T, F> Widget for SuspenseWidget<T, F>
where
    T: Widget,
    F: 'static + Future,
    F::Output: Widget,
{
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let label = std::any::type_name::<F::Output>();
        let placeholder_id = scope.attach(self.placeholder);

        scope.spawn_effect(
            FutureEffect::new(self.future, move |scope: &mut Scope<'_>, v| {
                scope.detach(placeholder_id);
                scope.attach(v);
            })
            .with_label(label),
        );
    }
}

pub struct Throbber {
    pub size: f32,
}

impl Throbber {
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

impl Widget for Throbber {
    fn mount(self, scope: &mut Scope<'_>) {
        scope
            .set(transform_origin(), vec2(0.5, 0.5))
            .set_default(rotation())
            .set_default(translation())
            .set_default(tweens());

        let mut start_rotation = 0.0;
        let mut start_time = None;

        scope.set(
            on_animation_frame(),
            Box::new(move |_, entity, elapsed, _| {
                let start_time = start_time.get_or_insert_with(|| elapsed);

                let time = 1.0;
                let progress = (elapsed - *start_time).as_secs_f32() / time;
                let rotation = &mut *entity.get_mut(rotation()).unwrap();
                if progress > 1.0 {
                    *start_time = elapsed;
                    start_rotation = *rotation;
                }

                let t = progress % 1.0;

                let change = TAU + 2.0;
                *rotation = start_rotation + ease_quad(change, t);
            }),
        );
        let spinner = scope
            .stylesheet()
            .get_clone(icon_spinner())
            .unwrap_or_default();

        Text::new(spinner)
            .with_font_size(self.size)
            .with_exact_size(Unit::px2(self.size, self.size))
            .with_margin(spacing_medium())
            .mount(scope);
    }
}

fn ease_quad(delta: f32, mut t: f32) -> f32 {
    t *= 2.0;

    let scalar = if t < 1.0 {
        t * t
    } else {
        let p = t - 1.0;
        (p * (p - 2.0) - 1.0) * -1.0
    };

    delta * scalar / 2.0
}

pub struct LoadingSpinner {
    text: String,
}

impl LoadingSpinner {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for LoadingSpinner {
    fn mount(self, scope: &mut Scope<'_>) {
        let size = scope.stylesheet().get_copy(text_small()).unwrap();
        row((bold(self.text), Throbber::new(size)))
            .with_cross_align(Align::Center)
            .mount(scope);
    }
}
