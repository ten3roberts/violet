use std::{cell::RefCell, time::Duration};

use glam::Vec2;
use winit::event::ElementState;

use crate::{
    executor::TaskHandle,
    input::{interactive, on_cursor_hover, on_mouse_input, HoverState, MouseInput},
    style::{SizeExt, WidgetSizeProps},
    time::sleep,
    widget::{label, pill},
    FutureEffect, Scope, ScopeRef, Widget,
};

use super::{
    overlay::{overlay_state, CloseOnDropHandle},
    tooltip::TooltipOverlay,
};

pub type ClickCallback = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>)>;
pub type PointerPressCallback = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>, ElementState)>;
pub type MouseInputCallback =
    Box<dyn Send + Sync + FnMut(&ScopeRef<'_>, MouseInput) -> Option<MouseInput>>;
pub type CreateTooltip = Box<dyn Send + Sync + Fn(&ScopeRef<'_>) -> Box<dyn Send + Widget>>;

pub struct TooltipOptions {
    pub delay: Duration,
    pub create_tooltip: CreateTooltip,
    pub offset: Vec2,
}

impl TooltipOptions {
    pub fn new<W: 'static + Send + Widget>(
        tooltip: impl Fn(&ScopeRef<'_>) -> W + Send + Sync + 'static,
    ) -> Self {
        Self {
            delay: Duration::from_millis(400),
            create_tooltip: Box::new(move |scope| Box::new(tooltip(scope))),
            offset: Vec2::new(10.0, 16.0),
        }
    }

    pub fn label(tooltip: impl Into<String>) -> Self {
        let tooltip = tooltip.into();
        Self::new(move |_| label(&tooltip))
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

/// Base interactive widget.
///
/// Consider [`Button`] or [`Checkbox`] instead.
///
/// Consumes click events.
pub struct InteractiveWidget<W> {
    on_click: Option<ClickCallback>,
    double_click: Option<ClickCallback>,
    on_press: Option<PointerPressCallback>,
    on_right_click: Option<ClickCallback>,
    on_mouse_input: Option<MouseInputCallback>,
    size: WidgetSizeProps,
    tooltip: Option<TooltipOptions>,
    inner: W,
}

impl<W: Widget> InteractiveWidget<W> {
    pub fn new(inner: W) -> Self {
        Self {
            on_click: None,
            on_press: None,
            on_right_click: None,
            on_mouse_input: None,
            tooltip: None,
            size: Default::default(),
            inner,
            double_click: None,
        }
    }

    pub fn on_click<F>(mut self, on_click: F) -> Self
    where
        F: FnMut(&ScopeRef<'_>) + Send + Sync + 'static,
    {
        self.on_click = Some(Box::new(on_click));
        self
    }

    pub fn on_double_click<F>(mut self, on_double_click: F) -> Self
    where
        F: FnMut(&ScopeRef<'_>) + Send + Sync + 'static,
    {
        self.double_click = Some(Box::new(on_double_click));
        self
    }

    pub fn on_double_click_opt<F>(mut self, on_double_click: Option<F>) -> Self
    where
        F: FnMut(&ScopeRef<'_>) + Send + Sync + 'static,
    {
        if let Some(on_double_click) = on_double_click {
            self.double_click = Some(Box::new(on_double_click));
        } else {
            self.double_click = None;
        }
        self
    }

    pub fn on_generic_mouse_input<F>(mut self, on_mouse_input: F) -> Self
    where
        F: FnMut(&ScopeRef<'_>, MouseInput) -> Option<MouseInput> + Send + Sync + 'static,
    {
        self.on_mouse_input = Some(Box::new(on_mouse_input));
        self
    }

    pub fn on_mouse_input_opt<F>(mut self, on_mouse_input: Option<F>) -> Self
    where
        F: FnMut(&ScopeRef<'_>, MouseInput) -> Option<MouseInput> + Send + Sync + 'static,
    {
        if let Some(on_mouse_input) = on_mouse_input {
            self.on_mouse_input = Some(Box::new(on_mouse_input));
        } else {
            self.on_mouse_input = None;
        }
        self
    }

    pub fn on_pointer_press<F>(mut self, on_press: F) -> Self
    where
        F: FnMut(&ScopeRef<'_>, ElementState) + Send + Sync + 'static,
    {
        self.on_press = Some(Box::new(on_press));
        self
    }

    pub fn on_right_click<F>(mut self, on_click: F) -> Self
    where
        F: FnMut(&ScopeRef<'_>) + Send + Sync + 'static,
    {
        self.on_right_click = Some(Box::new(on_click));
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

    pub fn with_tooltip_opt(mut self, tooltip: Option<TooltipOptions>) -> Self {
        self.tooltip = tooltip;
        self
    }
}

impl<W: Widget> Widget for InteractiveWidget<W> {
    fn mount(mut self, scope: &mut crate::Scope<'_>) {
        let mut is_pressed = false;

        let info = scope.store(RefCell::new(HoverInfo {
            tooltip: None,
            position: Vec2::ZERO,
            is_hovering: false,
        }));

        let tooltip_info = self.tooltip.map(|v| scope.store(v));

        scope.on_event(on_cursor_hover(), move |scope, event| {
            let existing_info = &mut scope.read(info).borrow_mut();

            existing_info.position = event.absolute_pos;

            if event.state == HoverState::Exited {
                existing_info.is_hovering = false;
                existing_info.tooltip = None;
            } else if existing_info.tooltip.is_none() && !existing_info.is_hovering {
                existing_info.is_hovering = true;
                existing_info.position = event.absolute_pos;

                if let Some(tooltip) = tooltip_info {
                    let tooltip_info = scope.read(tooltip);

                    scope.spawn_effect(FutureEffect::new(
                        sleep(tooltip_info.delay),
                        move |scope: &mut Scope, _| {
                            let tooltip_info = scope.read(&tooltip);
                            let mut info = scope.read(&info).borrow_mut();
                            if !info.is_hovering {
                                return;
                            }

                            let overlays = scope.get_context_cloned(overlay_state());
                            let overlay = TooltipOverlay::new(
                                info.position + tooltip_info.offset,
                                pill((tooltip_info.create_tooltip)(&ScopeRef::from_scope(scope))),
                            );

                            let handle = overlays.open(overlay);

                            info.tooltip = Some(CloseOnDropHandle::new(handle));
                        },
                    ));
                }
            }

            Some(event)
        });

        let mut last_click = None as Option<web_time::Instant>;
        let double_click_timeout = Duration::from_millis(250);
        let on_click = self.on_click.map(|v| scope.store(RefCell::new(v)));

        let mut click_action = None as Option<TaskHandle>;

        let mut click_handler = move |scope: &ScopeRef| {
            let now = web_time::Instant::now();

            // Perform double click detection
            if last_click.is_some_and(|v| now.duration_since(v) < double_click_timeout) {
                // abort normal click
                click_action.take().map(|v| v.abort());
                tracing::debug!("double click");
                if let Some(double_click) = &mut self.double_click {
                    double_click(scope);
                }
            } else if self.double_click.is_some() {
                // delay normal click
                click_action = Some(scope.spawn_effect(FutureEffect::new(
                    sleep(double_click_timeout),
                    move |scope: &mut Scope, _| {
                        if let Some(on_click) = on_click {
                            // If we have a click handler, call it
                            (scope.read(&on_click).borrow_mut())(&ScopeRef::from_scope(scope));
                        }
                    },
                )));
                last_click = Some(now);
            } else if let Some(on_click) = on_click {
                (scope.read(on_click).borrow_mut())(scope);
            }
        };

        scope
            .set_default(interactive())
            .on_event(on_mouse_input(), move |scope, input| {
                if let Some(on_mouse_input) = &mut self.on_mouse_input {
                    if (on_mouse_input)(scope, input).is_none() {
                        return None;
                    }
                }

                match input.button {
                    winit::event::MouseButton::Left => {
                        if let Some(on_press) = &mut self.on_press {
                            (on_press)(scope, input.state)
                        }

                        if input.state == ElementState::Pressed {
                            is_pressed = true;
                        } else if is_pressed {
                            is_pressed = false;
                            click_handler(scope)
                        }
                    }
                    winit::event::MouseButton::Right if input.state == ElementState::Pressed => {
                        if let Some(v) = &mut self.on_right_click {
                            (v)(scope)
                        }
                    }
                    _ => {}
                }

                None
            });

        self.size.mount(scope);
        self.inner.mount(scope);
    }
}

impl<W> SizeExt for InteractiveWidget<W> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}

struct HoverInfo {
    tooltip: Option<CloseOnDropHandle>,
    position: Vec2,
    is_hovering: bool,
}
