use std::{future::ready, sync::Arc};

use futures::{FutureExt, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};
use parking_lot::Mutex;
use tracing::info;

use super::{State, StateSink, StateStream, StateStreamRef};

/// Deduplicates a state updates for both sending and receiving halves.
///
/// This means that if the same item is sent to the sink multiple times in a row, it will only be
/// sent once.
///
/// Likewise, the stream will be filtered for duplicate items, to catch duplicates from external
/// sinks (as items can arrive from other sinks than the one that is being deduplicated).
pub struct Dedup<T: State> {
    last_sent: Mutable<Option<T::Item>>,
    inner: T,
}

impl<T: State> Dedup<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            last_sent: Default::default(),
        }
    }
}

impl<T: State> State for Dedup<T> {
    type Item = T::Item;
}

impl<T> StateStreamRef for Dedup<T>
where
    T: StateStreamRef,
    T::Item: 'static + Send + Sync + Clone + PartialEq,
{
    fn stream_ref<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static + Send + Sync>(
        &self,
        mut func: F,
    ) -> impl futures::prelude::Stream<Item = V> + 'static + Send
    where
        Self: Sized,
    {
        let mut last_seen = None;
        let mut last_sent = self.last_sent.signal_cloned().to_stream();

        self.inner
            .stream_ref(move |item| {
                let last_sent = last_sent.next().now_or_never().flatten().flatten();

                if last_seen.as_ref() != Some(item) && last_sent.as_ref() != Some(item) {
                    last_seen = Some(item.clone());
                    Some(func(item))
                } else {
                    None
                }
            })
            .filter_map(ready)
    }
}

impl<T> StateStream for Dedup<T>
where
    T: StateStream,
    T::Item: 'static + Send + Sync + PartialEq + Clone,
{
    fn stream(&self) -> futures::prelude::stream::BoxStream<'static, Self::Item> {
        let mut last_seen = None;
        let mut last_sent = self.last_sent.signal_cloned().to_stream();
        self.inner
            .stream()
            .filter_map(move |v| {
                let last_sent = last_sent.next().now_or_never().flatten().flatten();
                if last_seen.as_ref() != Some(&v) && last_sent.as_ref() != Some(&v) {
                    last_seen = Some(v.clone());
                    ready(Some(v))
                } else {
                    ready(None)
                }
            })
            .boxed()
    }
}

impl<T> StateSink for Dedup<T>
where
    T: StateSink,
    T::Item: 'static + Send + Sync + PartialEq + Clone,
{
    fn send(&self, item: Self::Item) {
        self.last_sent.set(Some(item.clone()));
        self.inner.send(item);
    }
}
