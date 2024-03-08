use std::{marker::PhantomData, sync::Arc};

use futures::{stream::BoxStream, Stream, StreamExt};
use futures_signals::signal::{Mutable, MutableSignalRef, SignalExt, SignalStream};

/// A trait to project an arbitrary type into a type `U`.
///
/// This can be used to "map" signals to other signals and composing and decomposing larger state
/// into smaller parts for reactivity
pub trait Project<U> {
    fn project<V, F: FnOnce(&U) -> V>(&self, f: F) -> V;
}

pub trait ProjectDyn<U> {
    fn project_copy(&self) -> U
    where
        U: Copy;

    fn project_cloned(&self) -> U
    where
        U: Clone;
}

impl<T, U> ProjectDyn<U> for T
where
    T: Project<U>,
{
    fn project_copy(&self) -> U
    where
        U: Copy,
    {
        self.project(|v| *v)
    }

    fn project_cloned(&self) -> U
    where
        U: Clone,
    {
        self.project(|v| v.clone())
    }
}

pub trait ProjectMut<U>: Project<U> {
    fn project_mut<V, F: FnOnce(&mut U) -> V>(&self, f: F) -> V
    where
        Self: Sized;
}

pub trait ProjectMutDyn<U>: ProjectDyn<U> {
    fn replace(&self, value: U);
}

impl<U, T> ProjectMutDyn<U> for T
where
    T: Sized + ProjectMut<U>,
{
    fn replace(&self, value: U) {
        self.project_mut(|v| *v = value);
    }
}

/// A trait to project an arbitrary type into a stream of type `U`.
pub trait ProjectStream<U>: Project<U> {
    type Stream<F: Send + Sync + FnMut(&U) -> V, V>: Stream<Item = V> + Send + Sync;
    fn project_stream<F: 'static + Send + Sync + FnMut(&U) -> V, V: 'static>(
        &self,
        func: F,
    ) -> Self::Stream<F, V>;
}

pub trait ProjectStreamDyn<U>: ProjectDyn<U> {
    fn project_stream_copy(&self) -> BoxStream<U>
    where
        U: 'static + Copy;

    fn project_stream_clone(&self) -> BoxStream<U>
    where
        U: 'static + Clone;
}

impl<U, T> ProjectStreamDyn<U> for T
where
    T: Send + Sync + ProjectStream<U>,
{
    fn project_stream_copy(&self) -> BoxStream<U>
    where
        U: 'static + Copy,
    {
        Box::pin(self.project_stream(|v: &U| *v))
    }

    fn project_stream_clone(&self) -> BoxStream<U>
    where
        U: 'static + Clone,
    {
        Box::pin(self.project_stream(|v| v.clone()))
    }
}

pub trait ProjectStreamDynMut<U>: ProjectMutDyn<U> + ProjectStreamDyn<U> {}

impl<U, T> ProjectStreamDynMut<U> for T where T: ProjectMutDyn<U> + ProjectStreamDyn<U> {}

impl<T> Project<T> for Mutable<T> {
    fn project<V, F: FnOnce(&T) -> V>(&self, f: F) -> V {
        f(&self.lock_ref())
    }
}

impl<T> ProjectMut<T> for Mutable<T> {
    fn project_mut<V, F: FnOnce(&mut T) -> V>(&self, f: F) -> V {
        f(&mut self.lock_mut())
    }
}

impl<T> ProjectStream<T> for Mutable<T>
where
    T: Send + Sync,
{
    type Stream<F: Send + Sync + FnMut(&T) -> V, V> = SignalStream<MutableSignalRef<T, F>>;

    fn project_stream<F: Send + Sync + FnMut(&T) -> V, V>(&self, func: F) -> Self::Stream<F, V> {
        self.signal_ref(func).to_stream()
    }
}

