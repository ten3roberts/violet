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
mod map;
mod memo;

pub use dedup::*;
pub use feedback::*;
pub use filter::*;
pub use map::*;
pub use memo::*;

use sync_wrapper::SyncWrapper;

pub trait State {
    type Item;

    /// Map a state from one type to another through reference projection
    ///
    /// This an be used to target a specific field of a struct or item in an array to transform.
    fn map_ref<F: Fn(&Self::Item) -> &U, G: Fn(&mut Self::Item) -> &mut U, U>(
        self,
        conv_to: F,
        conv_from: G,
    ) -> MapRef<Self, U, F, G>
    where
        Self: StateRef,
        Self: Sized,
    {
        MapRef::new(self, conv_to, conv_from)
    }

    /// Map a state from one type to another
    fn map<F: Fn(Self::Item) -> U, G: Fn(U) -> Self::Item, U>(
        self,
        conv_to: F,
        conv_from: G,
    ) -> Map<Self, U, F, G>
    where
        Self: Sized,
    {
        Map::new(self, conv_to, conv_from)
    }

    /// Map a state from one type to another through fallible conversion
    fn filter_map<F: Fn(Self::Item) -> Option<U>, G: Fn(U) -> Option<Self::Item>, U>(
        self,
        conv_to: F,
        conv_from: G,
    ) -> FilterMap<Self, U, F, G>
    where
        Self: Sized,
    {
        FilterMap::new(self, conv_to, conv_from)
    }

    fn dedup(self) -> Dedup<Self>
    where
        Self: Sized,
        Self::Item: PartialEq + Clone,
    {
        Dedup::new(self)
    }

    fn prevent_feedback(self) -> PreventFeedback<Self>
    where
        Self: Sized,
        Self::Item: PartialEq + Clone,
    {
        PreventFeedback::new(self)
    }

    fn memo(self, initial_value: Self::Item) -> Memo<Self, Self::Item>
    where
        Self: Sized,
        Self: StateStream,
        Self::Item: Clone,
    {
        Memo::new(self, initial_value)
    }
}

/// A trait to read a reference from a generic state
pub trait StateRef: State {
    fn read_ref<F: FnOnce(&Self::Item) -> V, V>(&self, f: F) -> V;
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
pub trait StateMut: StateRef {
    fn write_mut<F: FnOnce(&mut Self::Item) -> V, V>(&self, f: F) -> V;
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

pub type DynStateDuplex<T> = Box<dyn Send + Sync + StateDuplex<Item = T>>;

impl<T> StateDuplex for T where T: StateStream + StateSink {}

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

impl<T> StateMut for Mutable<T> {
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

/// Transforms one state of to another type through reference projection.
///
/// This is used to lower a state `T` of a struct to a state `U` of a field of that struct.
///
/// Can be used both to mutate and read the state, as well as to operate as a duplex sink and
/// stream.
pub struct MapRef<C, U, F, G> {
    inner: C,
    project: Arc<F>,
    project_mut: G,
    _marker: PhantomData<U>,
}

impl<C: State, U, F: Fn(&C::Item) -> &U, G: Fn(&mut C::Item) -> &mut U> MapRef<C, U, F, G> {
    pub fn new(inner: C, project: F, project_mut: G) -> Self {
        Self {
            inner,
            project: Arc::new(project),
            project_mut,
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> State for MapRef<C, U, F, G> {
    type Item = U;
}

impl<C, U, F, G> StateRef for MapRef<C, U, F, G>
where
    C: StateRef,
    F: Fn(&C::Item) -> &U,
{
    fn read_ref<H: FnOnce(&Self::Item) -> V, V>(&self, f: H) -> V {
        self.inner.read_ref(|v| f((self.project)(v)))
    }
}

impl<C, U, F, G> StateOwned for MapRef<C, U, F, G>
where
    C: StateRef,
    U: Clone,
    F: Fn(&C::Item) -> &U,
{
    fn read(&self) -> Self::Item {
        self.read_ref(|v| v.clone())
    }
}

impl<C, U, F, G> StateMut for MapRef<C, U, F, G>
where
    C: StateMut,
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    fn write_mut<H: FnOnce(&mut Self::Item) -> V, V>(&self, f: H) -> V {
        self.inner.write_mut(|v| f((self.project_mut)(v)))
    }
}

impl<C, U, F, G> StateStreamRef for MapRef<C, U, F, G>
where
    C: StateStreamRef,
    F: 'static + Fn(&C::Item) -> &U + Sync + Send,
{
    fn stream_ref<I: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static + Send>(
        &self,
        mut func: I,
    ) -> impl Stream<Item = V> + 'static + Send {
        let project = self.project.clone();
        self.inner.stream_ref(move |v| func(project(v)))
    }
}

impl<C, U, F, G> StateStream for MapRef<C, U, F, G>
where
    C: StateStreamRef,
    U: 'static + Send + Sync + Clone,
    F: 'static + Fn(&C::Item) -> &U + Sync + Send,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        self.stream_ref(|v| v.clone()).boxed()
    }
}

/// Bridge update-by-reference to update-by-value
impl<C, U, F, G> StateSink for MapRef<C, U, F, G>
where
    C: StateMut,
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    fn send(&self, value: Self::Item) {
        self.write_mut(|v| *v = value);
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
        {
            fn read(&self) -> Self::Item {
                (**self).read()
            }
        }

        impl<T> StateMut for $ty<T>
        where
            T: StateMut,
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

        let a = MapRef::new(state.clone(), |v| &v.0, |v| &mut v.0);

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

        let a = MapRef::new(state.clone(), |v| &v.0, |v| &mut v.0);

        let a = Box::new(a) as Box<dyn StateDuplex<Item = i32>>;

        let mut stream1 = a.stream();
        let mut stream2 = a.stream();

        assert_eq!(stream1.next().await, Some(1));
        a.send(2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }
}
