use std::{marker::PhantomData, rc::Rc, sync::Arc};

use futures::{stream::BoxStream, Stream, StreamExt};
use futures_signals::signal::{Mutable, MutableSignalRef, SignalExt, SignalStream};

/// A trait to project an arbitrary type into a type `U`.
///
/// This can be used to "map" signals to other signals and composing and decomposing larger state
/// into smaller parts for reactivity
pub trait ProjectRef<U> {
    fn project<V, F: FnOnce(&U) -> V>(&self, f: F) -> V;
}

pub trait ProjectOwned<U> {
    fn project_copy(&self) -> U
    where
        U: Copy;

    fn project_cloned(&self) -> U
    where
        U: Clone;
}

impl<T, U> ProjectOwned<U> for T
where
    T: ProjectRef<U>,
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

/// Ability to project a mutable reference to a type `U`.
pub trait ProjectMut<U>: ProjectRef<U> {
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
pub trait ProjectStream<U> {
    fn project_stream<F: 'static + Send + FnMut(&U) -> V, V: 'static>(
        &self,
        func: F,
    ) -> impl 'static + Send + Stream<Item = V>;
}

pub trait ProjectStreamOwned<U> {
    fn project_stream_copy(&self) -> BoxStream<'static, U>
    where
        U: 'static + Copy;

    fn project_stream_clone(&self) -> BoxStream<'static, U>
    where
        U: 'static + Clone;
}

impl<U, T> ProjectStreamOwned<U> for T
where
    T: Send + ProjectStream<U>,
{
    fn project_stream_copy(&self) -> BoxStream<'static, U>
    where
        U: 'static + Copy,
    {
        Box::pin(self.project_stream(|v: &U| *v))
    }

    fn project_stream_clone(&self) -> BoxStream<'static, U>
    where
        U: 'static + Clone,
    {
        Box::pin(self.project_stream(|v| v.clone()))
    }
}

/// Supertrait for types that support both sending and receiving a type `U`.
pub trait ProjectDuplex<U>: ProjectSink<U> + ProjectStreamOwned<U> {}

/// Supertrait which support mutable and reference projection and streaming of a type `U`.
///
/// This is the most general trait, and is useful for composing and decomposing state.
pub trait ProjectState<U>: ProjectMut<U> + ProjectStream<U> {}

impl<U, T> ProjectDuplex<U> for T where T: ProjectSink<U> + ProjectStreamOwned<U> {}

impl<T> ProjectRef<T> for Mutable<T> {
    fn project<V, F: FnOnce(&T) -> V>(&self, f: F) -> V {
        f(&self.lock_ref())
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

impl<T> ProjectStream<T> for Mutable<T>
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

/// A [`Mutable`](futures_signals::signal::Mutable) that is mapped to project a container of `T` to
/// a `U`.
///
/// In a way, this acts as a duplex Sink, that allowes sending a U, and a stream for receiving a U.
///
/// Please, for your own sanity, don't name this type yourself. It's a mouthful, compose and box it
/// like an iterator or stream.
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

    pub fn get(&self) -> U
    where
        U: Copy,
    {
        self.project(|v| *v)
    }

    pub fn get_cloned(&self) -> U
    where
        U: Clone,
    {
        self.project(|v| v.clone())
    }
}

impl<C: ProjectRef<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> ProjectRef<U>
    for MappedState<C, T, U, F, G>
{
    fn project<V, H: FnOnce(&U) -> V>(&self, f: H) -> V {
        self.inner.project(|v| f((self.project)(v)))
    }
}

impl<C: ProjectMut<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> ProjectMut<U>
    for MappedState<C, T, U, F, G>
{
    fn project_mut<V, H: FnOnce(&mut U) -> V>(&self, f: H) -> V {
        self.inner.project_mut(|v| f((self.project_mut)(v)))
    }
}

impl<C: ProjectMut<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> ProjectSink<U>
    for MappedState<C, T, U, F, G>
{
    fn project_send(&self, value: U) {
        self.project_mut(|v| *v = value);
    }
}

impl<C: ProjectStream<T>, T, U, F: Fn(&T) -> &U, G: Fn(&mut T) -> &mut U> ProjectStream<U>
    for MappedState<C, T, U, F, G>
where
    T: 'static + Send + Sync,
    U: 'static + Copy + Send,
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
            T: ProjectRef<U>,
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

        impl<T, U> ProjectStream<U> for $ty<T>
        where
            T: ProjectStream<U>,
        {
            fn project_stream<F: 'static + Send + FnMut(&U) -> V, V: 'static>(
                &self,
                func: F,
            ) -> impl Stream<Item = V> + 'static {
                (**self).project_stream(func)
            }
        }
    };
}

impl_container!(Box);
impl_container!(Arc);
impl_container!(Rc);

impl<T> ProjectStream<T> for flume::Receiver<T>
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
    use futures::StreamExt;

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

        let mut stream1 = a.project_stream_copy();
        let mut stream2 = a.project_stream_clone();

        assert_eq!(stream1.next().await, Some(1));
        a.project_send(2);
        assert_eq!(stream1.next().await, Some(2));
        assert_eq!(stream2.next().await, Some(2));
    }
}
