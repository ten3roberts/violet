use winit::event::ElementState;

use crate::{
    input::{interactive, on_keyboard_input, on_mouse_input, KeyboardInput},
    ScopeRef,
};

use super::Widget;

pub mod button;
pub mod drag;
pub mod input;
pub mod overlay;
pub mod slider;
pub mod tooltip;

pub trait InteractiveExt {
    fn on_press<F: 'static + Send + Sync + FnMut(&ScopeRef<'_>) -> Option<()>>(
        self,
        on_press: F,
    ) -> OnPress<Self, F>
    where
        Self: Sized;

    fn on_key<
        F: 'static + Send + Sync + FnMut(&ScopeRef<'_>, KeyboardInput) -> Option<KeyboardInput>,
    >(
        self,
        on_key: F,
    ) -> OnKey<Self, F>
    where
        Self: Sized;
}

impl<W: Widget> InteractiveExt for W {
    fn on_press<F: 'static + Send + Sync + FnMut(&ScopeRef<'_>) -> Option<()>>(
        self,
        on_press: F,
    ) -> OnPress<Self, F>
    where
        Self: Sized,
    {
        OnPress {
            widget: self,
            func: on_press,
        }
    }

    fn on_key<
        F: 'static + Send + Sync + FnMut(&ScopeRef<'_>, KeyboardInput) -> Option<KeyboardInput>,
    >(
        self,
        on_key: F,
    ) -> OnKey<Self, F>
    where
        Self: Sized,
    {
        OnKey {
            widget: self,
            func: on_key,
        }
    }
}

pub struct OnPress<W, F> {
    widget: W,
    func: F,
}

impl<W, F> Widget for OnPress<W, F>
where
    W: Widget,
    F: 'static + Send + Sync + FnMut(&ScopeRef<'_>) -> Option<()>,
{
    fn mount(mut self, scope: &mut crate::Scope<'_>) {
        self.widget.mount(scope);

        scope
            .set_default(interactive())
            .on_event(on_mouse_input(), move |scope, input| {
                if input.state == ElementState::Pressed {
                    (self.func)(scope).map(|()| input)
                } else {
                    None
                }
            });
    }
}

pub struct OnKey<W, F> {
    widget: W,
    func: F,
}

impl<W, F> Widget for OnKey<W, F>
where
    W: Widget,
    F: 'static + Send + Sync + FnMut(&ScopeRef<'_>, KeyboardInput) -> Option<KeyboardInput>,
{
    fn mount(mut self, scope: &mut crate::Scope<'_>) {
        self.widget.mount(scope);

        scope
            .set_default(interactive())
            .on_event(on_keyboard_input(), move |scope, input| {
                (self.func)(scope, input)
            });
    }
}
