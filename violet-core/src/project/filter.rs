use std::{future::ready, marker::PhantomData, sync::Arc};

use futures::{stream::BoxStream, StreamExt};

use super::{State, StateSink, StateStream};

/// Transforms one type to another through fallible conversion.
pub struct FilterDuplex<C, U, F, G> {
    inner: C,
    conv_to: Arc<F>,
    conv_from: G,
    _marker: PhantomData<U>,
}

impl<C, U, F, G> State for FilterDuplex<C, U, F, G> {
    type Item = U;
}

impl<C: State, U, F: Fn(C::Item) -> Option<U>, G: Fn(U) -> Option<C::Item>>
    FilterDuplex<C, U, F, G>
{
    pub fn new(inner: C, conv_to: F, conv_from: G) -> Self {
        Self {
            inner,
            conv_to: Arc::new(conv_to),
            conv_from,
            _marker: PhantomData,
        }
    }
}

impl<C, U, F, G> StateStream for FilterDuplex<C, U, F, G>
where
    C: StateStream,
    C::Item: 'static + Send,
    U: 'static + Send + Sync + Clone,
    F: 'static + Send + Sync + Fn(C::Item) -> Option<U>,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        let project = self.conv_to.clone();
        self.inner
            .stream()
            .filter_map(move |v| ready(project(v)))
            .boxed()
    }
}

/// Bridge update-by-reference to update-by-value
impl<C, U, F, G> StateSink for FilterDuplex<C, U, F, G>
where
    C: StateSink,
    G: Fn(U) -> Option<C::Item>,
{
    fn send(&self, value: Self::Item) {
        if let Some(v) = (self.conv_from)(value) {
            self.inner.send(v)
        }
    }
}
