use flax::EntityRef;
use palette::Srgba;
use winit::event::{ElementState, MouseButton};

use crate::{
    components::{self},
    input::{interactive, on_mouse_input},
    layout::Align,
    scope::ScopeRef,
    state::{StateDuplex, StateStream, WatchState},
    style::*,
    unit::Unit,
    widget::{ContainerStyle, Stack, Text},
    Scope, Widget, WidgetCollection,
};

type ButtonCallback = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>, winit::event::MouseButton)>;
type ButtonClickCallback = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>)>;

#[derive(Debug, Copy, Clone)]
pub struct ColorPair<T> {
    pub surface: T,
    pub element: T,
}

impl<T> ColorPair<T> {
    pub fn new(surface: impl Into<T>, element: impl Into<T>) -> Self {
        Self {
            surface: surface.into(),
            element: element.into(),
        }
    }
}

impl<T: ResolvableStyle> ResolvableStyle for ColorPair<T> {
    type Value = ColorPair<T::Value>;

    fn resolve(self, stylesheet: &EntityRef<'_>) -> ColorPair<T::Value> {
        ColorPair {
            surface: self.surface.resolve(stylesheet),
            element: self.element.resolve(stylesheet),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ButtonStyle {
    pub normal: ColorPair<ValueOrRef<Srgba>>,
    pub pressed: ColorPair<ValueOrRef<Srgba>>,
    pub hover: ColorPair<ValueOrRef<Srgba>>,
}

impl ButtonStyle {
    pub fn new(
        normal: ColorPair<ValueOrRef<Srgba>>,
        pressed: ColorPair<ValueOrRef<Srgba>>,
        hover: ColorPair<ValueOrRef<Srgba>>,
    ) -> Self {
        Self {
            normal,
            pressed,
            hover,
        }
    }

    pub fn success() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive_success(), element_interactive_success()),
            pressed: ColorPair::new(surface_pressed_success(), element_pressed_success()),
            hover: ColorPair::new(surface_hover_success(), element_hover_success()),
        }
    }

    pub fn danger() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive_danger(), element_interactive_danger()),
            pressed: ColorPair::new(surface_pressed_danger(), element_pressed_danger()),
            hover: ColorPair::new(surface_hover_danger(), element_hover_danger()),
        }
    }

    pub fn warning() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive_warning(), element_interactive_warning()),
            pressed: ColorPair::new(surface_pressed_warning(), element_pressed_warning()),
            hover: ColorPair::new(surface_hover_warning(), element_hover_warning()),
        }
    }

    pub fn accent() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive_accent(), element_interactive_accent()),
            pressed: ColorPair::new(surface_pressed_accent(), element_pressed_accent()),
            hover: ColorPair::new(surface_hover_accent(), element_hover_accent()),
        }
    }
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: ColorPair::new(surface_interactive(), element_interactive()),
            pressed: ColorPair::new(surface_pressed(), element_pressed()),
            hover: ColorPair::new(surface_hover(), element_hover()),
        }
    }
}

