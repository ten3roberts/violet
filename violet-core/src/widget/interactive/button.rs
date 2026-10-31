use std::cell::RefCell;

use flax::Component;
use glam::{vec2, Vec2};
use palette::Srgba;

use crate::{
    components::LayoutAlignment,
    icon::IconSet,
    input::MouseInput,
    layout::Align,
    scope::ScopeRef,
    state::{StateDuplex, StateExt, StateStream, WatchState},
    stored::WeakHandle,
    style::*,
    unit::Unit,
    widget::{
        icon,
        interactive::base::{ClickCallback, MouseInputCallback},
        label, ContainerStyle, Rectangle, Stack, Text,
    },
    Edges, Scope, Widget, WidgetCollection,
};

use super::base::{InteractionState, InteractiveWidget, TooltipOptions};

/// Looks of a button-like widget per interaction state, unselected and selected
#[derive(Debug, Copy, Clone)]
pub struct ButtonStyle {
    /// Looks while not selected; the only ones a plain button shows
    pub unselected: StateStyles,
    /// Looks while selected, such as a checked checkbox or the chosen entry of a list
    pub selected: StateStyles,
    pub align: LayoutAlignment,
    pub size: WidgetSizeProps,
}

const TRANSPARENT: Srgba = Srgba::new(0.0, 0.0, 0.0, 0.0);

/// A control raised off the surface
fn raised() -> Decoration {
    Decoration::default().with_shadow(shadow_raised())
}

/// A control sunk into the surface while pressed
fn sunk() -> Decoration {
    Decoration::default().with_inset_shadow(inset_shadow_pressed())
}

/// A control carved into the surface, such as a checkbox
fn well() -> Decoration {
    Decoration::default().with_inset_shadow(inset_shadow_well())
}

/// How far a raised control drops while pressed
const PRESS_DROP: Vec2 = vec2(0.0, 1.0);

/// Raised while normal and hovered, sunk and dropped while pressed
fn raised_states(normal: StateStyle, hover: StateStyle, pressed: StateStyle) -> StateStyles {
    StateStyles::new(
        normal.with_decoration(raised()),
        hover.with_decoration(raised()),
        pressed.with_decoration(sunk()).with_translation(PRESS_DROP),
    )
}

/// The selected looks of flat widgets that only sink into a pressed colour
fn flat_selected() -> StateStyles {
    StateStyles::new(
        StateStyle::new(surface_pressed(), element_pressed()),
        StateStyle::new(surface_hover_accent(), element_pressed()),
        StateStyle::new(surface_pressed_accent(), element_pressed()),
    )
}

/// Plain button looks of one colour role
fn role(normal: (StateStyle, StateStyle, StateStyle)) -> (StateStyles, StateStyles) {
    let unselected = raised_states(normal.0, normal.1, normal.2);
    (unselected, StateStyles::uniform(unselected.pressed))
}

fn small_size() -> WidgetSizeProps {
    WidgetSizeProps::default()
        .with_padding(spacing_small())
        .with_margin(spacing_small())
        .with_corner_radius(default_corner_radius())
}

impl ButtonStyle {
    pub fn new(unselected: StateStyles, selected: StateStyles) -> Self {
        Self {
            unselected,
            selected,
            ..Default::default()
        }
    }

    /// A button that is invisible until hovered or selected but still interactive
    pub fn hidden() -> Self {
        ButtonStyle {
            unselected: StateStyles::new(
                StateStyle::new(TRANSPARENT, TRANSPARENT),
                StateStyle::new(surface_hover(), element_hover()),
                StateStyle::new(surface_pressed(), element_pressed()),
            ),
            selected: flat_selected(),
            size: small_size(),
            ..Default::default()
        }
    }

    /// Similar to hidden but with a muted background
    pub fn muted() -> Self {
        ButtonStyle {
            unselected: StateStyles::new(
                StateStyle::new(surface_secondary(), element_primary()),
                StateStyle::new(surface_hover(), element_hover()),
                StateStyle::new(surface_tertiary(), element_primary()),
            ),
            selected: flat_selected(),
            size: small_size(),
            ..Default::default()
        }
    }

    pub fn success() -> Self {
        let (unselected, selected) = role((
            StateStyle::new(surface_interactive_success(), element_interactive_success()),
            StateStyle::new(surface_hover_success(), element_hover_success()),
            StateStyle::new(surface_pressed_success(), element_pressed_success()),
        ));
        ButtonStyle::new(unselected, selected)
    }

