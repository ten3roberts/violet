use futures::{stream::BoxStream, FutureExt, StreamExt};
use parking_lot::Mutex;

use super::{State, StateRef, StateSink, StateStream, StateStreamRef, StateWrite};

struct Inner<T> {
    value: T,
    stream: BoxStream<'static, T>,
}

/// Memo is a state that remembers the last value sent.
///
/// This allows converting stream only state into ref states.
pub struct Memo<C, T> {
    value: Mutex<Inner<T>>,
    inner: C,
}

impl<C, T> Memo<C, T> {
    /// Create a new memo state.
    pub fn new(inner: C, initial_value: T) -> Self
    where
        C: StateStream<Item = T>,
    {
        Self {
            value: Mutex::new(Inner {
                value: initial_value,
                stream: inner.stream(),
            }),
            inner,
        }
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.value.lock().value
    }
}

impl<C, T> State for Memo<C, T> {
    type Item = T;
}

impl<C: StateStream, T> StateRef for Memo<C, T> {
    fn read_ref<F: FnOnce(&Self::Item) -> V, V>(&self, f: F) -> V {
        let inner = &mut *self.value.lock();
        if let Some(new_value) = inner.stream.next().now_or_never().flatten() {
            inner.value = new_value;
        }

        f(&inner.value)
    }
}

impl<C: StateSink<Item = T> + StateStream<Item = T>, T: Clone> StateWrite for Memo<C, T> {
    fn write_mut<F: FnOnce(&mut Self::Item) -> V, V>(&self, f: F) -> V {
        let inner = &mut *self.value.lock();

        if let Some(new_value) = inner.stream.next().now_or_never().flatten() {
            inner.value = new_value;
        }

        let w = f(&mut inner.value);
        self.send(inner.value.clone());
        w
    }
}

impl<C: StateStream<Item = T>, T: 'static + Clone> StateStreamRef for Memo<C, T> {
    fn stream_ref<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static + Send>(
        &self,
        mut func: F,
    ) -> impl futures::prelude::Stream<Item = V> + 'static + Send
    where
        Self: Sized,
    {
        self.inner.stream().map(move |v| func(&v)).boxed()
    }
}

impl<C: StateStream<Item = T>, T: Clone> StateStream for Memo<C, T> {
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        self.inner.stream()
    }
}

impl<C: StateSink<Item = T>, T: Clone> StateSink for Memo<C, T> {
    fn send(&self, value: Self::Item) {
        self.inner.send(value);
    }
}
