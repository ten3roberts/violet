//! Module for state projection and transformation
//!
//! This simplifies the process of working with signals and mapping state from different types or
//! smaller parts of a larger state.
use std::{marker::PhantomData, rc::Rc, sync::Arc};

use futures::{stream::BoxStream, FutureExt, Stream, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};

pub mod constant;
mod dedup;
mod feedback;
mod filter;
pub mod lower_opt;
mod map;
mod memo;
mod project;
mod transform;

pub use dedup::*;
pub use feedback::*;
pub use filter::*;
use lower_opt::LowerOption;
pub use map::*;
pub use memo::*;
pub use project::*;
use sync_wrapper::SyncWrapper;
pub use transform::*;

pub trait State {
    type Item: ?Sized;
}

pub trait StateExt: State + Sized {
    /// Map a state from one type to another through reference projection
    ///
    /// This an be used to target a specific field of a struct or item in an array to transform.
    fn project_ref<F: Fn(&Self::Item) -> &U, G: Fn(&mut Self::Item) -> &mut U, U: ?Sized>(
        self,
        conv_to: F,
        conv_from: G,
    ) -> Project<Self, U, F, G>
    where
        Self: StateRef,
    {
        Project::new(self, conv_to, conv_from)
    }

    /// Map a state from one type to another
    fn map_value<F: Fn(Self::Item) -> U, G: Fn(U) -> Self::Item, U>(
        self,
        to: F,
        from: G,
    ) -> MapValue<Self, U, F, G>
    where
        Self::Item: Sized,
    {
        MapValue::new(self, to, from)
    }

    /// Transform a state from one to another using get and set operations. This is most similar to
    /// a C# properties for transforming values.
    fn transform<F: Fn(&Self::Item) -> U, G: Fn(&mut Self::Item, U), U>(
        self,
        get: F,
        set: G,
    ) -> Transform<Self, U, F, G>
    where
        Self: StateWrite,
    {
        Transform::new(self, get, set)
    }

    /// Map a state from one type to another through fallible conversion
    fn filter_map<F: Fn(Self::Item) -> Option<U>, G: Fn(U) -> Option<Self::Item>, U>(
        self,
        to: F,
        from: G,
    ) -> FilterMap<Self, U, F, G>
    where
        Self::Item: Sized,
    {
        FilterMap::new(self, to, from)
    }

    /// Lower an option into a stream of values
    fn lower_option(self) -> LowerOption<Self>
    where
        LowerOption<Self>: State,
    {
        LowerOption::new(self)
    }

    /// Deduplicate a stream of values through PartialEq.
    fn dedup(self) -> Dedup<Self>
    where
        Self::Item: PartialEq + Clone,
    {
        Dedup::new(self)
    }

    /// Prevents receiving streams from receiving the value just sent over the state
    fn prevent_feedback(self) -> PreventFeedback<Self>
    where
        Self::Item: PartialEq + Clone,
    {
        PreventFeedback::new(self)
    }

    /// Transforms a stream into a stateful source. The last value can be read and written at any
    /// time.
    fn memo(self, initial_value: Self::Item) -> Memo<Self, Self::Item>
    where
        Self: StateStream,
        Self::Item: Clone,
    {
        Memo::new(self, initial_value)
    }
}

impl<T> StateExt for T where T: State {}

/// A trait to read a reference from a generic state
pub trait StateRef: State {
    fn read_ref<F: FnOnce(&Self::Item) -> V, V>(&self, f: F) -> V
    where
        Self: Sized;
}

/// Allows reading an owned value from a state
pub trait StateOwned: State {
    fn read(&self) -> Self::Item;
}

/// A trait to read a mutable reference from a generic state.
///
/// As opposed to [`StateSink`], this allows in place mutation or partial mutation of the state.
///
/// Used as a building block for sinks to target e.g; specific fields of a struct.
pub trait StateWrite: StateRef {
    fn write_mut<F: FnOnce(&mut Self::Item) -> V, V>(&self, f: F) -> V
    where
        Self: Sized;
}

