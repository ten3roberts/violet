use futures::Stream;
use futures_signals::signal::{self, SignalExt};

use crate::{components::layout, layout::Layout, Scope, StreamEffect, Widget};

pub struct Signal<S>(pub S);

impl<S> Signal<S> {
    pub fn new(signal: S) -> Self
    where
        S: 'static + signal::Signal,
        <S as signal::Signal>::Item: Widget,
    {
        Self(signal)
    }
}

impl<S, W> Widget for Signal<S>
where
    S: 'static + signal::Signal<Item = W>,
    W: Widget,
{
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let mut child = None;
        let stream = self.0.to_stream();

        scope.spawn_effect(StreamEffect::new(
            stream,
            move |scope: &mut Scope<'_>, v| {
                if let Some(child) = child {
                    scope.detach(child);
                }

                child = Some(scope.attach(v));
            },
        ));
    }
}

pub struct StreamWidget<S>(pub S);

impl<S, W> Widget for StreamWidget<S>
where
    S: 'static + Stream<Item = W>,
    W: Widget,
{
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let mut child = None;

        scope.spawn_effect(StreamEffect::new(
            self.0,
            move |scope: &mut Scope<'_>, v| {
                if let Some(child) = child {
                    scope.detach(child);
                }

                child = Some(scope.attach(v));
            },
        ));
    }
}