/// A button which invokes the callback when clicked
pub struct Button<W = Text> {
    on_press: ButtonCallback,
    on_click: ButtonClickCallback,
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
            on_click: Box::new(|_| {}),
            label,
            style: Default::default(),
            size: WidgetSize::default()
                .with_padding(spacing_medium())
                .with_margin(spacing_medium())
                .with_corner_radius(default_corner_radius()),
            // .with_min_size(Unit::px2(28.0, 28.0)),
            is_pressed: false,
        }
    }

    /// Handle the button press
    pub fn on_mousebutton_down(
        mut self,
        on_press: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>, MouseButton),
    ) -> Self {
        self.on_press = Box::new(on_press);
        self
    }

    /// Handle the button press
    pub fn on_click(mut self, on_press: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>)) -> Self {
        self.on_click = Box::new(on_press);
        self
    }

    pub fn success(mut self) -> Self {
        self.style = ButtonStyle::success();
        self
    }

    pub fn danger(mut self) -> Self {
        self.style = ButtonStyle::danger();
        self
    }

    pub fn warning(mut self) -> Self {
        self.style = ButtonStyle::warning();
        self
    }

    pub fn accent(mut self) -> Self {
        self.style = ButtonStyle::accent();

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

impl<W> StyleExt for Radio<W> {
    type Style = ButtonStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl<W> StyleExt for Checkbox<W> {
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

        let pressed = self.style.pressed.resolve(&stylesheet);
        let normal = self.style.normal.resolve(&stylesheet);
        let _hover = self.style.hover.resolve(&stylesheet);

        let mut is_pressed = false;

        let content = scope.attach(self.label);

        scope
            .set(interactive(), ())
            .on_event(on_mouse_input(), move |scope, input| {
                let color = if input.state.is_pressed() {
                    pressed
                } else {
                    normal
                };

                scope
                    .world()
                    .entity(content)
                    .unwrap()
                    .update_dedup(components::color(), color.element);

                scope.update_dedup(components::color(), color.surface);

                if input.state == ElementState::Pressed {
                    is_pressed = true;
                    (self.on_press)(scope, input.button);
                } else if is_pressed {
                    is_pressed = false;
                    (self.on_click)(scope);
                }

                None
            });

        Stack::new(())
            .with_style(ContainerStyle {
                background: Some(Background::new(normal.surface)),
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
                .with_corner_radius(default_corner_radius())
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

impl<W: Widget> Widget for Checkbox<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed = self.style.pressed.resolve(&stylesheet);
        let normal = self.style.normal.resolve(&stylesheet);
        let _hover = self.style.hover.resolve(&stylesheet);

        let content = scope.attach(self.label);

        scope.spawn_stream(self.state.stream(), {
            move |scope, state| {
                let color = if state { pressed } else { normal };

                scope
                    .world()
                    .entity(content)
                    .unwrap()
                    .update_dedup(components::color(), color.element);

                scope.set(components::color(), color.surface);
            }
        });

        let mut last_state = WatchState::new(self.state.stream());

        scope
            .set(interactive(), ())
            .on_event(on_mouse_input(), move |_, input| {
                if input.state == ElementState::Pressed {
                    if let Some(state) = last_state.get() {
                        self.state.send(!state)
                    }
                }

                None
            });

        Stack::new(())
            .with_style(ContainerStyle {
                background: Some(Background::new(normal.surface)),
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
            size: WidgetSize::default()
                .with_padding(spacing_medium())
                .with_margin(spacing_medium())
                .with_corner_radius(default_corner_radius())
                .with_min_size(Unit::px2(28.0, 28.0)),
            label,
        }
    }

    pub fn new_indexed(
        label: W,
        state: impl 'static + Send + Sync + StateDuplex<Item = usize>,
        index: usize,
    ) -> Self {
        Self::new(label, state.map_value(move |v| v == index, move |_| index))
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

impl<W: Widget> Widget for Radio<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed = self.style.pressed.resolve(&stylesheet);
        let normal = self.style.normal.resolve(&stylesheet);
        let _hover = self.style.hover.resolve(&stylesheet);

        let content = scope.attach(self.label);

        scope.spawn_stream(self.state.stream(), {
            move |scope, state| {
                let color = if state { pressed } else { normal };

                scope
                    .world()
                    .entity(content)
                    .unwrap()
                    .update_dedup(components::color(), color.element);
                scope.set(components::color(), color.surface);
            }
        });

        scope
            .set(interactive(), ())
            .on_event(on_mouse_input(), move |_, input| {
                if input.state == ElementState::Pressed {
                    self.state.send(true)
                }

                None
            });

        Stack::new(())
            .with_style(ContainerStyle {
                background: Some(Background::new(normal.surface)),
            })
            .with_horizontal_alignment(Align::Center)
            .with_vertical_alignment(Align::Center)
            .with_size_props(self.size)
            .mount(scope);
    }
}
