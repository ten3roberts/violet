use std::{pin::Pin, task::Poll};

use futures::{ready, Future, FutureExt, Stream};

#[macro_export]
macro_rules! to_owned {
    ($ident: ident, $($rest: tt)*) => {
        to_owned!($ident);
        to_owned!($($rest)*);
    };
    ($ident: ident=$expr: expr, $($rest: tt)*) => {
        to_owned!($ident=$expr);
        to_owned!($($rest)*);
    };
    ($ident: ident=$expr: expr) => {
        let $ident = $expr.to_owned();
    };
    ($ident: ident) => {
        let $ident = $ident.to_owned();
    };
    () => {};
}

/// Combines two streams yielding the latest value from each stream
pub fn zip_latest_ref<A, B, F, V>(a: A, b: B, func: F) -> ZipLatest<A, B, F>
where
    A: Stream,
    B: Stream,
    F: Fn(&A::Item, &B::Item) -> V,
{
    ZipLatest::new(a, b, func)
}

/// Combines two streams yielding the latest value from each stream
#[allow(clippy::type_complexity)]
pub fn zip_latest<A, B>(
    a: A,
    b: B,
) -> ZipLatest<A, B, impl Fn(&A::Item, &B::Item) -> (A::Item, B::Item)>
where
    A: Stream,
    B: Stream,
    A::Item: Clone,
    B::Item: Clone,
{
    ZipLatest::new(a, b, |a: &A::Item, b: &B::Item| (a.clone(), b.clone()))
}
#[pin_project::pin_project]
pub struct ZipLatest<A: Stream, B: Stream, F> {
    #[pin]
    a: A,
    #[pin]
    b: B,
    b_item: Option<B::Item>,
    a_item: Option<A::Item>,
    func: F,
}

impl<A: Stream, B: Stream, F> ZipLatest<A, B, F> {
    pub fn new(a: A, b: B, func: F) -> Self {
        Self {
            a,
            b,
            a_item: None,
            b_item: None,
            func,
        }
    }
}

impl<A, B, F, V> Stream for ZipLatest<A, B, F>
where
    A: Stream,
    B: Stream,
    F: FnMut(&A::Item, &B::Item) -> V,
{
    type Item = V;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let p = self.project();
        let mut ready = false;

        if let Poll::Ready(value) = p.a.poll_next(cx) {
            *p.a_item = value;
            ready = true;
        }

        if let Poll::Ready(value) = p.b.poll_next(cx) {
            *p.b_item = value;
            ready = true;
        }

        match (&p.a_item, &p.b_item) {
            (Some(a), Some(b)) if ready => Poll::Ready(Some((p.func)(a, b))),
            _ => Poll::Pending,
        }
    }
}

/// Throttles a stream with another future
#[pin_project::pin_project]
pub struct Throttle<S, T, F> {
    #[pin]
    stream: S,
    future: F,
    #[pin]
    pending: Option<T>,
    skip: bool,
}

impl<S, T, F> Throttle<S, T, F> {
    pub fn new(stream: S, future: F) -> Self {
        Self {
            stream,
            future,
            pending: None,
            skip: false,
        }
    }

    pub fn skip(stream: S, future: F) -> Self {
        Self {
            stream,
            future,
            pending: None,
            skip: true,
        }
    }
}

impl<S, T, F> Stream for Throttle<S, T, F>
where
    S: Stream,
    F: FnMut() -> T,
    T: Future,
{
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut p = self.project();

        if let Some(pending) = p.pending.as_mut().as_pin_mut() {
            ready!(pending.poll(cx));
            p.pending.set(None);
        }

        if *p.skip {
            let mut item = None;

            loop {
                match p.stream.as_mut().poll_next(cx) {
                    Poll::Ready(Some(v)) => {
                        item = Some(v);
                        break;
                    }
                    Poll::Ready(None) => return Poll::Ready(None),
                    Poll::Pending => break,
                }
            }

            p.pending.set(Some((p.future)()));

            Poll::Ready(item)
        } else {
            let item = ready!(p.stream.poll_next(cx));
            p.pending.set(Some((p.future)()));
            Poll::Ready(item)
        }
    }
}

/// Throttles a stream with the provided future
pub fn throttle<S, F, T>(stream: S, throttle: F) -> Throttle<S, T, F>
where
    S: Stream,
    T: Future<Output = ()>,
    F: FnMut() -> T,
{
    Throttle::new(stream, throttle)
}

/// Throttles a stream with the provided future
pub fn throttle_skip<S, F, T>(stream: S, throttle: F) -> Throttle<S, T, F>
where
    S: Stream,
    T: Future<Output = ()>,
    F: FnMut() -> T,
{
    Throttle::new(stream, throttle)
}
