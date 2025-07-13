use std::{pin::Pin, task::Poll};

use futures::{prelude::Stream, stream::BoxStream, FutureExt, StreamExt};
use futures_signals::signal::{Mutable, MutableSignalCloned, SignalExt};
use parking_lot::Mutex;
use pin_project::pin_project;

use super::{State, StateRef, StateSink, StateStream, StateStreamRef, StateWrite};

struct Inner<T> {
    seen_value: bool,
    stream: BoxStream<'static, T>,
}

// impl<T> Inner<T> {
//     // fn last_value<U>(&mut self, f: impl FnOnce(&T) -> U) -> U {
//     //     if let Some(new_value) = self.stream.next().now_or_never().flatten() {
//     //         self.seen_value = true;
//     //         self.last_value = new_value;
//     //     }

//     //     f(&self.last_value)
//     // }

//     // fn last_value_mut<U>(&mut self, f: impl FnOnce(&mut T) -> U) -> U {
//     //     if let Some(new_value) = self.stream.next().now_or_never().flatten() {
//     //         self.seen_value = true;
//     //         self.last_value = new_value;
//     //     }

//     //     f(&mut self.last_value)
//     // }
// }

/// Memo is a state that remembers the last value sent.
///
/// This allows converting stream only state into ref states.
pub struct Memo<C, T> {
    incoming: Mutex<Inner<T>>,
    state: Mutable<T>,
    source: C,
}

fn last_value<T, S: Unpin + Stream<Item = T>>(s: &mut S) -> Option<T> {
    let mut last = None;

    while let Some(value) = s.next().now_or_never().flatten() {
        last = Some(value);
    }

    last
}

impl<C, T> Memo<C, T> {
    /// Create a new memo state.
    pub fn new(source: C, initial_value: T) -> Self
    where
        C: StateStream<Item = T>,
    {
        Self {
            incoming: Mutex::new(Inner {
                seen_value: false,
                stream: source.stream(),
            }),
            state: Mutable::new(initial_value),
            source,
        }
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.state.get()
    }

    fn read_last_value<F: FnOnce(&T) -> V, V>(&self, f: F) -> V
    where
        C: StateStream<Item = T>,
    {
        if let Some(value) = last_value(&mut self.incoming.lock().stream) {
            // eprintln!("Memo::read_last_value: found last value");
            let w = f(&value);
            self.state.set(value);
            w
        } else {
            self.state.read_ref(f)
        }
    }

    fn write_last_value<F: FnOnce(&mut T) -> V, V>(&self, f: F) -> V
    where
        C: StateStream<Item = T> + StateSink,
        T: Clone,
    {
        let mut incoming = self.incoming.lock();
        if let Some(mut value) = last_value(&mut incoming.stream) {
            let w = f(&mut value);
            self.state.set(value.clone());
            self.source.send(value);
            w
        } else {
            incoming.seen_value = true;
            let mut current = &mut self.state.lock_mut();
            let res = f(&mut current);
            self.source.send(current.clone());
            res
        }
    }

    /// Sends the initial value to the underlying state of no value is yielded yet
    pub fn sync_initial(&self)
    where
        C: StateSink<Item = T>,
        T: Clone,
    {
        let mut incoming = self.incoming.lock();
        if !incoming.seen_value {
            if let Some(value) = last_value(&mut incoming.stream) {
                self.state.set(value);
                incoming.seen_value = true;
            } else {
                incoming.seen_value = true;
                self.source.send(self.state.get_cloned());
            }
        }
    }
}

impl<C, T> State for Memo<C, T> {
    type Item = T;
}

impl<C: StateStream<Item = T>, T> StateRef for Memo<C, T> {
    fn read_ref<F: FnOnce(&Self::Item) -> V, V>(&self, f: F) -> V {
        self.read_last_value(f)
    }
}

impl<C: StateSink<Item = T> + StateStream<Item = T>, T: Clone> StateWrite for Memo<C, T> {
    fn write_mut<F: FnOnce(&mut Self::Item) -> V, V>(&self, f: F) -> V {
        self.write_last_value(f)
    }
}

impl<C: StateStream<Item = T>, T: 'static + Clone + Send + Sync> StateStreamRef for Memo<C, T> {
    fn stream_ref<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static + Send>(
        &self,
        mut func: F,
    ) -> impl Stream<Item = V> + 'static + Send
    where
        Self: Sized,
    {
        MemoStream {
            source: self.source.stream().boxed(),
            state: self.state.signal_cloned(),
        }
        .map(move |v| func(&v))
        .boxed()
    }
}

impl<C: StateStream<Item = T>, T: 'static + Send + Sync + Clone> StateStream for Memo<C, T> {
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        Box::pin(MemoStream {
            source: self.source.stream(),
            state: self.state.signal_cloned(),
        })
    }
}

impl<C: StateSink<Item = T>, T: Clone> StateSink for Memo<C, T> {
    fn send(&self, new_value: Self::Item) {
        self.state.set(new_value.clone());
        self.source.send(new_value);
    }
}

#[pin_project]
struct MemoStream<T> {
    source: BoxStream<'static, T>,
    state: MutableSignalCloned<T>,
}

impl<T: Clone + Send> Stream for MemoStream<T> {
    type Item = T;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.source.poll_next_unpin(cx) {
            Poll::Ready(v) => {
                let _ = this.state.poll_change_unpin(cx);
                Poll::Ready(v)
            }
            // No value yet from source
            // Poll::Pending => this.state.poll_change_unpin(cx),
            Poll::Pending => this.state.poll_change_unpin(cx),
        }
    }
}