/// Convert a state to a stream of state changes through reference projection.
///
/// This is only available for some states as is used to lower or transform non-cloneable states into smaller parts.
pub trait StateStreamRef: State {
    /// Subscribe to a stream of the state
    ///
    /// The passed function is used to transform the value in the stream, to allow for handling
    /// non-static or non-cloneable types.
    fn stream_ref<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static + Send>(
        &self,
        func: F,
    ) -> impl Stream<Item = V> + 'static + Send
    where
        Self: Sized;
}

/// Convert a state to a stream of state changes.
pub trait StateStream: State {
    fn stream(&self) -> BoxStream<'static, Self::Item>;
}

/// A trait to send a value to a generic state
pub trait StateSink: State {
    /// Send a value to the state
    fn send(&self, value: Self::Item);

    fn map_sink<F, T>(self, f: F) -> MapSink<Self, F, T>
    where
        F: FnMut(T) -> Self::Item,
        Self: Sized,
    {
        MapSink {
            sink: self,
            func: f,
            _marker: PhantomData,
        }
    }
}

pub struct MapSink<S, F, T> {
    sink: S,
    func: F,
    _marker: PhantomData<T>,
}

impl<S, F, T> State for MapSink<S, F, T>
where
    S: StateSink,
    F: Fn(T) -> S::Item,
{
    type Item = T;
}

impl<S, F, T> StateSink for MapSink<S, F, T>
where
    S: StateSink,
    F: Fn(T) -> S::Item,
    S::Item: Sized,
{
    fn send(&self, value: Self::Item) {
        self.sink.send((self.func)(value))
    }
}

/// Allows sending and receiving a value to a state
///
///
/// This is the most common form of state and is used for both reading state updates, and sending
/// new state.
///
/// Notably, this does not allow to directly read the state, as it may not always be available due
/// to filtered states. Instead, you can subscribe to changes and use [`WatchState`] to hold on to
/// the latest known state.
pub trait StateDuplex: StateStream + StateSink {}
pub trait StateProjected: StateStreamRef + StateWrite {}

pub type DynStateDuplex<T> = Box<dyn Send + Sync + StateDuplex<Item = T>>;

impl<T> StateDuplex for T where T: StateStream + StateSink {}
impl<T> StateProjected for T where T: StateStreamRef + StateWrite {}

impl<T> State for Mutable<T> {
    type Item = T;
}

impl<T> StateRef for Mutable<T> {
    fn read_ref<F: FnOnce(&Self::Item) -> V, V>(&self, f: F) -> V {
        f(&self.lock_ref())
    }
}

impl<T: Clone> StateOwned for Mutable<T> {
    fn read(&self) -> Self::Item {
        self.get_cloned()
    }
}

impl<T> StateWrite for Mutable<T> {
    fn write_mut<F: FnOnce(&mut Self::Item) -> V, V>(&self, f: F) -> V {
        f(&mut self.lock_mut())
    }
}

impl<T> StateStreamRef for Mutable<T>
where
    T: 'static + Send + Sync,
{
    fn stream_ref<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static + Send>(
        &self,
        func: F,
    ) -> impl Stream<Item = V> + 'static + Send {
        self.signal_ref(func).to_stream()
    }
}

impl<T> StateStream for Mutable<T>
where
    T: 'static + Send + Sync + Clone,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        self.signal_cloned().to_stream().boxed()
    }
}

impl<T> StateSink for Mutable<T> {
    fn send(&self, value: Self::Item) {
        self.set(value);
    }
}

pub struct WatchState<S: Stream> {
    stream: SyncWrapper<S>,
    last_item: Option<S::Item>,
}

impl<S: Stream> WatchState<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream: SyncWrapper::new(stream),
            last_item: None,
        }
    }

    pub fn last_item(&self) -> Option<&S::Item> {
        self.last_item.as_ref()
    }

    pub fn get(&mut self) -> Option<&S::Item>
    where
        S: Unpin,
    {
        let new =
            std::iter::from_fn(|| self.stream.get_mut().next().now_or_never().flatten()).last();

        if let Some(new) = new {
            self.last_item = Some(new);
        }

        self.last_item.as_ref()
    }
}

// type MappedMutableStream<S, T, V> =
//     SignalStream<MutableSignalRef<T, Box<dyn Send + Sync + FnMut(&T) -> V>>>;
// pub struct MappedStream<T, V> {
//     inner: MappedMutableStream<T, V>,
// }

