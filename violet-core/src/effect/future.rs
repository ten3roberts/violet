use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{ready, Future};
use pin_project::pin_project;

use super::Effect;

/// Execute an effect upon the world when the provided future resolves
#[pin_project]
pub struct FutureEffect<Fut, F> {
    #[pin]
    fut: Fut,
    func: Option<F>,
}

impl<Fut, F> FutureEffect<Fut, F> {
    pub fn new(fut: Fut, func: F) -> Self {
        Self {
            fut,
            func: Some(func),
        }
    }
}

impl<Fut, F, Data> Effect<Data> for FutureEffect<Fut, F>
where
    Fut: Future,
    F: FnOnce(&mut Data, Fut::Output),
{
    fn poll(self: Pin<&mut Self>, context: &mut Context, frame: &mut Data) -> Poll<()> {
        let p = self.project();

        let val = ready!(p.fut.poll(context));
        (p.func.take().unwrap())(frame, val);

        Poll::Ready(())
    }
}
