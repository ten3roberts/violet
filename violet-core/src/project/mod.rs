use std::{marker::PhantomData, rc::Rc, sync::Arc};

use futures::{stream::BoxStream, Stream, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};

mod duplex;
pub use duplex::*;

/// A trait to project an arbitrary type into a type `U`.
///
/// This can be used to "map" signals to other signals and composing and decomposing larger state
/// into smaller parts for reactivity
pub trait ProjectRef<U> {
    fn project<V, F: FnOnce(&U) -> V>(&self, f: F) -> V;
}

pub trait ProjectOwned<U>: ProjectRef<U> {
    fn project_owned(&self) -> U;
}

// impl<T, U> ProjectOwned<U> for T
// where
//     T: ProjectRef<U>,
//     U: Clone,
// {
//     fn project_owned(&self) -> U
//     where
//         U: Clone,
//     {
//         self.project(|v| v.clone())
//     }
// }

/// Ability to project a mutable reference to a type `U`.
pub trait ProjectMut<U> {
    fn project_mut<V, F: FnOnce(&mut U) -> V>(&self, f: F) -> V
    where
        Self: Sized;
}

/// Ability to receive a type of `U`
pub trait ProjectSink<U> {
    fn project_send(&self, value: U);
}

// impl<U, T> ProjectSink<U> for T
// where
//     T: Sized + ProjectMut<U>,
// {
//     fn project_send(&self, value: U) {
//         self.project_mut(|v| *v = value);
//     }
// }

/// A trait to produce a stream projection of a type `U`.
pub trait ProjectStreamRef<U> {
    fn project_stream<F: 'static + Send + FnMut(&U) -> V, V: 'static>(
        &self,
        func: F,
    ) -> impl 'static + Send + Stream<Item = V>
    where
        Self: Sized;
}

pub trait ProjectStreamOwned<U>: ProjectStreamRef<U> {
    fn project_stream_owned(&self) -> BoxStream<'static, U>;
}

/// Supertrait for types that support both sending and receiving a type `U`.
pub trait ProjectDuplex<U>: ProjectSink<U> + ProjectStreamOwned<U> {}

/// Supertrait which support mutable and reference projection and streaming of a type `U`.
///
/// This is the most general trait, and is useful for composing and decomposing state.
pub trait ProjectState<U>: ProjectMut<U> + ProjectStreamRef<U> {}

impl<U, T> ProjectDuplex<U> for T where T: ProjectSink<U> + ProjectStreamOwned<U> {}

impl<T> ProjectRef<T> for Mutable<T> {
    fn project<V, F: FnOnce(&T) -> V>(&self, f: F) -> V {
        f(&self.lock_ref())
    }
}

impl<T: Clone> ProjectOwned<T> for Mutable<T> {
    fn project_owned(&self) -> T {
        self.get_cloned()
    }
}

impl<T> ProjectMut<T> for Mutable<T> {
    fn project_mut<V, F: FnOnce(&mut T) -> V>(&self, f: F) -> V {
        f(&mut self.lock_mut())
    }
}

impl<T> ProjectSink<T> for Mutable<T> {
    fn project_send(&self, value: T) {
        self.set(value);
    }
}

impl<T> ProjectStreamRef<T> for Mutable<T>
where
    T: 'static + Send + Sync,
{
    fn project_stream<F: 'static + Send + FnMut(&T) -> V, V: 'static>(
        &self,
        func: F,
    ) -> impl 'static + Send + Stream<Item = V> {
        self.signal_ref(func).to_stream()
    }
}

impl<T> ProjectStreamOwned<T> for Mutable<T>
where
    T: 'static + Send + Sync + Clone,
{
    fn project_stream_owned(&self) -> BoxStream<'static, T>
    where
        T: 'static + Clone,
    {
        self.signal_cloned().to_stream().boxed()
    }
}

/// A trait to read a reference from a generic state
pub trait StateRef {
    type Item;
    fn read_ref<F: FnOnce(&Self::Item) -> V, V>(&self, f: F) -> V;
}

