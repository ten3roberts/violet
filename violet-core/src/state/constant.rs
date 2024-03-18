use futures::{prelude::stream::BoxStream, StreamExt};

use super::{State, StateRef, StateStream};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Constant<T>(pub T);

impl<T> State for Constant<T> {
    type Item = T;
}

impl<T> StateRef for Constant<T> {
    type Item = T;

    fn read_ref<F: FnOnce(&Self::Item) -> V, V>(&self, f: F) -> V {
        (f)(&self.0)
    }
}

impl<T> StateStream for Constant<T>
where
    T: 'static + Send + Sync + Clone,
{
    fn stream(&self) -> BoxStream<'static, Self::Item> {
        let value = self.0.clone();
        futures::stream::once(async move { value }).boxed()
    }
}
