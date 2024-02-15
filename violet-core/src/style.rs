use std::ops::{Deref, DerefMut};

use flax::{component::ComponentValue, Component};
use glam::Vec2;

use crate::{
    components::{self, Edges},
    unit::Unit,
    Widget,
};

pub trait StyleExt {
    fn with_margin(self, margin: Edges) -> WithComponent<Self, Edges>
    where
        Self: Sized,
    {
        self.with_component(components::margin(), margin)
    }

    fn with_min_size(self, min_size: Unit<Vec2>) -> WithComponent<Self, Unit<Vec2>>
    where
        Self: Sized,
    {
        self.with_component(components::min_size(), min_size)
    }

    fn with_size(self, size: Unit<Vec2>) -> WithComponent<Self, Unit<Vec2>>
    where
        Self: Sized,
    {
        self.with_component(components::size(), size)
    }

    fn with_aspect_ratio(self, aspect_ratio: f32) -> WithComponent<Self, f32>
    where
        Self: Sized,
    {
        self.with_component(components::aspect_ratio(), aspect_ratio)
    }

    #[inline]
    fn with_component<T: ComponentValue>(
        self,
        component: Component<T>,
        value: T,
    ) -> WithComponent<Self, T>
    where
        Self: Sized,
    {
        WithComponent::new(self, component, value)
    }
}

impl<W> StyleExt for W where W: Widget {}

/// A widget extended with a single component
#[derive(Debug, Clone)]
pub struct WithComponent<W, T> {
    widget: W,
    component: Component<T>,
    value: T,
}

impl<W, T> Deref for WithComponent<W, T> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<W, T> DerefMut for WithComponent<W, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<W, T> WithComponent<W, T> {
    pub fn new(widget: W, component: Component<T>, value: T) -> Self {
        Self {
            widget,
            component,
            value,
        }
    }
}

impl<W: Widget, T: ComponentValue> Widget for WithComponent<W, T> {
    #[inline]
    fn mount(self, scope: &mut crate::Scope<'_>) {
        self.widget.mount(scope);
        scope.set(self.component, self.value);
    }
}
