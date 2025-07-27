use std::future::ready;

use futures::StreamExt;

use super::{State, StateSink, StateStream, StateStreamRef};

/// Deduplicates a state updates for receiving streams.
///
/// **NOTE**: Does not deduplicate for sending to sinks as it is not possible to know if the item
/// has been set by another sink or not without readback.
pub struct Dedup<T: State>
where
    T::Item: Sized,
{
    inner: T,
}

impl<T: Clone + State> Clone for Dedup<T>
where
    T::Item: Sized,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: State> Dedup<T>
where
    T::Item: Sized,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: State> State for Dedup<T>
where
    T::Item: Sized,
{
    type Item = T::Item;
}

impl<T> StateStreamRef for Dedup<T>
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
        let mut last_seen = None;

        self.inner
            .stream_ref(move |item| {
                if last_seen.as_ref() != Some(item) {
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
        self.inner
            .stream()
            .filter_map(move |v| {
                if last_seen.as_ref() != Some(&v) {
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
        self.inner.send(item);
    }
}
