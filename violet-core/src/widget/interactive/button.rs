use flax::EntityRef;
use palette::Srgba;

use crate::{
    components::{color, LayoutAlignment},
    input::MouseInput,
    layout::Align,
    scope::ScopeRef,
    state::{StateDuplex, StateExt, StateStream, WatchState},
    style::*,
    tweens::tweens,
    unit::Unit,
    widget::{
        interactive::base::{ClickCallback, MouseInputCallback},
        label, ContainerStyle, Rectangle, Stack, Text,
    },
    Scope, Widget, WidgetCollection,
};

use super::base::{InteractiveWidget, TooltipOptions};

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

    fn resolve(&self, stylesheet: EntityRef<'_>) -> ColorPair<T::Value> {
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
    pub align: LayoutAlignment,
    pub size: WidgetSizeProps,
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
            ..Default::default()
        }
    }

    pub fn hidden() -> Self {
        ButtonStyle {
            normal: ColorPair::new(
                Srgba::new(0.0, 0.0, 0.0, 0.0),
                Srgba::new(0.0, 0.0, 0.0, 0.0),
            ),
            pressed: ColorPair::new(surface_pressed(), element_pressed()),
            hover: ColorPair::new(surface_hover(), element_hover()),
            size: WidgetSizeProps::default()
                .with_padding(spacing_small())
                .with_margin(spacing_small())
                .with_corner_radius(default_corner_radius()),
            ..Default::default()
        }
    }

    pub fn success() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive_success(), element_interactive_success()),
            pressed: ColorPair::new(surface_pressed_success(), element_pressed_success()),
            hover: ColorPair::new(surface_hover_success(), element_hover_success()),
            ..Default::default()
        }
    }

    pub fn disabled() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_disabled(), element_secondary()),
            pressed: ColorPair::new(surface_disabled(), element_secondary()),
            hover: ColorPair::new(surface_disabled(), element_secondary()),
            ..Default::default()
        }
    }

    pub fn danger() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive_danger(), element_interactive_danger()),
            pressed: ColorPair::new(surface_pressed_danger(), element_pressed_danger()),
            hover: ColorPair::new(surface_hover_danger(), element_hover_danger()),
            ..Default::default()
        }
    }

    pub fn warning() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive_warning(), element_interactive_warning()),
            pressed: ColorPair::new(surface_pressed_warning(), element_pressed_warning()),
            hover: ColorPair::new(surface_hover_warning(), element_hover_warning()),
            ..Default::default()
        }
    }

    pub fn accent() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive_accent(), element_interactive_accent()),
            pressed: ColorPair::new(surface_pressed_accent(), element_pressed_accent()),
            hover: ColorPair::new(surface_hover_accent(), element_hover_accent()),
            ..Default::default()
        }
    }

    pub fn selectable_entry() -> Self {
        ButtonStyle {
            normal: ColorPair::new(Srgba::new(0.0, 0.0, 0.0, 0.0), element_interactive()),
            pressed: ColorPair::new(surface_pressed(), element_pressed()),
            hover: ColorPair::new(surface_hover(), element_hover()),
            size: WidgetSizeProps::default()
                .with_padding(spacing_small())
                .with_margin(spacing_small())
                .with_corner_radius(default_corner_radius()),
            align: LayoutAlignment::left_center(),
            ..Default::default()
        }
    }

    pub fn radio() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive(), surface_interactive()),
            pressed: ColorPair::new(surface_interactive(), surface_pressed()),
            hover: ColorPair::new(surface_interactive(), surface_hover()),
            size: WidgetSizeProps::default()
                .with_padding(spacing_small())
                .with_margin(spacing_small())
                .with_corner_radius(Unit::rel(1.0))
                .with_min_size(Unit::px2(20.0, 20.0)),
            ..Default::default()
        }
    }

    pub fn checkbox() -> Self {
        ButtonStyle {
            normal: ColorPair::new(surface_interactive(), Srgba::new(0.0, 0.0, 0.0, 0.0)),
            pressed: ColorPair::new(surface_pressed(), element_primary()),
            hover: ColorPair::new(surface_interactive(), surface_hover()),
            size: WidgetSizeProps::default()
                // .with_padding(spacing_small())
                .with_margin(spacing_small())
                .with_corner_radius(default_corner_radius())
                .with_min_size(Unit::px2(20.0, 20.0))
                .with_max_size(Unit::px2(20.0, 20.0)),
            ..Default::default()
        }
    }

    pub fn with_align(mut self, align: LayoutAlignment) -> Self {
        self.align = align;
        self
    }
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: ColorPair::new(surface_interactive(), element_interactive()),
            pressed: ColorPair::new(surface_pressed(), element_pressed()),
            hover: ColorPair::new(surface_hover(), element_hover()),
            size: WidgetSizeProps::default()
                .with_padding(spacing_medium())
                .with_margin(spacing_medium())
                .with_corner_radius(default_corner_radius()),
            align: LayoutAlignment::new(Align::Center, Align::Center),
        }
    }
}