    /// A button that does not react to the pointer
    pub fn disabled() -> Self {
        let style = StateStyles::uniform(StateStyle::new(surface_disabled(), element_secondary()));
        ButtonStyle::new(style, style)
    }

    pub fn danger() -> Self {
        let (unselected, selected) = role((
            StateStyle::new(surface_interactive_danger(), element_interactive_danger()),
            StateStyle::new(surface_hover_danger(), element_hover_danger()),
            StateStyle::new(surface_pressed_danger(), element_pressed_danger()),
        ));
        ButtonStyle::new(unselected, selected)
    }

    pub fn warning() -> Self {
        let (unselected, selected) = role((
            StateStyle::new(surface_interactive_warning(), element_interactive_warning()),
            StateStyle::new(surface_hover_warning(), element_hover_warning()),
            StateStyle::new(surface_pressed_warning(), element_pressed_warning()),
        ));
        ButtonStyle::new(unselected, selected)
    }

    pub fn accent() -> Self {
        let (unselected, selected) = role((
            StateStyle::new(surface_interactive_accent(), element_interactive_accent()),
            StateStyle::new(surface_hover_accent(), element_hover_accent()),
            StateStyle::new(surface_pressed_accent(), element_pressed_accent()),
        ));
        ButtonStyle::new(unselected, selected)
    }

    pub fn selectable_entry() -> Self {
        ButtonStyle {
            unselected: StateStyles::new(
                StateStyle::new(TRANSPARENT, element_interactive()),
                StateStyle::new(surface_hover(), element_hover()),
                StateStyle::new(surface_pressed(), element_pressed()),
            ),
            selected: flat_selected(),
            size: small_size(),
            align: LayoutAlignment::left_center(),
        }
    }

    /// A round well whose dot shows the selection
    pub fn radio() -> Self {
        let state = |surface: Component<Srgba>, dot: Component<Srgba>| {
            StateStyle::new(surface, dot).with_decoration(well())
        };
        ButtonStyle {
            unselected: StateStyles::new(
                state(surface_interactive(), surface_interactive()),
                state(surface_hover(), surface_hover()),
                state(surface_hover(), surface_hover()),
            ),
            selected: StateStyles::new(
                state(surface_interactive(), surface_pressed()),
                state(surface_hover(), surface_pressed()),
                state(surface_hover(), surface_pressed_accent()),
            ),
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
            unselected: StateStyles::new(
                StateStyle::new(surface_interactive(), TRANSPARENT).with_decoration(well()),
                StateStyle::new(surface_hover(), TRANSPARENT).with_decoration(well()),
                StateStyle::new(surface_hover(), TRANSPARENT).with_decoration(well()),
            ),
            selected: StateStyles::new(
                StateStyle::new(surface_pressed(), element_primary()).with_decoration(well()),
                StateStyle::new(surface_hover_accent(), element_primary()).with_decoration(well()),
                StateStyle::new(surface_pressed_accent(), element_primary())
                    .with_decoration(well()),
            ),
            size: WidgetSizeProps::default()
                .with_margin(spacing_small())
                .with_corner_radius(default_corner_radius())
                .with_min_size(Unit::px2(20.0, 20.0))
                .with_max_size(Unit::px2(20.0, 20.0)),
            ..Default::default()
        }
    }

    /// A pill-shaped well whose element colours the knob of a [`Switch`](super::Switch)
    pub fn switch() -> Self {
        ButtonStyle {
            unselected: StateStyles::new(
                StateStyle::new(surface_interactive(), element_primary()).with_decoration(well()),
                StateStyle::new(surface_hover(), element_primary()).with_decoration(well()),
                StateStyle::new(surface_hover(), element_primary()).with_decoration(well()),
            ),
            selected: StateStyles::new(
                StateStyle::new(surface_pressed(), element_primary()).with_decoration(well()),
                StateStyle::new(surface_hover_accent(), element_primary()).with_decoration(well()),
                StateStyle::new(surface_pressed_accent(), element_primary())
                    .with_decoration(well()),
            ),
            size: WidgetSizeProps::default()
                .with_min_size(Unit::px2(40.0, 24.0))
                .with_max_size(Unit::px2(40.0, 24.0))
                .with_padding(Edges::even(2.0))
                .with_margin(spacing_small())
                .with_corner_radius(Unit::rel(1.0)),
            align: LayoutAlignment::left_center(),
        }
    }

