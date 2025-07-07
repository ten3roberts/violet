use std::{marker::PhantomData, sync::Arc};

use futures::{stream::BoxStream, StreamExt};

use super::{State, StateOwned, StateSink, StateStream};

/// Two way map to convert a stat of type `T` to type `U` and back.
///
/// Implements StateOwned, StateStream, and StateSink traits depending on the underlying state.
pub struct MapValue<C, U: ?Sized, F, G> {
    inner: C,
    conv_to: Arc<F>,
    conv_from: Arc<G>,
    _marker: PhantomData<U>,
}

impl<C, U, F, G> Clone for MapValue<C, U, F, G>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            conv_to: self.conv_to.clone(),
            conv_from: self.conv_from.clone(),
            _marker: PhantomData,
        }
    }
}

impl<C, U: ?Sized, F, G> State for MapValue<C, U, F, G> {
    type Item = U;
}

impl<C: State, U, F: Fn(C::Item) -> U, G: Fn(U) -> C::Item> MapValue<C, U, F, G>
where
    C::Item: Sized,
{
    pub fn new(inner: C, project: F, project_mut: G) -> Self {
        Self {
            inner,
            conv_to: Arc::new(project),
            conv_from: Arc::new(project_mut),
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> StateOwned for MapValue<C, U, F, G>
where
    C: StateOwned,
    F: Fn(C::Item) -> U,
    C::Item: Sized,
{
    fn read(&self) -> Self::Item {
        (self.conv_to)(self.inner.read())
    }
}

impl<C, U, F, G> StateStream for MapValue<C, U, F, G>
where
    C: StateStream,
    C::Item: 'static + Send,
    U: 'static + Send + Sync,
    F: 'static + Fn(C::Item) -> U + Sync + Send,
    C::Item: Sized,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        let project = self.conv_to.clone();
        self.inner.stream().map(move |v| (project)(v)).boxed()
    }
}

/// Bridge update-by-reference to update-by-value
impl<C, U, F, G> StateSink for MapValue<C, U, F, G>
where
    C: StateSink,
    F: Fn(C::Item) -> U,
    G: Fn(U) -> C::Item,
    C::Item: Sized,
{
    fn send(&self, value: Self::Item) {
        self.inner.send((self.conv_from)(value))
    }
}
