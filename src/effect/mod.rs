mod future;
mod stream;

use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub use future::FutureEffect;
pub use stream::StreamEffect;

/// An asynchronous computation which has access to `Data` when polled
///
///
/// Similar to [`std::future::Future`] but provides mutable access to shared data during poll
pub trait Effect<Data> {
    /// Polls the effect
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>, data: &mut Data) -> Poll<()>;
}
