use flax::Component;
use palette::Srgba;
use winit::event::{ElementState, MouseButton};

use crate::{
    components::{color, Edges},
    input::{focusable, on_focus, on_mouse_input},
    layout::Alignment,
    style::{
        get_stylesheet, interactive_active, interactive_pressed, spacing, Background, StyleExt,
    },
    widget::{ContainerStyle, Stack, Text},
    Frame, Scope, Widget,
};

type ButtonCallback = Box<dyn Send + Sync + FnMut(&Frame, winit::event::MouseButton)>;

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal_color: Component<Srgba>,
    pub pressed_color: Component<Srgba>,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal_color: interactive_active(),
            pressed_color: interactive_pressed(),
        }
    }
}

/// A button which invokes the callback when clicked
pub struct Button<W = Text> {
    on_press: ButtonCallback,
    label: W,
    style: ButtonStyle,
}

impl<W> Button<W> {
    pub fn new(label: W) -> Self {
        Self {
            on_press: Box::new(|_, _| {}),
            label,
            style: Default::default(),
        }
    }

    /// Handle the button press
    pub fn on_press(
        mut self,
        on_press: impl 'static + Send + Sync + FnMut(&Frame, MouseButton),
    ) -> Self {
        self.on_press = Box::new(on_press);
        self
    }
}

impl Button<Text> {
    pub fn with_label(label: impl Into<String>) -> Self {
        Self::new(Text::new(label.into()))
    }
}

impl<W> StyleExt for Button<W> {
    type Style = ButtonStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl<W: Widget> Widget for Button<W> {
    fn mount(mut self, scope: &mut Scope<'_>) {
        let stylesheet = get_stylesheet(scope);

        let spacing = stylesheet.get_copy(spacing()).unwrap_or_default();
        let margin = Edges::even(spacing.size(2));
        let padding = Edges::even(spacing.size(2));

        let pressed_color = stylesheet
            .get_copy(self.style.pressed_color)
            .unwrap_or_default();

        let normal_color = stylesheet
            .get_copy(self.style.normal_color)
            .unwrap_or_default();
        scope
            .set(focusable(), ())
            .on_event(on_focus(), move |_, entity, focus| {
                entity.update_dedup(color(), if focus { pressed_color } else { normal_color });
            })
            .on_event(on_mouse_input(), move |frame, _, input| {
                if input.state == ElementState::Pressed {
                    (self.on_press)(frame, input.button);
                }
            });

        Stack::new(self.label)
            .with_style(ContainerStyle {
                margin,
                padding,
                background: Some(Background::new(normal_color)),
            })
            .with_horizontal_alignment(Alignment::Center)
            .with_vertical_alignment(Alignment::Center)
            .mount(scope);
    }
}
