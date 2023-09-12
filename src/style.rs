use flax::{Component, ComponentValue};

use crate::{
    components::{self, Edges},
    Widget,
};

pub trait StyleExt {
    fn with_margin(self, margin: Edges) -> WithComponent<Self, Edges>
    where
        Self: Sized;

    fn with_padding(self, padding: Edges) -> WithComponent<Self, Edges>
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
    #[inline]
    fn with_margin(self, margin: Edges) -> WithComponent<Self, Edges> {
        WithComponent::new(self, components::margin(), margin)
    }

    #[inline]
    fn with_padding(self, padding: Edges) -> WithComponent<Self, Edges> {
        WithComponent::new(self, components::padding(), padding)
    }
}
