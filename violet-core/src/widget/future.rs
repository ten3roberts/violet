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

pub struct FutureWidget<S>(pub S)
where
    S: Future,
    S::Output: Widget;

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