/// Allows reading an owned value from a state
pub trait StateOwned: StateRef {
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

pub trait State {
    type Item;
}

/// A trait to read a stream from a generic state
pub trait StateStream: State {
    /// Subscribe to a stream of the state
    ///
    /// The passed function is used to transform the value in the stream, to allow for handling
    /// non-static or non-cloneable types.
    fn stream<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static>(
        &self,
        func: F,
    ) -> impl Stream<Item = V> + 'static + Send
    where
        Self: Sized;
}

pub trait StateStreamOwned: StateStream {
    fn stream_owned(&self) -> BoxStream<'static, Self::Item>;
}

/// A trait to send a value to a generic state
pub trait StateSink: State {
    /// Send a value to the state
    fn send(&self, value: Self::Item);
}

/// Allows sending and receiving a value to a state
pub trait StateDuplex: StateStreamOwned + StateSink {}

impl<T> StateDuplex for T where T: StateStreamOwned + StateSink {}

impl<T> State for Mutable<T> {
    type Item = T;
}

impl<T> StateRef for Mutable<T> {
    type Item = T;
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

impl<T> StateStream for Mutable<T>
where
    T: 'static + Send + Sync,
{
    fn stream<F: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static>(
        &self,
        func: F,
    ) -> impl Stream<Item = V> + 'static + Send {
        self.signal_ref(func).to_stream()
    }
}

/// Transforms one state of to another type through reference projection.
///
/// This is used to lower a state `T` of a struct to a state `U` of a field of that struct.
///
/// Can be used both to mutate and read the state, as well as to operate as a duplex sink and
/// stream.
pub struct MappedState<C, U, F, G> {
    inner: C,
    project: Arc<F>,
    project_mut: G,
    _marker: PhantomData<U>,
}

impl<C: StateRef, U, F: Fn(&C::Item) -> &U, G: Fn(&mut C::Item) -> &mut U> MappedState<C, U, F, G> {
    pub fn new(inner: C, project: F, project_mut: G) -> Self {
        Self {
            inner,
            project: Arc::new(project),
            project_mut,
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> State for MappedState<C, U, F, G> {
    type Item = U;
}

impl<C, U, F, G> StateRef for MappedState<C, U, F, G>
where
    C: StateRef,
    F: Fn(&C::Item) -> &U,
{
    type Item = U;
    fn read_ref<H: FnOnce(&Self::Item) -> V, V>(&self, f: H) -> V {
        self.inner.read_ref(|v| f((self.project)(v)))
    }
}

impl<C, U, F, G> StateOwned for MappedState<C, U, F, G>
where
    C: StateRef,
    U: Clone,
    F: Fn(&C::Item) -> &U,
{
    fn read(&self) -> Self::Item {
        self.read_ref(|v| v.clone())
    }
}

impl<C, U, F, G> StateMut for MappedState<C, U, F, G>
where
    C: StateMut,
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    fn write_mut<H: FnOnce(&mut Self::Item) -> V, V>(&self, f: H) -> V {
        self.inner.write_mut(|v| f((self.project_mut)(v)))
    }
}

impl<C, U, F, G> StateStream for MappedState<C, U, F, G>
where
    C: StateStream,
    F: 'static + Fn(&C::Item) -> &U + Sync + Send,
{
    fn stream<I: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static>(
        &self,
        mut func: I,
    ) -> impl Stream<Item = V> + 'static + Send {
        let project = self.project.clone();
        self.inner.stream(move |v| func(project(v)))
    }
}

impl<C, U, F, G> StateStreamOwned for MappedState<C, U, F, G>
where
    C: StateStream,
    U: 'static + Clone,
    F: 'static + Fn(&C::Item) -> &U + Sync + Send,
{
    fn stream_owned(&self) -> BoxStream<'static, Self::Item> {
        self.stream(|v| v.clone()).boxed()
    }
}

/// Bridge update-by-reference to update-by-value
impl<C, U, F, G> StateSink for MappedState<C, U, F, G>
where
    C: StateMut,
    F: Fn(&C::Item) -> &U,
    G: Fn(&mut C::Item) -> &mut U,
{
    fn send(&self, value: Self::Item) {
        self.write_mut(|v| *v = value);
    }
}

/// Transforms one state to another through type conversion
///
///
/// This allows deriving state from another where the derived state is not present in the original.
///
/// However, as this does not assume the derived state is contained withing the original state is
/// does not allow in-place mutation.
pub struct MappedDuplex<C, U, F, G> {
    inner: C,
    project: Arc<F>,
    project_mut: G,
    _marker: PhantomData<U>,
}

impl<C, U, F, G> State for MappedDuplex<C, U, F, G> {
    type Item = U;
}

impl<C: StateRef, U, F: Fn(&C::Item) -> U, G: Fn(&U) -> C::Item> MappedDuplex<C, U, F, G> {
    pub fn new(inner: C, project: F, project_mut: G) -> Self {
        Self {
            inner,
            project: Arc::new(project),
            project_mut,
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> StateRef for MappedDuplex<C, U, F, G>
where
    C: StateRef,
    F: Fn(&C::Item) -> U,
{
    type Item = U;
    fn read_ref<H: FnOnce(&Self::Item) -> V, V>(&self, f: H) -> V {
        f(&self.inner.read_ref(|v| (self.project)(v)))
    }
}

impl<C, U, F, G> StateOwned for MappedDuplex<C, U, F, G>
where
    C: StateRef,
    F: Fn(&C::Item) -> U,
{
    fn read(&self) -> Self::Item {
        self.inner.read_ref(|v| (self.project)(v))
    }
}

impl<C, U, F, G> StateStream for MappedDuplex<C, U, F, G>
where
    C: StateStream,
    F: 'static + Fn(&C::Item) -> U + Sync + Send,
{
    fn stream<I: 'static + Send + Sync + FnMut(&Self::Item) -> V, V: 'static>(
        &self,
        mut func: I,
    ) -> impl Stream<Item = V> + 'static + Send {
        let project = self.project.clone();
        self.inner.stream(move |v| func(&project(v)))
    }
}

impl<C, U, F, G> StateStreamOwned for MappedDuplex<C, U, F, G>
where
    C: StateStream,
    U: 'static + Clone,
    F: 'static + Fn(&C::Item) -> U + Sync + Send,
{
    fn stream_owned(&self) -> BoxStream<'static, Self::Item> {
        self.stream(|v| v.clone()).boxed()
    }
}

/// Bridge update-by-reference to update-by-value
impl<C, U, F, G> StateSink for MappedDuplex<C, U, F, G>
where
    C: StateSink,
    F: Fn(C::Item) -> U,
    G: Fn(U) -> C::Item,
{
    fn send(&self, value: Self::Item) {
        self.inner.send((self.project_mut)(value))
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
        impl<T, U> ProjectRef<U> for $ty<T>
        where
            T: ?Sized + ProjectRef<U>,
        {
            fn project<V, F: FnOnce(&U) -> V>(&self, f: F) -> V {
                (**self).project(f)
            }
        }

        impl<T, U> ProjectMut<U> for $ty<T>
        where
            T: ProjectMut<U>,
        {
            fn project_mut<V, F: FnOnce(&mut U) -> V>(&self, f: F) -> V {
                (**self).project_mut(f)
            }
        }

        impl<T, U> ProjectStreamRef<U> for $ty<T>
        where
            T: ProjectStreamRef<U>,
        {
            fn project_stream<F: 'static + Send + FnMut(&U) -> V, V: 'static>(
                &self,
                func: F,
            ) -> impl Stream<Item = V> + 'static {
                (**self).project_stream(func)
            }
        }

        impl<T, U> ProjectStreamOwned<U> for $ty<T>
        where
            T: ProjectStreamOwned<U>,
        {
            fn project_stream_owned(&self) -> BoxStream<'static, U> {
                (**self).project_stream_owned()
            }
        }