/// A button which invokes the callback when clicked
pub struct Button<W = Text> {
    on_click: ClickCallback,
    on_double_click: Option<ClickCallback>,
    tooltip: Option<TooltipOptions>,
    label: W,
    style: ButtonStyle,
    is_pressed: bool,
}

impl<W> Button<W> {
    pub fn new(label: W) -> Self
    where
        W: Widget,
    {
        Self {
            on_click: Box::new(|_| {}),
            label,
            style: Default::default(),
            is_pressed: false,
            tooltip: None,
            on_double_click: None,
        }
    }

    /// Handle the button press
    pub fn on_click(mut self, func: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>)) -> Self {
        self.on_click = Box::new(func);
        self
    }

    /// Handle the button press
    pub fn on_double_click(
        mut self,
        func: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>),
    ) -> Self {
        self.on_double_click = Some(Box::new(func));
        self
    }

    /// Handle the button press
    pub fn on_double_click_opt(mut self, func: Option<ClickCallback>) -> Self {
        self.on_double_click = func;
        self
    }

    pub fn with_tooltip_text(mut self, tooltip: impl Into<String>) -> Self {
        let tooltip = tooltip.into();
        self.tooltip = Some(TooltipOptions::new(move || label(&tooltip)));
        self
    }

    pub fn with_tooltip(mut self, tooltip: TooltipOptions) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    pub fn success(mut self) -> Self {
        self.style = ButtonStyle::success();
        self
    }

    pub fn disabled(mut self) -> Self {
        self.style = ButtonStyle::disabled();
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

impl StyleExt for Radio {
    type Style = ButtonStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl StyleExt for Checkbox {
    type Style = ButtonStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl<W> SizeExt for Button<W> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.style.size
    }
}

impl<W: Widget> Widget for Button<W> {
    fn mount(mut self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed = self.style.pressed.resolve(stylesheet);
        let normal = self.style.normal.resolve(stylesheet);
        let _hover = self.style.hover.resolve(stylesheet);

        let _content = scope.attach(self.label);

        let inner = Stack::new(())
            .with_background(Background::new(normal.surface))
            .with_horizontal_alignment(self.style.align.horizontal)
            .with_vertical_alignment(self.style.align.vertical);

        scope.set_default(tweens());

        InteractiveWidget::new(inner)
            .with_size_props(self.style.size)
            .on_click(move |scope| (self.on_click)(scope))
            .on_double_click_opt(self.on_double_click)
            .on_pointer_press(move |scope, state| {
                // let current_color = scope.get(color());
                let new_color = if state.is_pressed() { pressed } else { normal };

                // scope
                //     .world()
                //     .entity(content)
                //     .unwrap()
                //     .update_dedup(components::color(), color.element);

                // TODO: support tween for Srgba
                // scope.add_tween(color(), Tweener::linear(current_color, color.surface, 0.2));
                scope.update_dedup(color(), new_color.surface);
            })
            .with_tooltip_opt(self.tooltip)
            .mount(scope);
    }
}

pub struct Checkbox {
    state: Box<dyn Send + Sync + StateDuplex<Item = bool>>,
    tooltip: Option<TooltipOptions>,
    style: ButtonStyle,
}

impl Checkbox {
    pub fn new(state: impl 'static + Send + Sync + StateDuplex<Item = bool>) -> Self {
        Self {
            state: Box::new(state),
            style: ButtonStyle::checkbox(),
            tooltip: None,
        }
    }

    pub fn with_tooltip_text(mut self, tooltip: impl Into<String>) -> Self {
        let tooltip = tooltip.into();
        self.tooltip = Some(TooltipOptions::new(move || label(&tooltip)));
        self
    }

    pub fn with_tooltip(mut self, tooltip: TooltipOptions) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
}

impl Widget for Checkbox {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed = self.style.pressed.resolve(stylesheet);
        let normal = self.style.normal.resolve(stylesheet);
        let _hover = self.style.hover.resolve(stylesheet);
        let check_icon = stylesheet.get_clone(icon_check()).unwrap_or_default();

        let content = scope.attach(label(check_icon).with_color(normal.element));

        scope.spawn_stream(self.state.stream(), {
            move |scope, state| {
                let new_color = if state { pressed } else { normal };

                scope
                    .world()
                    .entity(content)
                    .unwrap()
                    .update_dedup(color(), new_color.element)
                    .unwrap();

                scope.update_dedup(color(), new_color.surface).unwrap();
            }
        });

        let mut last_state = WatchState::new(self.state.stream());

        let inner = Stack::new(())
            .with_alignment(LayoutAlignment::center())
            .with_style(ContainerStyle {
                background: Some(Background::new(normal.surface)),
            })
            .with_horizontal_alignment(self.style.align.horizontal)
            .with_vertical_alignment(self.style.align.vertical)
            .with_size_props(self.style.size);

        InteractiveWidget::new(inner)
            .on_pointer_press(move |_, state| {
                if state.is_pressed() {
                    if let Some(state) = last_state.get() {
                        self.state.send(!state)
                    }
                }
            })
            .mount(scope);
    }
}