    /// One option of a [`SegmentedControl`](super::SegmentedControl), raised while selected
    pub fn segment() -> Self {
        ButtonStyle {
            unselected: StateStyles::new(
                StateStyle::new(TRANSPARENT, element_secondary()),
                StateStyle::new(surface_hover(), element_primary()),
                StateStyle::new(surface_tertiary(), element_primary()),
            ),
            selected: StateStyles::new(
                StateStyle::new(surface_interactive(), element_primary()).with_decoration(raised()),
                StateStyle::new(surface_hover(), element_primary()).with_decoration(raised()),
                StateStyle::new(surface_hover(), element_primary()).with_decoration(raised()),
            ),
            size: WidgetSizeProps::default()
                .with_padding(spacing_small())
                .with_margin(Edges::even(0.0))
                .with_corner_radius(default_corner_radius()),
            align: LayoutAlignment::center(),
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
            unselected: raised_states(
                StateStyle::new(surface_interactive(), element_interactive()),
                StateStyle::new(surface_hover(), element_hover()),
                StateStyle::new(surface_pressed(), element_pressed()),
            ),
            selected: StateStyles::new(
                StateStyle::new(surface_pressed(), element_pressed()),
                StateStyle::new(surface_hover_accent(), element_pressed()),
                StateStyle::new(surface_pressed_accent(), element_pressed()),
            )
            .map(|v| v.with_decoration(sunk()).with_translation(PRESS_DROP)),
            size: WidgetSizeProps::default()
                .with_padding(spacing_medium())
                .with_margin(default_separation())
                .with_corner_radius(default_corner_radius()),
            align: LayoutAlignment::new(Align::Center, Align::Center),
        }
    }
}

/// Interaction state handler of a button-like widget's [`InteractiveWidget`], driving its look
pub(super) fn interaction_handler(
    driver: WeakHandle<RefCell<StyleDriver>>,
) -> impl FnMut(&ScopeRef<'_>, InteractionState) + Send + Sync + 'static {
    move |scope, state| {
        scope
            .read(driver)
            .borrow_mut()
            .set_interaction(scope.entity(), state)
    }
}

/// Mounts the [`StyleDriver`] of a button-like widget, returning it with the background to draw
/// the widget's surface, in the colour the driver starts with
pub(super) fn mount_driver(
    scope: &mut Scope<'_>,
    style: &ButtonStyle,
    element_target: Option<flax::Entity>,
    is_selected: bool,
) -> (WeakHandle<RefCell<StyleDriver>>, Background) {
    let driver = StyleDriver::mount(
        scope,
        &style.unselected,
        &style.selected,
        element_target,
        is_selected,
    );
    let surface = scope.read(&driver).borrow().target().surface;
    (driver, Background::new(surface))
}

/// Shows the selected look whenever `selected` changes, at once for its first value
pub(super) fn follow_selection(
    scope: &mut Scope<'_>,
    driver: WeakHandle<RefCell<StyleDriver>>,
    selected: impl 'static + futures::Stream<Item = bool>,
) {
    let mut first = true;
    scope.spawn_stream(selected, move |scope, selected| {
        scope
            .read(&driver)
            .borrow_mut()
            .set_selected(&scope.entity(), selected, !first);
        first = false;
    });
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
        self.tooltip = Some(TooltipOptions::new(move |_| label(&tooltip)));
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

impl<W> StyleExt for Checkbox<W> {
    type Style = ButtonStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

impl<W> SizeExt for Checkbox<W> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.style.size
    }
}

impl<W> SizeExt for Button<W> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.style.size
    }
}

impl<W: Widget> Widget for Button<W> {
    fn mount(mut self, scope: &mut Scope<'_>) {
        let label = scope.attach(self.label);
        let (driver, background) = mount_driver(scope, &self.style, Some(label), self.is_pressed);

        let inner = Stack::new(())
            .with_background(background)
            .with_horizontal_alignment(self.style.align.horizontal)
            .with_vertical_alignment(self.style.align.vertical);

        InteractiveWidget::new(inner)
            .with_size_props(self.style.size)
            .on_click(move |scope| (self.on_click)(scope))
            .on_double_click_opt(self.on_double_click)
            .on_interaction_state(interaction_handler(driver))
            .with_tooltip_opt(self.tooltip)
            .mount(scope);
    }
}