// impl<T, V> Stream for MappedStream<T, V> {
//     type Item = V;

//     fn poll_next(
//         mut self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Self::Item>> {
//         self.inner.poll_next_unpin(cx)
//     }
// }

macro_rules! impl_container {
    ($ty: ident) => {
        impl<T> State for $ty<T>
        where
            T: ?Sized + State,
        {
            type Item = T::Item;
        }

        impl<T> StateRef for $ty<T>
        where
            T: StateRef,
        {
            fn read_ref<F: FnOnce(&Self::Item) -> V, V>(&self, f: F) -> V {
                (**self).read_ref(f)
            }
        }

        impl<T> StateOwned for $ty<T>
        where
            T: ?Sized + StateOwned,
            Self::Item: Sized,
        {
            fn read(&self) -> Self::Item {
                (**self).read()
            }
        }

        impl<T> StateWrite for $ty<T>
        where
            T: StateWrite,
        {
            fn write_mut<F: FnOnce(&mut Self::Item) -> V, V>(&self, f: F) -> V {
                (**self).write_mut(f)
            }
        }

        impl<T> StateStreamRef for $ty<T>
        where
            T: StateStreamRef,
        {
            fn stream_ref<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static + Send>(
                &self,
                func: F,
            ) -> impl Stream<Item = V> + 'static + Send {
                (**self).stream_ref(func)
            }
        }

        impl<T> StateStream for $ty<T>
        where
            T: ?Sized + StateStream,
        {
            fn stream(&self) -> BoxStream<'static, Self::Item> {
                (**self).stream()
            }
        }

        impl<T> StateSink for $ty<T>
        where
            T: ?Sized + StateSink,
            Self::Item: Sized,
        {
            fn send(&self, value: Self::Item) {
                (**self).send(value)
            }
        }
    };
}

impl_container!(Box);
impl_container!(Arc);
impl_container!(Rc);

// impl<T> State for Arc<dyn StateSink<Item = T>> {
//     type Item = T;

//     fn filter_map<F: Fn(Self::Item) -> Option<U>, G: Fn(U) -> Option<Self::Item>, U>(
//         self,
//         to: F,
//         from: G,
//     ) -> FilterMap<Self, U, F, G> {
//         todo!()
//     }
// }

// impl<T> StateSink for Arc<dyn StateSink<Item = T>> {
//     fn send(&self, value: Self::Item) {
//         todo!()
//     }
// }

impl<T> State for flume::Receiver<T> {
    type Item = T;
}

impl<T> State for flume::Sender<T> {
    type Item = T;
}

impl<T> StateStreamRef for flume::Receiver<T>
where
    T: 'static + Send + Sync,
{
    fn stream_ref<F: 'static + Send + FnMut(&T) -> V, V: 'static + Send>(
        &self,
        mut func: F,
    ) -> impl 'static + Send + Stream<Item = V> {
        self.clone().into_stream().map(move |v| func(&v))
    }
}

impl<T> StateStream for flume::Receiver<T>
where
    T: 'static + Send + Sync,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        self.clone().into_stream().boxed()
    }
}

impl<T> StateSink for flume::Sender<T> {
    fn send(&self, value: T) {
        self.send(value).unwrap();
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn mapped_mutable() {
        let state = Mutable::new((1, 2));

        let a = Project::new(state.clone(), |v| &v.0, |v| &mut v.0);

        assert_eq!(a.read(), 1);

        let mut stream1 = a.stream_ref(|v| *v);
        let mut stream2 = a.stream_ref(|v| *v);

        assert_eq!(stream1.next().await, Some(1));
        a.write_mut(|v| *v = 2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }

    #[tokio::test]
    async fn project_duplex() {
        let state = Mutable::new((1, 2));

        let a = Project::new(state.clone(), |v| &v.0, |v| &mut v.0);

        let a = Box::new(a) as Box<dyn StateDuplex<Item = i32>>;

        let mut stream1 = a.stream();
        let mut stream2 = a.stream();

        assert_eq!(stream1.next().await, Some(1));
        a.send(2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }
}