/// A button that can only be set
pub struct Radio {
    state: Box<dyn Send + Sync + StateDuplex<Item = bool>>,
    tooltip: Option<TooltipOptions>,
    style: ButtonStyle,
}

impl Radio {
    pub fn new(state: impl 'static + Send + Sync + StateDuplex<Item = bool>) -> Self {
        Self {
            state: Box::new(state),
            style: ButtonStyle::radio(),
            tooltip: None,
        }
    }

    pub fn new_value<T: 'static + Send + Sync + Copy + PartialEq>(
        state: impl 'static + Send + Sync + StateDuplex<Item = T>,
        index: T,
    ) -> Self {
        Self::new(state.map_value(move |v| v == index, move |_| index))
    }

    pub fn with_tooltip(mut self, tooltip: TooltipOptions) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
}

impl SizeExt for Radio {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.style.size
    }
}

impl Widget for Radio {
    fn mount(self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed = self.style.pressed.resolve(stylesheet);
        let normal = self.style.normal.resolve(stylesheet);
        let _hover = self.style.hover.resolve(stylesheet);

        let content =
            scope.attach(Rectangle::new(normal.element).with_corner_radius(Unit::rel(1.0)));

        scope.spawn_stream(self.state.stream(), {
            move |scope, state| {
                let new_color = if state { pressed } else { normal };

                scope
                    .world()
                    .entity(content)
                    .unwrap()
                    .update_dedup(color(), new_color.element);

                scope.set(color(), new_color.surface);
            }
        });

        let inner = Stack::new(())
            .with_background(Background::new(normal.surface))
            .with_horizontal_alignment(Align::Center)
            .with_vertical_alignment(Align::Center);

        scope.set_default(tweens());

        InteractiveWidget::new(inner)
            .with_size_props(self.style.size)
            .on_pointer_press(move |_, state| {
                if state.is_pressed() {
                    self.state.send(true)
                }
            })
            .with_tooltip_opt(self.tooltip)
            .mount(scope);
    }
}

