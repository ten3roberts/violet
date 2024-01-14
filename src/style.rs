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
        Self: Sized;

    fn with_min_size(self, min_size: Unit<Vec2>) -> WithComponent<Self, Unit<Vec2>>
    where
        Self: Sized;

    fn with_size(self, size: Unit<Vec2>) -> WithComponent<Self, Unit<Vec2>>
    where
        Self: Sized;
}

/// A widget extended with a single component
pub struct WithComponent<W, T> {
    widget: W,
    component: Component<T>,
    value: T,
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

impl<W> StyleExt for W
where
    W: Widget,
{
    fn with_margin(self, margin: Edges) -> WithComponent<Self, Edges> {
        WithComponent::new(self, components::margin(), margin)
    }

    fn with_size(self, size: Unit<Vec2>) -> WithComponent<Self, Unit<Vec2>> {
        WithComponent::new(self, components::size(), size)
    }

    fn with_min_size(self, min_size: Unit<Vec2>) -> WithComponent<Self, Unit<Vec2>> {
        WithComponent::new(self, components::min_size(), min_size)
    }
}
