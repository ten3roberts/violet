use palette::Srgba;
use winit::event::{ElementState, MouseButton};

use crate::{
    components::{self, color},
    input::{focusable, on_mouse_input},
    layout::Align,
    scope::ScopeRef,
    state::{StateDuplex, StateStream, WatchState},
    style::{
        danger_element, interactive_passive, interactive_pressed, spacing_medium, success_element,
        warning_element, Background, SizeExt, StyleExt, ValueOrRef, WidgetSize,
    },
    unit::Unit,
    widget::{ContainerStyle, Stack, Text},
    Scope, Widget, WidgetCollection,
};

type ButtonCallback = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>, winit::event::MouseButton)>;

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal_color: ValueOrRef<Srgba>,
    pub pressed_color: ValueOrRef<Srgba>,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal_color: interactive_passive().into(),
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
    is_pressed: bool,
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
            size: WidgetSize::default()
                .with_padding(spacing_medium())
                .with_margin(spacing_medium()),
            // .with_min_size(Unit::px2(28.0, 28.0)),
            is_pressed: false,
        }
    }

    /// Handle the button press
    pub fn on_press(
        mut self,
        on_press: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>, MouseButton),
    ) -> Self {
        self.on_press = Box::new(on_press);
        self
    }

    pub fn success(mut self) -> Self {
        self.style.normal_color = success_element().into();
        self
    }

    pub fn danger(mut self) -> Self {
        self.style.normal_color = danger_element().into();
        self
    }

    pub fn warning(mut self) -> Self {
        self.style.normal_color = warning_element().into();
        self
    }

    pub fn is_pressed(mut self, pressed: bool) -> Self {
        self.is_pressed = pressed;
        self
    }
}

impl Button<Text> {
    pub fn label(label: impl Into<String>) -> Self {
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

        let pressed_color = self.style.pressed_color.resolve(&stylesheet);
        let normal_color = self.style.normal_color.resolve(&stylesheet);

        scope
            .set(focusable(), ())
            .on_event(on_mouse_input(), move |scope, input| {
                if input.state == ElementState::Pressed {
                    scope.update_dedup(color(), pressed_color);
                    (self.on_press)(scope, input.button);
                } else {
                    scope.update_dedup(color(), normal_color);
                }
            });

        Stack::new(self.label)
            .with_style(ContainerStyle {
                background: Some(Background::new(normal_color)),
            })
            .with_horizontal_alignment(Align::Center)
            .with_vertical_alignment(Align::Center)
            .with_size_props(self.size)
            .mount(scope);
    }
}

pub struct Checkbox<W = ()> {
    state: Box<dyn Send + Sync + StateDuplex<Item = bool>>,
    style: ButtonStyle,
    size: WidgetSize,
    label: W,
}

impl<W: WidgetCollection> Checkbox<W> {
    pub fn new(label: W, state: impl 'static + Send + Sync + StateDuplex<Item = bool>) -> Self {
        Self {
            state: Box::new(state),
            style: Default::default(),
            size: WidgetSize::default()
                .with_padding(spacing_medium())
                .with_margin(spacing_medium())
                .with_min_size(Unit::px2(28.0, 28.0)),
            label,
        }
    }
}

impl Checkbox<Text> {
    pub fn label(
        label: impl Into<String>,
        state: impl 'static + Send + Sync + StateDuplex<Item = bool>,
    ) -> Self {
        Self::new(Text::new(label.into()), state)
    }
}

impl<W: WidgetCollection> Widget for Checkbox<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed_color = self.style.pressed_color.resolve(&stylesheet);
        let normal_color = self.style.normal_color.resolve(&stylesheet);

        scope.spawn_stream(self.state.stream(), {
            move |scope, state| {
                let color = if state { pressed_color } else { normal_color };

                scope.set(components::color(), color);
            }
        });

        let mut last_state = WatchState::new(self.state.stream());

        scope
            .set(focusable(), ())
            .on_event(on_mouse_input(), move |_, input| {
                if input.state == ElementState::Pressed {
                    if let Some(state) = last_state.get() {
                        self.state.send(!state)
                    }
                }
            });

        Stack::new(self.label)
            .with_style(ContainerStyle {
                background: Some(Background::new(normal_color)),
            })
            .with_horizontal_alignment(Align::Center)
            .with_vertical_alignment(Align::Center)
            .with_size_props(self.size)
            .mount(scope);
    }
}

/// A button that can only be set
pub struct Radio<W> {
    state: Box<dyn Send + Sync + StateDuplex<Item = bool>>,
    style: ButtonStyle,
    size: WidgetSize,
    label: W,
}

impl<W: WidgetCollection> Radio<W> {
    pub fn new(label: W, state: impl 'static + Send + Sync + StateDuplex<Item = bool>) -> Self {
        Self {
            state: Box::new(state),
            style: Default::default(),
            size: WidgetSize::default().with_padding(spacing_medium()),
            // .with_margin(spacing_medium()),
            // .with_min_size(Unit::px2(28.0, 28.0)),
            label,
        }
    }
}
impl Radio<Text> {
    pub fn label(
        label: impl Into<String>,
        state: impl 'static + Send + Sync + StateDuplex<Item = bool>,
    ) -> Self {
        Self::new(Text::new(label.into()), state)
    }
}

impl<T> SizeExt for Radio<T> {
    fn size_mut(&mut self) -> &mut WidgetSize {
        &mut self.size
    }
}

impl<W: WidgetCollection> Widget for Radio<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed_color = self.style.pressed_color.resolve(&stylesheet);
        let normal_color = self.style.normal_color.resolve(&stylesheet);

        scope.spawn_stream(self.state.stream(), {
            move |scope, state| {
                let color = if state { pressed_color } else { normal_color };

                scope.set(components::color(), color);
            }
        });

        scope
            .set(focusable(), ())
            .on_event(on_mouse_input(), move |_, input| {
                if input.state == ElementState::Pressed {
                    self.state.send(true)
                }
            });

        Stack::new(self.label)
            .with_style(ContainerStyle {
                background: Some(Background::new(normal_color)),
            })
            .with_horizontal_alignment(Align::Center)
            .with_vertical_alignment(Align::Center)
            .with_size_props(self.size)
            .mount(scope);
    }
}
