use winit::event::ElementState;

use crate::{
    input::{focusable, on_mouse_input},
    ScopeRef,
};

use super::Widget;

pub mod button;
pub mod input;
pub mod slider;

pub trait InteractiveExt {
    fn on_press<F: 'static + Send + Sync + FnMut(&ScopeRef<'_>)>(
        self,
        on_press: F,
    ) -> OnPress<Self, F>
    where
        Self: Sized;
}

impl<W: Widget> InteractiveExt for W {
    fn on_press<F: 'static + Send + Sync + FnMut(&ScopeRef<'_>)>(
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
}

pub struct OnPress<W, F> {
    widget: W,
    func: F,
}

impl<W, F> Widget for OnPress<W, F>
where
    W: Widget,
    F: 'static + Send + Sync + FnMut(&ScopeRef<'_>),
{
    fn mount(mut self, scope: &mut crate::Scope<'_>) {
        self.widget.mount(scope);

        scope
            .set_default(focusable())
            .on_event(on_mouse_input(), move |scope, input| {
                if input.state == ElementState::Released {
                    (self.func)(scope)
                }
            });
    }
}
