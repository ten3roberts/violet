use palette::Srgba;
use winit::event::{ElementState, MouseButton};

use crate::{
    components::color,
    input::{focusable, on_mouse_input},
    layout::Alignment,
    style::{
        danger_item, interactive_inactive, interactive_pressed, spacing_medium, success_item,
        warning_item, Background, SizeExt, StyleExt, ValueOrRef, WidgetSize,
    },
    widget::{ContainerStyle, Stack, Text},
    Frame, Scope, Widget,
};

type ButtonCallback = Box<dyn Send + Sync + FnMut(&Frame, winit::event::MouseButton)>;

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal_color: ValueOrRef<Srgba>,
    pub pressed_color: ValueOrRef<Srgba>,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal_color: interactive_inactive().into(),
            pressed_color: interactive_pressed().into(),
        }
    }
}

/// A button which invokes the callback when clicked
pub struct Button<W = Text> {
    on_press: ButtonCallback,
    label: W,
    style: ButtonStyle,
    size: WidgetSize,
}

impl<W> Button<W> {
    pub fn new(label: W) -> Self
    where
        W: Widget,
    {
        Self {
            on_press: Box::new(|_, _| {}),
            label,
            style: Default::default(),
            size: WidgetSize::default().with_padding(spacing_medium()),
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

    pub fn success(mut self) -> Self {
        self.style.normal_color = success_item().into();
        self
    }

    pub fn danger(mut self) -> Self {
        self.style.normal_color = danger_item().into();
        self
    }

    pub fn warning(mut self) -> Self {
        self.style.normal_color = warning_item().into();
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

impl<W> SizeExt for Button<W> {
    fn size_mut(&mut self) -> &mut WidgetSize {
        &mut self.size
    }
}

impl<W: Widget> Widget for Button<W> {
    fn mount(mut self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed_color = self.style.pressed_color.resolve(stylesheet);
        let normal_color = self.style.normal_color.resolve(stylesheet);

        scope
            .set(focusable(), ())
            .on_event(on_mouse_input(), move |frame, entity, input| {
                if input.state == ElementState::Pressed {
                    entity.update_dedup(color(), pressed_color);
                    (self.on_press)(frame, input.button);
                } else {
                    entity.update_dedup(color(), normal_color);
                }
            });

        Stack::new(self.label)
            .with_style(ContainerStyle {
                background: Some(Background::new(normal_color)),
            })
            .with_horizontal_alignment(Alignment::Center)
            .with_vertical_alignment(Alignment::Center)
            .with_size_props(self.size)
            .mount(scope);
    }
}