pub struct CheckboxTick {}

impl Widget for CheckboxTick {
    fn mount(self, scope: &mut Scope<'_>) {
        let check_icon = scope
            .stylesheet()
            .get_clone(icon_check())
            .unwrap_or_else(|_| IconSet::default().check);
        icon(check_icon).mount(scope);
    }
}

pub struct Checkbox<W = CheckboxTick> {
    label: W,
    state: Box<dyn Send + Sync + StateDuplex<Item = bool>>,
    tooltip: Option<TooltipOptions>,
    style: ButtonStyle,
}

impl Checkbox<CheckboxTick> {
    pub fn new(state: impl 'static + Send + Sync + StateDuplex<Item = bool>) -> Self {
        Self {
            label: CheckboxTick {},
            state: Box::new(state),
            style: ButtonStyle::checkbox(),
            tooltip: None,
        }
    }
}

impl<W> Checkbox<W> {
    pub fn with_label(
        label: W,
        state: impl 'static + Send + Sync + StateDuplex<Item = bool>,
    ) -> Self {
        Self {
            label,
            state: Box::new(state),
            style: ButtonStyle::checkbox(),
            tooltip: None,
        }
    }

    pub fn with_tooltip_text(mut self, tooltip: impl Into<String>) -> Self {
        let tooltip = tooltip.into();
        self.tooltip = Some(TooltipOptions::new(move |_| label(&tooltip)));
        self
    }

    pub fn with_tooltip(mut self, tooltip: TooltipOptions) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
}