/// A button that can only be set
pub struct Selectable<W> {
    state: Box<dyn Send + Sync + StateDuplex<Item = bool>>,
    tooltip: Option<TooltipOptions>,
    on_double_click: Option<ClickCallback>,
    on_mouse_input: Option<MouseInputCallback>,
    style: ButtonStyle,
    label: W,
}

impl<W: WidgetCollection> Selectable<W> {
    pub fn new(label: W, state: impl 'static + Send + Sync + StateDuplex<Item = bool>) -> Self {
        Self {
            state: Box::new(state),
            on_double_click: None,
            style: ButtonStyle::default(),
            label,
            tooltip: None,
            on_mouse_input: None,
        }
    }

    pub fn on_mouse_input(
        mut self,
        func: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>, MouseInput) -> Option<MouseInput>,
    ) -> Self {
        self.on_mouse_input = Some(Box::new(func));
        self
    }

    pub fn selectable_entry(
        label: W,
        state: impl 'static + Send + Sync + StateDuplex<Item = bool>,
    ) -> Self {
        Self {
            state: Box::new(state),
            on_double_click: None,
            style: ButtonStyle::selectable_entry(),
            label,
            tooltip: None,
            on_mouse_input: None,
        }
    }

    pub fn new_value<T: 'static + Send + Sync + Clone + PartialEq>(
        label: W,
        state: impl 'static + Send + Sync + StateDuplex<Item = T>,
        index: T,
    ) -> Self {
        let index2 = index.clone();
        Self::new(
            label,
            state.map_value(move |v| v == index2, move |_| index.clone()),
        )
    }

    pub fn on_double_click(
        mut self,
        func: impl 'static + Send + Sync + FnMut(&ScopeRef<'_>),
    ) -> Self {
        self.on_double_click = Some(Box::new(func));
        self
    }

    pub fn with_tooltip(mut self, tooltip: TooltipOptions) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
}

impl Selectable<Text> {
    pub fn label(
        label: impl Into<String>,
        state: impl 'static + Send + Sync + StateDuplex<Item = bool>,
    ) -> Self {
        Self::new(Text::new(label.into()), state)
    }
}

impl<T> SizeExt for Selectable<T> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.style.size
    }
}

impl<W: Widget> Widget for Selectable<W> {
    fn mount(mut self, scope: &mut Scope<'_>) {
        let stylesheet = scope.stylesheet();

        let pressed = self.style.pressed.resolve(stylesheet);
        let normal = self.style.normal.resolve(stylesheet);
        let _hover = self.style.hover.resolve(stylesheet);

        let content = scope.attach(self.label);

        scope.spawn_stream(self.state.stream(), {
            move |scope, state| {
                let new_color = if state { pressed } else { normal };

                scope
                    .world()
                    .entity(content)
                    .unwrap()
                    .update_dedup(color(), new_color.element);

                scope.set(color(), new_color.surface);
            }
        });

        let inner = Stack::new(())
            .with_background(Background::new(normal.surface))
            .with_horizontal_alignment(self.style.align.horizontal)
            .with_vertical_alignment(self.style.align.vertical);

        scope.set_default(tweens());

        InteractiveWidget::new(inner)
            .with_size_props(self.style.size)
            .on_mouse_input(move |scope, input| {
                if let Some(mouse_input) = &mut self.on_mouse_input {
                    if (mouse_input)(scope, input).is_none() {
                        return None;
                    }
                }

                if input.state.is_pressed() {
                    self.state.send(true);
                    return None;
                }

                Some(input)
            })
            .on_double_click_opt(self.on_double_click)
            .with_tooltip_opt(self.tooltip)
            .mount(scope);
    }
}

impl<W> StyleExt for Selectable<W> {
    type Style = ButtonStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}
