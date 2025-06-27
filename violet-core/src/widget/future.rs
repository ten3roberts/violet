use futures::{Future, Stream};
use futures_signals::signal::{self, SignalExt};

use crate::{effect::Effect, FutureEffect, Scope, StreamEffect, Widget};

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
pub struct DeferWidget<T, F> {
    placeholder: T,
    future: F,
}

impl<T, F> DeferWidget<T, F>
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

impl<T, F> Widget for DeferWidget<T, F>
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
