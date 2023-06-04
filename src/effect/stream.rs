use std::{marker::PhantomData, pin::Pin, task::Poll};

use futures::{ready, Stream};
use pin_project::pin_project;

use super::Effect;

/// Execute an effect upon the world for each item in the stream
#[pin_project]
pub struct StreamEffect<Data, Fut, F> {
    #[pin]
    stream: Fut,
    func: F,
    _marker: PhantomData<Data>,
}

impl<Data, S, F> StreamEffect<Data, S, F>
where
    S: Stream,
    F: FnMut(&mut Data, S::Item),
{
    pub fn new(stream: S, func: F) -> Self {
        Self {
            stream,
            func,
            _marker: PhantomData,
        }
    }
}

impl<S, F, Data> Effect<Data> for StreamEffect<Data, S, F>
where
    S: Stream,
    F: FnMut(&mut Data, S::Item),
{
    fn poll(self: Pin<&mut Self>, context: &mut std::task::Context, frame: &mut Data) -> Poll<()> {
        let p = self.project();

        let mut stream = p.stream;
        let func = p.func;

        while let Some(val) = ready!(stream.as_mut().poll_next(context)) {
            func(frame, val);
        }

        Poll::Ready(())
    }
}
