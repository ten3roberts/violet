use std::task::Poll;

use futures::{ready, Future, Stream};

#[macro_export]
macro_rules! to_owned {
    ($($ident: ident),*) => (
        $(let $ident = $ident.to_owned();)*
    )
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

#[pin_project::pin_project]
pub struct Throttle<S, F, C> {
    #[pin]
    stream: S,
    #[pin]
    fut: Option<F>,
    throttle: C,
}

impl<S, F, C> Throttle<S, F, C> {
    pub fn new(stream: S, throttle: C) -> Self {
        Self {
            stream,
            fut: None,
            throttle,
        }
    }
}

impl<S, F, C> Stream for Throttle<S, F, C>
where
    S: Stream,
    F: Future<Output = ()>,
    C: FnMut() -> F,
{
    type Item = S::Item;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut p = self.project();

        if let Some(fut) = p.fut.as_mut().as_pin_mut() {
            ready!(fut.poll(cx));
            p.fut.set(None);
        }

        let item = ready!(p.stream.poll_next(cx));

        if let Some(item) = item {
            p.fut.set(Some((p.throttle)()));
            Poll::Ready(Some(item))
        } else {
            Poll::Ready(None)
        }
    }
}

/// Throttles a stream with the provided future
pub fn throttle<S, F, C>(stream: S, throttle: C) -> Throttle<S, F, C>
where
    S: Stream,
    F: Future<Output = ()>,
    C: FnMut() -> F,
{
    Throttle::new(stream, throttle)
}
