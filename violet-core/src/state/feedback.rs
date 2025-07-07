use std::future::ready;

use futures::StreamExt;
use futures_signals::signal::Mutable;

use super::{State, StateSink, StateStream, StateStreamRef};

/// Prevents feedback loops by dropping items in the receiving stream that were sent to the sink.
pub struct PreventFeedback<T: State>
where
    <T as State>::Item: Sized,
{
    last_sent: Mutable<Option<T::Item>>,
    inner: T,
}

impl<T: State> PreventFeedback<T>
where
    T::Item: Sized,
{
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            last_sent: Default::default(),
        }
    }
}

impl<T: State> State for PreventFeedback<T>
where
    T::Item: Sized,
{
    type Item = T::Item;
}

impl<T> StateStreamRef for PreventFeedback<T>
where
    T: StateStreamRef,
    T::Item: 'static + Send + Sync + Clone + PartialEq,
{
    fn stream_ref<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static + Send>(
        &self,
        mut func: F,
    ) -> impl futures::prelude::Stream<Item = V> + 'static + Send
    where
        Self: Sized,
    {
        let last_sent = self.last_sent.clone();

        self.inner
            .stream_ref(move |item| {
                let last_sent = last_sent.lock_ref();
                if last_sent.as_ref() != Some(item) {
                    Some(func(item))
                } else {
                    None
                }
            })
            .filter_map(ready)
    }
}

impl<T> StateStream for PreventFeedback<T>
where
    T: StateStream,
    T::Item: 'static + Send + Sync + PartialEq + Clone,
{
    fn stream(&self) -> futures::prelude::stream::BoxStream<'static, Self::Item> {
        let last_sent = self.last_sent.clone();
        self.inner
            .stream()
            .filter_map(move |v| {
                let last_sent = last_sent.lock_ref();

                if last_sent.as_ref() != Some(&v) {
                    ready(Some(v))
                } else {
                    ready(None)
                }
            })
            .boxed()
    }
}

impl<T> StateSink for PreventFeedback<T>
where
    T: StateSink,
    T::Item: 'static + Send + Sync + PartialEq + Clone,
{
    fn send(&self, item: Self::Item) {
        if self.last_sent.lock_ref().as_ref() == Some(&item) {
            // If the item is the same as the last sent, we drop it to prevent feedback.
            return;
        }

        self.last_sent.set(Some(item.clone()));
        self.inner.send(item);
    }
}