/// A [`Mutable`](futures_signals::signal::Mutable) that is mapped to project a container of `T` to
/// a `U`.
///
/// In a way, this acts as a duplex Sink, that allowes sending a U, and a stream for receiving a U.
///
/// Please, for your own sanity, don't name this type yourself. It's a mouthful, compose and box it
/// like an iterator or stream.
pub struct Mapped<C, T, U, F, G> {
    inner: C,
    project: Arc<F>,
    project_mut: G,
    _marker: std::marker::PhantomData<(T, U)>,
}

impl<C: Project<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> Mapped<C, T, U, F, G> {
    pub fn new(inner: C, project: F, project_mut: G) -> Self {
        Self {
            inner,
            project: Arc::new(project),
            project_mut,
            _marker: PhantomData,
        }
    }

    pub fn get(&self) -> U
    where
        U: Copy,
    {
        self.project(|v| *v)
    }

    pub fn get_cloned(&self) -> U
    where
        U: Copy,
    {
        self.project(|v| v.clone())
    }
}

impl<C: Project<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> Project<U>
    for Mapped<C, T, U, F, G>
{
    fn project<V, H: FnOnce(&U) -> V>(&self, f: H) -> V {
        self.inner.project(|v| f((self.project)(v)))
    }
}

impl<C: ProjectMut<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> ProjectMut<U>
    for Mapped<C, T, U, F, G>
{
    fn project_mut<V, H: FnOnce(&mut U) -> V>(&self, f: H) -> V {
        self.inner.project_mut(|v| f((self.project_mut)(v)))
    }
}
impl<C: ProjectStream<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> ProjectStream<U>
    for Mapped<C, T, U, F, G>
where
    T: 'static + Send + Sync,
    U: 'static + Copy + Send + Sync,
    F: 'static + Send + Sync + Fn(&T) -> &U,
{
    type Stream<H: Send + Sync + FnMut(&U) -> V, V> =
        C::Stream<Box<dyn Send + Sync + FnMut(&T) -> V>, V>;

    fn project_stream<H: 'static + Send + Sync + FnMut(&U) -> V, V: 'static>(
        &self,
        mut func: H,
    ) -> Self::Stream<H, V> {
        let p = self.project.clone();

        let func =
            Box::new(move |v: &T| -> V { func(p(v)) }) as Box<dyn Send + Sync + FnMut(&T) -> V>;

        self.inner.project_stream(func)
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

impl<T, U> Project<U> for Arc<T>
where
    T: Project<U>,
{
    fn project<V, F: FnOnce(&U) -> V>(&self, f: F) -> V {
        (**self).project(f)
    }
}

impl<T, U> ProjectMut<U> for Arc<T>
where
    T: ProjectMut<U>,
{
    fn project_mut<V, F: FnOnce(&mut U) -> V>(&self, f: F) -> V {
        (**self).project_mut(f)
    }
}

impl<T, U> ProjectStream<U> for Arc<T>
where
    T: ProjectStream<U>,
{
    type Stream<F: Send + Sync + FnMut(&U) -> V, V> = T::Stream<F, V>;

    fn project_stream<F: 'static + Send + Sync + FnMut(&U) -> V, V: 'static>(
        &self,
        func: F,
    ) -> Self::Stream<F, V> {
        (**self).project_stream(func)
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    #[tokio::test]
    async fn mapped_mutable() {
        let state = Mutable::new((1, 2));

        let a = Mapped::new(state.clone(), |v| &v.0, |v| &mut v.0);

        assert_eq!(a.get(), 1);

        let mut stream1 = a.project_stream(|v| *v);
        let mut stream2 = a.project_stream(|v| *v);

        assert_eq!(stream1.next().await, Some(1));
        a.project_mut(|v| *v = 2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }

    #[tokio::test]
    async fn mapped_mutable_dyn() {
        let state = Mutable::new((1, 2));

        let a = Mapped::new(state.clone(), |v| &v.0, |v| &mut v.0);

        let a = Box::new(a) as Box<dyn ProjectStreamDynMut<i32>>;

        assert_eq!(a.project_copy(), 1);

        let mut stream1 = a.project_stream_copy();
        let mut stream2 = a.project_stream_clone();

        assert_eq!(stream1.next().await, Some(1));
        a.replace(2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }
}
