use futures::Stream;
use futures_signals::signal::{Signal, SignalExt};

use crate::{components::layout, layout::Layout, Scope, StreamEffect, Widget};

pub struct SignalWidget<S> {
    signal: S,
}

impl<S> SignalWidget<S> {
    pub fn new(signal: S) -> Self {
        Self { signal }
    }
}

impl<S, W> Widget for SignalWidget<S>
where
    S: 'static + Signal<Item = W>,
    W: Widget,
{
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let mut child = None;
        let stream = self.signal.to_stream();

        scope.set(layout(), Layout::default());

        scope.spawn(StreamEffect::new(
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

pub struct StreamWidget<S> {
    stream: S,
}

impl<S, W> Widget for StreamWidget<S>
where
    S: 'static + Stream<Item = W>,
    W: Widget,
{
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let mut child = None;

        scope.spawn(StreamEffect::new(
            self.stream,
            move |scope: &mut Scope<'_>, v| {
                if let Some(child) = child {
                    scope.detach(child);
                }

                child = Some(scope.attach(v));
            },
        ));
    }
}
