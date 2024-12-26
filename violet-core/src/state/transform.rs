use std::{marker::PhantomData, sync::Arc};

use futures::StreamExt;

use super::{State, StateMut, StateOwned, StateSink, StateStream};

pub struct Transform<C, U, F, G> {
    inner: C,
    to: Arc<F>,
    from: G,
    _marker: PhantomData<U>,
}

impl<C, U, F, G> Clone for Transform<C, U, F, G>
where
    C: Clone,
    G: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            to: self.to.clone(),
            from: self.from.clone(),
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> Transform<C, U, F, G> {
    pub fn new(inner: C, to: F, from: G) -> Self {
        Self {
            inner,
            to: Arc::new(to),
            from,
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> State for Transform<C, U, F, G> {
    type Item = U;
}

impl<C, U, F, G> StateStream for Transform<C, U, F, G>
where
    C: StateStream,
    C::Item: 'static + Send,
    U: 'static + Send + Sync,
    F: 'static + Fn(&C::Item) -> U + Sync + Send,
{
    fn stream(&self) -> futures::stream::BoxStream<'static, Self::Item> {
        let to = self.to.clone();
        self.inner.stream().map(move |v| to(&v)).boxed()
    }
}

impl<C, U, F, G> StateSink for Transform<C, U, F, G>
where
    C: StateMut,
    C::Item: 'static + Send,
    U: 'static + Send + Sync,
    G: 'static + Fn(&mut C::Item, U) + Sync + Send,
{
    fn send(&self, value: Self::Item) {
        self.inner.write_mut(|v| (self.from)(v, value))
    }
}

impl<C, U, F, G> StateOwned for Transform<C, U, F, G>
where
    C: StateMut,
    C::Item: 'static + Send,
    U: 'static + Send + Sync,
    F: 'static + Fn(&C::Item) -> U + Sync + Send,
{
    fn read(&self) -> Self::Item {
        self.inner.read_ref(|v| (self.to)(v))
    }
}
