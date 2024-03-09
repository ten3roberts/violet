use std::{marker::PhantomData, sync::Arc};

use futures::{stream::BoxStream, Stream, StreamExt};

use super::{ProjectOwned, ProjectRef, ProjectSink, ProjectStreamOwned, ProjectStreamRef};

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
