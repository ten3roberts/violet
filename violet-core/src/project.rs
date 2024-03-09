use std::{marker::PhantomData, rc::Rc, sync::Arc};

use futures::{stream::BoxStream, Stream, StreamExt};
use futures_signals::signal::{Mutable, SignalExt};

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
/// A [`Mutable`](futures_signals::signal::Mutable) that is mapped to project a container of `T` to
/// a `U`.
pub struct MappedState<C, T, U, F, G> {
    inner: C,
    project: Arc<F>,
    project_mut: G,
    _marker: std::marker::PhantomData<(T, U)>,
}

impl<C: ProjectRef<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> MappedState<C, T, U, F, G> {
    pub fn new(inner: C, project: F, project_mut: G) -> Self {
        Self {
            inner,
            project: Arc::new(project),
            project_mut,
            _marker: PhantomData,
        }
    }
}

impl<C: ProjectRef<T>, T, U, F, G> MappedState<C, T, U, F, G> {
    pub fn get(&self) -> U
    where
        U: Copy,
        F: Fn(&T) -> &U,
    {
        self.project(|v| *v)
    }

    pub fn get_cloned(&self) -> U
    where
        U: Clone,
        F: Fn(&T) -> &U,
    {
        self.project(|v| v.clone())
    }
}

impl<C: ProjectRef<T>, T, U, F: Fn(&T) -> &U, G> ProjectRef<U> for MappedState<C, T, U, F, G> {
    fn project<V, H: FnOnce(&U) -> V>(&self, f: H) -> V {
        self.inner.project(|v| f((self.project)(v)))
    }
}

impl<C: ProjectRef<T>, T, U: Clone, F: Fn(&T) -> &U, G> ProjectOwned<U>
    for MappedState<C, T, U, F, G>
{
    fn project_owned(&self) -> U {
        self.project(|v| v.clone())
    }
}

impl<C: ProjectMut<T>, T, U, F, G: Fn(&mut T) -> &mut U> ProjectMut<U>
    for MappedState<C, T, U, F, G>
{
    fn project_mut<V, H: FnOnce(&mut U) -> V>(&self, f: H) -> V {
        self.inner.project_mut(|v| f((self.project_mut)(v)))
    }
}

impl<C: ProjectMut<T>, T, U, F, G: Fn(&mut T) -> &mut U> ProjectSink<U>
    for MappedState<C, T, U, F, G>
{
    fn project_send(&self, value: U) {
        self.project_mut(|v| *v = value);
    }
}

impl<C: ProjectStreamRef<T>, T, U, F: Fn(&T) -> &U, G> ProjectStreamRef<U>
    for MappedState<C, T, U, F, G>
where
    T: 'static + Send + Sync,
    U: 'static + Send,
    F: 'static + Send + Sync + Fn(&T) -> &U,
{
    fn project_stream<H: 'static + Send + FnMut(&U) -> V, V: 'static>(
        &self,
        mut func: H,
    ) -> impl Stream<Item = V> + 'static {
        let p = self.project.clone();

        let func = Box::new(move |v: &T| -> V { func(p(v)) }) as Box<dyn Send + FnMut(&T) -> V>;

        self.inner.project_stream(func)
    }
}

impl<C: ProjectStreamRef<T>, T, U, F: Fn(&T) -> &U, G> ProjectStreamOwned<U>
    for MappedState<C, T, U, F, G>
where
    T: 'static + Send + Sync,
    U: 'static + Clone + Send,
    F: 'static + Send + Sync + Fn(&T) -> &U,
{
    fn project_stream_owned(&self) -> BoxStream<'static, U> {
        let f = self.project.clone();
        self.inner.project_stream(move |v| f(v).clone()).boxed()
    }
}

/// A [`Mutable`](futures_signals::signal::Mutable) that is mapped to project a container of `T` to
/// owned instances of `U`.
///
/// In a way, this acts as a duplex Sink, that allowes sending a U, and a stream for receiving a U.
pub struct MappedDuplex<C, T, U, F, G> {
    inner: C,
    into: Arc<F>,
    from: G,
    _marker: std::marker::PhantomData<(T, U)>,
}

impl<C, T, U, F: Fn(&T) -> U, G: Fn(U) -> T> MappedDuplex<C, T, U, F, G> {
    pub fn new(inner: C, into: F, from: G) -> Self {
        Self {
            inner,
            into: Arc::new(into),
            from,
            _marker: PhantomData,
        }
    }
}

impl<C: ProjectRef<T>, T, U: Clone, F: Fn(&T) -> U, G> ProjectRef<U>
    for MappedDuplex<C, T, U, F, G>
{
    fn project<V, H: FnOnce(&U) -> V>(&self, f: H) -> V {
        f(&self.project_owned())
    }
}
impl<C: ProjectRef<T>, T, U: Clone, F: Fn(&T) -> U, G> ProjectOwned<U>
    for MappedDuplex<C, T, U, F, G>
{
    fn project_owned(&self) -> U {
        self.inner.project(|v| (self.into)(v))
    }
}

impl<C: ProjectSink<T>, T, U, F: Fn(&T) -> U, G: Fn(U) -> T> ProjectSink<U>
    for MappedDuplex<C, T, U, F, G>
{
    fn project_send(&self, value: U) {
        self.inner.project_send((self.from)(value));
    }
}

impl<C: ProjectStreamRef<T>, T, U, F: Fn(&T) -> U, G: Fn(U) -> T> ProjectStreamRef<U>
    for MappedDuplex<C, T, U, F, G>
where
    T: 'static + Send + Sync,
    U: 'static + Send,
    F: 'static + Send + Sync + Fn(&T) -> U,
{
    fn project_stream<H: 'static + Send + FnMut(&U) -> V, V: 'static>(
        &self,
        mut func: H,
    ) -> impl 'static + Send + Stream<Item = V> {
        let f = self.into.clone();
        self.inner.project_stream(move |v| func(&f(v))).boxed()
    }
}

impl<C: ProjectStreamRef<T>, T, U, F: Fn(&T) -> U, G: Fn(U) -> T> ProjectStreamOwned<U>
    for MappedDuplex<C, T, U, F, G>
where
    T: 'static + Send + Sync,
    U: 'static + Send,
    F: 'static + Send + Sync + Fn(&T) -> U,
{
    fn project_stream_owned(&self) -> BoxStream<'static, U>
    where
        U: 'static,
    {
        let f = self.into.clone();
        self.inner.project_stream(move |v| f(v)).boxed()
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

        assert_eq!(a.get(), 1);

        let mut stream1 = a.project_stream(|v| *v);
        let mut stream2 = a.project_stream(|v| *v);

        assert_eq!(stream1.next().await, Some(1));
        a.project_mut(|v| *v = 2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }

    #[tokio::test]
    async fn project_duplex() {
        let state = Mutable::new((1, 2));

        let a = MappedState::new(state.clone(), |v| &v.0, |v| &mut v.0);

        let a = Box::new(a) as Box<dyn ProjectDuplex<i32>>;

        let mut stream1 = a.project_stream_owned();
        let mut stream2 = a.project_stream_owned();

        assert_eq!(stream1.next().await, Some(1));
        a.project_send(2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }
}