impl<W: Widget> Widget for Checkbox<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let content = scope.attach(self.label);
        let (driver, background) = mount_driver(scope, &self.style, Some(content), false);
        follow_selection(scope, driver, self.state.stream());

        let mut last_state = WatchState::new(self.state.stream());

        let inner = Stack::new(())
            .with_alignment(LayoutAlignment::center())
            .with_style(ContainerStyle {
                background: Some(background),
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
            .on_interaction_state(interaction_handler(driver))
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
        let content =
            scope.attach(Rectangle::new(Srgba::default()).with_corner_radius(Unit::rel(1.0)));
        let (driver, background) = mount_driver(scope, &self.style, Some(content), false);
        follow_selection(scope, driver, self.state.stream());

        let inner = Stack::new(())
            .with_background(background)
            .with_horizontal_alignment(Align::Center)
            .with_vertical_alignment(Align::Center);

        InteractiveWidget::new(inner)
            .with_size_props(self.style.size)
            .on_pointer_press(move |_, state| {
                if state.is_pressed() {
                    self.state.send(true)
                }
            })
            .on_interaction_state(interaction_handler(driver))
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
        value: T,
    ) -> Self {
        let index2 = value.clone();
        Self::new(
            label,
            state.map_value(move |v| v == index2, move |_| value.clone()),
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
        let content = scope.attach(self.label);
        let (driver, background) = mount_driver(scope, &self.style, Some(content), false);
        follow_selection(scope, driver, self.state.stream());

        let inner = Stack::new(())
            .with_background(background)
            .with_horizontal_alignment(self.style.align.horizontal)
            .with_vertical_alignment(self.style.align.vertical);

        InteractiveWidget::new(inner)
            .with_size_props(self.style.size)
            .on_generic_mouse_input(move |scope, input| {
                if let Some(mouse_input) = &mut self.on_mouse_input {
                    if (mouse_input)(scope, input).is_none() {
                        return None;
                    }
                }

                Some(input)
            })
            .on_click(move |_| {
                self.state.send(true);
            })
            .on_double_click_opt(self.on_double_click)
            .on_interaction_state(interaction_handler(driver))
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use flax::{Entity, Schedule, World};
    use futures_signals::signal::Mutable;
    use glam::{vec2, Vec2};
    use palette::Srgba;
    use winit::{
        event::{ElementState, MouseButton},
        keyboard::ModifiersState,
    };

    use crate::{
        assets::AssetCache,
        components::{app_instance, children, color, delta_time, translation},
        executor::Executor,
        input::{on_cursor_hover, on_mouse_input, CursorMove, CursorOver, HoverState, MouseInput},
        style::*,
        tweens::update_tweens_system,
        Frame, Scope, ScopeRef, Widget,
    };

    use super::{Button, Checkbox};

    struct Harness {
        /// Runs the widget's streams; kept alive with the frame
        _ex: Executor,
        frame: Frame,
        sheet: Entity,
        widget: Entity,
    }

    impl Harness {
        fn new(widget: impl 'static + Widget) -> Self {
            let mut ex = Executor::new();
            let mut frame = Frame::new(ex.spawner(), AssetCache::new(), World::new());
            let sheet = StylesheetOptions::new().build().spawn(&mut frame.world);

            let root = frame.new_root(move |scope: &mut Scope| {
                scope.set(stylesheet(sheet), ());
                scope.attach(widget);
            });
            ex.tick(&mut frame);

            let widget = frame.world.entity(root).unwrap().get(children()).unwrap()[0];

            Self {
                _ex: ex,
                frame,
                sheet,
                widget,
            }
        }

        fn sheet_color(&self, component: flax::Component<Srgba>) -> Srgba {
            self.frame
                .world
                .entity(self.sheet)
                .unwrap()
                .get_copy(component)
                .unwrap()
        }

        fn color(&self) -> Srgba {
            self.frame
                .world
                .entity(self.widget)
                .unwrap()
                .get_copy(color())
                .unwrap()
        }

        fn translation(&self) -> Vec2 {
            self.frame
                .world
                .entity(self.widget)
                .unwrap()
                .get_copy(translation())
                .unwrap()
        }

        fn hover(&self, state: HoverState) {
            let entity = self.frame.world.entity(self.widget).unwrap();
            let mut handler = entity.get_mut(on_cursor_hover()).unwrap();
            handler(
                &ScopeRef::new(&self.frame, entity),
                CursorOver {
                    state,
                    absolute_pos: Vec2::ZERO,
                    local_pos: Vec2::ZERO,
                },
            );
        }

        fn press(&self, state: ElementState) {
            let entity = self.frame.world.entity(self.widget).unwrap();
            let mut handler = entity.get_mut(on_mouse_input()).unwrap();
            handler(
                &ScopeRef::new(&self.frame, entity),
                MouseInput {
                    modifiers: ModifiersState::empty(),
                    state,
                    cursor: CursorMove {
                        modifiers: ModifiersState::empty(),
                        absolute_pos: Vec2::ZERO,
                        local_pos: Vec2::ZERO,
                    },
                    button: MouseButton::Left,
                },
            );
        }

        /// Runs the tween system past the end of any transition
        fn settle(&mut self) {
            self.frame
                .world
                .set(app_instance(), delta_time(), Duration::from_millis(200))
                .unwrap();
            Schedule::builder()
                .with_system(update_tweens_system())
                .build()
                .execute_seq(&mut self.frame.world)
                .unwrap();
        }
    }

    fn assert_close(a: Srgba, b: Srgba) {
        let diff = [
            a.red - b.red,
            a.green - b.green,
            a.blue - b.blue,
            a.alpha - b.alpha,
        ];
        assert!(
            diff.iter().all(|v| v.abs() < 1e-3),
            "{a:?} is not close to {b:?}"
        );
    }

    #[test]
    fn button_hover_and_press() {
        let mut h = Harness::new(Button::label("Press"));

        h.hover(HoverState::Entered);
        h.settle();
        assert_close(h.color(), h.sheet_color(surface_hover()));

        // The label's colour is written to its own entity while the tween system runs
        let label = h.frame.world.entity(h.widget).unwrap().get(children()).unwrap()[0];
        let label_color = h.frame.world.get(label, color()).map(|v| *v).unwrap();
        assert_close(label_color, h.sheet_color(element_hover()));

        h.press(ElementState::Pressed);
        h.settle();
        assert_close(h.color(), h.sheet_color(surface_pressed()));
        assert_eq!(h.translation(), vec2(0.0, 1.0));

        h.press(ElementState::Released);
        h.hover(HoverState::Exited);
        h.settle();
        assert_close(h.color(), h.sheet_color(surface_interactive()));
        assert_eq!(h.translation(), Vec2::ZERO);
    }

    #[test]
    fn checkbox_selected_hover() {
        let mut h = Harness::new(Checkbox::new(Mutable::new(true)));

        // The first selection shows at once, without a transition from the unselected look
        assert_close(h.color(), h.sheet_color(surface_pressed()));

        h.hover(HoverState::Entered);
        h.settle();
        assert_close(h.color(), h.sheet_color(surface_hover_accent()));
    }
}
