use std::{marker::PhantomData, pin::Pin, task::Poll};

use futures::{ready, Future};
use pin_project::pin_project;

use crate::Effect;

/// Execute an effect upon the world when the provided future resolves
#[pin_project]
pub struct FutureEffect<Data, Fut, F> {
    #[pin]
    fut: Fut,
    func: Option<F>,
    _marker: PhantomData<Data>,
}

impl<Data, Fut, F> FutureEffect<Data, Fut, F>
where
    Fut: Future,
    F: FnOnce(&mut Data, Fut::Output),
{
    pub fn new(fut: Fut, func: F) -> Self {
        Self {
            fut,
            func: Some(func),
            _marker: PhantomData,
        }
    }
}

impl<Fut, F, Data> Effect<Data> for FutureEffect<Data, Fut, F>
where
    Fut: Future,
    F: FnOnce(&mut Data, Fut::Output),
{
    fn poll(self: Pin<&mut Self>, context: &mut std::task::Context, frame: &mut Data) -> Poll<()> {
        let p = self.project();

        let val = ready!(p.fut.poll(context));
        (p.func.take().unwrap())(frame, val);

        Poll::Ready(())
    }
}
