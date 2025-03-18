use crate::Scope;
mod basic;
mod container;
mod future;
pub mod image;
mod interactive;
mod scroll;

pub use basic::*;
pub use container::*;
use flax::{component::ComponentValue, components::name, Component};
pub use future::{SignalWidget, StreamWidget};
use futures_signals::signal::Mutable;
pub use image::*;
pub use interactive::{button::*, drag::*, input::*, overlay::*, slider::*, InteractiveExt};
pub use scroll::ScrollArea;

/// A widget is a description of a part of the Ui with the capability to mount itself into the world.
///
/// This trait rarely required Send nor Sync, or a static lifetime as it does remain in the world after it is mounted.
pub trait Widget: BoxedWidget {
    /// Mount the widget into the world, returning a handle to refer to it
    fn mount(self, scope: &mut Scope<'_>);

    fn name(&self) -> String {
        tynm::type_name::<Self>()
    }
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

impl Widget for Box<dyn Widget>
// where
//     T: ?Sized + Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.mount_boxed(scope)
    }

    fn name(&self) -> String {
        (**self).name()
    }
}

impl Widget for Box<dyn Send + Sync + Widget>
// where
//     T: ?Sized + Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.mount_boxed(scope)
    }

    fn name(&self) -> String {
        (**self).name()
    }
}

impl<T: Widget> Widget for Option<T> {
    fn mount(self, scope: &mut Scope<'_>) {
        if let Some(widget) = self {
            widget.mount(scope);
        }
    }
}

impl<F> Widget for F
where
    F: FnOnce(&mut Scope<'_>),
{
    fn mount(self, scope: &mut Scope<'_>) {
        self(scope);
    }
}

pub type OnChangeCallback<T> = dyn Fn(Option<&T>);

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
        on_change: Box<OnChangeCallback<T>>,
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
    on_change: Box<OnChangeCallback<T>>,
}

impl<W: Widget, T: Clone + ComponentValue> Widget for Monitor<W, T> {
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);
        scope.monitor(self.component, self.on_change);
    }
}

/// An explicitly named widget. Used for diagnostic purposes
#[derive(Debug, Clone)]
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

pub struct EmptyWidget;

impl Widget for EmptyWidget {
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
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H, 8 => I, 9 => J, 10 => K, 11 => L, 12 => M }
