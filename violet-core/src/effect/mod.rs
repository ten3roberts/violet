mod future;
mod stream;

use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub use future::FutureEffect;
use pin_project::pin_project;
pub use stream::StreamEffect;

/// An asynchronous computation which has access to `Data` when polled
///
///
/// Similar to [`std::future::Future`] but provides mutable access to shared data during poll
pub trait Effect<Data> {
    /// Polls the effect
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>, data: &mut Data) -> Poll<()>;
    fn label(&self) -> Option<&str> {
        None
    }

    fn with_label(self, label: impl Into<String>) -> EffectWithLabel<Self>
    where
        Self: Sized,
    {
        EffectWithLabel::new(self, Some(label.into()))
    }
}

#[pin_project]
#[doc(hidden)]
pub struct EffectWithLabel<E> {
    #[pin]
    effect: E,
    label: Option<String>,
}

impl<E> EffectWithLabel<E> {
    pub fn new(effect: E, label: Option<String>) -> Self {
        Self { effect, label }
    }
}

impl<E, Data> Effect<Data> for EffectWithLabel<E>
where
    E: Effect<Data>,
{
    #[inline]
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>, data: &mut Data) -> Poll<()> {
        self.project().effect.poll(context, data)
    }

    fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
}
