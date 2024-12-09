use std::{marker::PhantomData, sync::Arc};

use futures::{stream::BoxStream, StreamExt};

use super::{State, StateOwned, StateSink, StateStream};

/// Transforms one state to another through type conversion
///
///
/// This allows deriving state from another where the derived state is not present in the original.
///
/// However, as this does not assume the derived state is contained withing the original state is
/// does not allow in-place mutation.
pub struct Map<C, U, F, G> {
    inner: C,
    conv_to: Arc<F>,
    conv_from: G,
    _marker: PhantomData<U>,
}

impl<C, U, F, G> Clone for Map<C, U, F, G>
where
    C: Clone,
    G: Clone,
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

impl<C, U, F, G> State for Map<C, U, F, G> {
    type Item = U;
}

impl<C: State, U, F: Fn(C::Item) -> U, G: Fn(U) -> C::Item> Map<C, U, F, G> {
    pub fn new(inner: C, project: F, project_mut: G) -> Self {
        Self {
            inner,
            conv_to: Arc::new(project),
            conv_from: project_mut,
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> StateOwned for Map<C, U, F, G>
where
    C: StateOwned,
    F: Fn(C::Item) -> U,
{
    fn read(&self) -> Self::Item {
        (self.conv_to)(self.inner.read())
    }
}

impl<C, U, F, G> StateStream for Map<C, U, F, G>
where
    C: StateStream,
    C::Item: 'static + Send,
    U: 'static + Send + Sync,
    F: 'static + Fn(C::Item) -> U + Sync + Send,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        let project = self.conv_to.clone();
        self.inner.stream().map(move |v| (project)(v)).boxed()
    }
}

/// Bridge update-by-reference to update-by-value
impl<C, U, F, G> StateSink for Map<C, U, F, G>
where
    C: StateSink,
    F: Fn(C::Item) -> U,
    G: Fn(U) -> C::Item,
{
    fn send(&self, value: Self::Item) {
        self.inner.send((self.conv_from)(value))
    }
}