        impl<T, U> ProjectSink<U> for $ty<T>
        where
            T: ?Sized + ProjectSink<U>,
        {
            fn project_send(&self, value: U) {
                (**self).project_send(value);
            }
        }
    };
}

impl_container!(Box);
impl_container!(Arc);
impl_container!(Rc);

impl<T> ProjectStreamRef<T> for flume::Receiver<T>
where
    T: 'static + Send + Sync,
{
    fn project_stream<F: 'static + Send + FnMut(&T) -> V, V: 'static>(
        &self,
        mut func: F,
    ) -> impl 'static + Send + Stream<Item = V> {
        self.clone().into_stream().map(move |v| func(&v))
    }
}

impl<T> ProjectSink<T> for flume::Sender<T> {
    fn project_send(&self, value: T) {
        self.send(value).unwrap();
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn mapped_mutable() {
        let state = Mutable::new((1, 2));

        let a = MappedState::new(state.clone(), |v| &v.0, |v| &mut v.0);

        assert_eq!(a.read(), 1);

        let mut stream1 = a.stream(|v| *v);
        let mut stream2 = a.stream(|v| *v);

        assert_eq!(stream1.next().await, Some(1));
        a.write_mut(|v| *v = 2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }

    #[tokio::test]
    async fn project_duplex() {
        let state = Mutable::new((1, 2));

        let a = MappedState::new(state.clone(), |v| &v.0, |v| &mut v.0);

        let a = Box::new(a) as Box<dyn StateDuplex<Item = i32>>;

        let mut stream1 = a.stream_owned();
        let mut stream2 = a.stream_owned();

        assert_eq!(stream1.next().await, Some(1));
        a.send(2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }
}
