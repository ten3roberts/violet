use crate::Scope;
mod future;

pub use future::{SignalWidget, StreamWidget};

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

/// Represents a list of widgets
pub trait WidgetCollection {
    fn attach(self, scope: &mut Scope);
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

macro_rules! tuple_impl {
    ($($idx: tt => $ty: ident),*) => {
        impl<$($ty),*> WidgetCollection for ($($ty,)*)
            where $($ty: Widget,)*
        {
            fn attach(self, scope: &mut Scope<'_>) {
                $(
                    scope.attach(self.$idx);
                )*
            }
        }
    };
}

tuple_impl! { 0 => A }
tuple_impl! { 0 => A, 1 => B }
tuple_impl! { 0 => A, 1 => B, 2 => C }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E }
tuple_impl! { 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F }
