use crate::Scope;
mod basic;
mod container;
mod future;
mod input;

pub use basic::{BoxSized, Image, Positioned, Rectangle, Text};
pub use container::{ContainerStyle, List, Movable, Stack};
use flax::{component::ComponentValue, components::name, Component};
pub use future::{Signal, StreamWidget};
use futures_signals::signal::Mutable;
pub use input::{
    button::*,
    slider::{Slider, SliderStyle, SliderValue, SliderWithLabel},
};

/// Represents a widget in the UI tree which can mount itself into the frame.
///
/// Is inert before mounting
pub trait Widget: BoxedWidget {
    /// Mount the widget into the world, returning a handle to refer to it
    fn mount(self, scope: &mut Scope<'_>);
}

pub trait BoxedWidget {
    fn mount_boxed(self: Box<Self>, scope: &mut Scope<'_>);
}

impl<T> BoxedWidget for T
where
    T: Widget,
{
    fn mount_boxed(self: Box<Self>, scope: &mut Scope<'_>) {
        (*self).mount(scope)
    }
}

impl<T> Widget for Box<T>
where
    T: ?Sized + Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.mount_boxed(scope)
    }
}

impl<T: Widget> Widget for Option<T> {
    fn mount(self, scope: &mut Scope<'_>) {
        if let Some(widget) = self {
            widget.mount(scope);
        }
    }
}

pub trait WidgetExt: Widget + Sized {
    fn boxed<'a>(self) -> Box<dyn 'a + Widget>
    where
        Self: 'a + Sized,
    {
        Box::new(self)
    }

    fn with_name(self, name: impl Into<String>) -> Named<Self> {
        Named {
            name: name.into(),
            widget: self,
        }
    }

    fn monitor<T: ComponentValue>(
        self,
        component: Component<T>,
        on_change: Box<dyn Fn(Option<&T>)>,
    ) -> Monitor<Self, T> {
        Monitor {
            widget: self,
            component,
            on_change,
        }
    }

    fn monitor_signal<T: Clone + ComponentValue>(
        self,
        component: Component<T>,
        on_change: Mutable<Option<T>>,
    ) -> Monitor<Self, T> {
        Monitor {
            widget: self,
            component,
            on_change: Box::new(move |val| {
                on_change.set(val.cloned());
            }),
        }
    }
}

pub struct Monitor<W, T> {
    widget: W,
    component: Component<T>,
    on_change: Box<dyn Fn(Option<&T>)>,
}

impl<W: Widget, T: Clone + ComponentValue> Widget for Monitor<W, T> {
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);
        scope.monitor(self.component, self.on_change);
    }
}

/// An explicitly named widget. Used for diagnostic purposes
pub struct Named<W> {
    widget: W,
    name: String,
}

impl<W: Widget> Widget for Named<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);
        scope.set(name(), self.name);
    }
}

impl<W> WidgetExt for W where W: Widget {}

/// Represents a list of widgets
pub trait WidgetCollection {
    fn attach(self, scope: &mut Scope);
}

impl<W> WidgetCollection for W
where
    W: Widget,
{
    fn attach(self, scope: &mut Scope) {
        scope.attach(self);
    }
}

impl<const C: usize, W: Widget> WidgetCollection for [W; C] {
    fn attach(self, scope: &mut Scope) {
        for widget in self {
            scope.attach(widget);
        }
    }
}

impl<W: Widget> WidgetCollection for Vec<W> {
    fn attach(self, scope: &mut Scope) {
        for widget in self {
            scope.attach(widget);
        }
    }
}

pub struct NoOp;

impl Widget for NoOp {
    fn mount(self, _scope: &mut Scope<'_>) {}
}

macro_rules! tuple_impl {
    ($($idx: tt => $ty: ident),*) => {
        impl<$($ty),*> WidgetCollection for ($($ty,)*)
            where $($ty: Widget,)*
        {
            fn attach(self, _scope: &mut Scope<'_>) {
                $(
                    _scope.attach(self.$idx);
                )*
            }
        }
    };
}

tuple_impl! {}
tuple_impl! { 0 => A }
tuple_impl! { 0 => A, 1 => B }
tuple_impl! { 0 => A, 1 => B, 2 => C }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H }