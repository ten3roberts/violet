use flax::components::name;
use palette::Srgba;
use winit::event::ElementState;

use crate::{
    components::{color, filled_rect},
    input::{on_focus, on_mouse_input},
    shapes::FilledRect,
    Frame, Scope, Widget,
};

/// A rectangular widget
pub struct Rectangle {
    color: Srgba,
}

impl Rectangle {
    pub fn new(color: Srgba) -> Self {
        Self { color }
    }
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "Rectangle".into())
            .set(
                filled_rect(),
                FilledRect {
                    color: self.color,
                    fill_image: None,
                },
            )
            .set(color(), self.color);
    }
}

type ButtonCallback = Box<dyn Send + Sync + FnMut(&Frame, winit::event::MouseButton)>;

/// A button which invokes the callback when clicked
pub struct Button {
    normal_color: Srgba,
    pressed_color: Srgba,

    on_click: ButtonCallback,
}

impl Button {
    pub fn new(normal_color: Srgba, pressed_color: Srgba, on_click: ButtonCallback) -> Self {
        Self {
            normal_color,
            pressed_color,
            on_click,
        }
    }
}

impl Widget for Button {
    fn mount(mut self, scope: &mut Scope<'_>) {
        scope
            .set(
                filled_rect(),
                FilledRect {
                    color: self.normal_color,
                    fill_image: None,
                },
            )
            .set(color(), self.normal_color)
            .set(
                on_focus(),
                Box::new(move |_, entity, focus| {
                    entity.update_dedup(
                        color(),
                        if focus {
                            self.pressed_color
                        } else {
                            self.normal_color
                        },
                    );
                }),
            )
            .set(
                on_mouse_input(),
                Box::new(move |frame, _, state, button| {
                    if state == ElementState::Released {
                        (self.on_click)(frame, button);
                    }
                }),
            );
    }
}
