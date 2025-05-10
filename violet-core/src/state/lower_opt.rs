use std::{future::ready, marker::Send};

use futures::{stream::BoxStream, StreamExt};

use super::{State, StateSink, StateStream};

/// Lowers an option into a stream of values.
///
/// Allows transforming a `State<Item = Option<T>>` into a `State<Item = T>`.
///
/// Streams will filter until a value is available, sending will wrap the value in `Some`.
pub struct LowerOption<S> {
    inner: S,
}

impl<S> LowerOption<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S: State<Item = Option<T>>, T> State for LowerOption<S> {
    type Item = T;
}

impl<S, T> StateStream for LowerOption<S>
where
    S: StateStream<Item = Option<T>>,
    T: 'static + Send,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        self.inner.stream().filter_map(ready).boxed()
    }
}

/// Bridge update-by-reference to update-by-value
impl<S, T> StateSink for LowerOption<S>
where
    S: StateSink<Item = Option<T>>,
{
    fn send(&self, value: Self::Item) {
        self.inner.send(Some(value));
    }
}
