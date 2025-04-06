use std::{cell::RefCell, time::Duration};

use glam::Vec2;

use crate::{
    components::{offset, opacity},
    input::{interactive, on_cursor_hover, HoverState},
    time::sleep,
    unit::Unit,
    widget::Stack,
    FutureEffect, Scope, Widget,
};

use super::overlay::{overlay_state, CloseOnDropHandle, Overlay};

/// Show a tooltip on hover
pub struct Tooltip<T, P> {
    widget: T,
    preview: Box<dyn Fn() -> P>,
    delay: Duration,
    offset: Vec2,
}

impl<T, P> Tooltip<T, P> {
    pub fn new(widget: T, preview: impl 'static + Fn() -> P) -> Self {
        Self {
            widget,
            preview: Box::new(preview),
            delay: Duration::from_millis(400),
            offset: Vec2::new(10.0, 16.0),
        }
    }

    /// Set the tooltip offset
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Set the delay
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

struct HoverInfo {
    tooltip: Option<CloseOnDropHandle>,
    position: Vec2,
    is_hovering: bool,
}

impl<T: Widget, P: 'static + Send + Widget> Widget for Tooltip<T, P> {
    fn mount(self, scope: &mut Scope<'_>) {
        let create_preview = scope.store(self.preview);

        let info = scope.store(RefCell::new(HoverInfo {
            tooltip: None,
            position: Vec2::ZERO,
            is_hovering: false,
        }));

        scope
            .on_event(on_cursor_hover(), move |scope, event| {
                let existing_info = &mut scope.read(info).borrow_mut();

                existing_info.position = event.absolute_pos;

                if event.state == HoverState::Exited {
                    existing_info.is_hovering = false;
                    existing_info.tooltip = None;
                } else if existing_info.tooltip.is_none() && !existing_info.is_hovering {
                    existing_info.is_hovering = true;
                    existing_info.position = event.absolute_pos;

                    scope.spawn_effect(FutureEffect::new(
                        sleep(self.delay),
                        move |scope: &mut Scope, _| {
                            let mut info = scope.read(&info).borrow_mut();
                            if !info.is_hovering {
                                return;
                            }

                            let overlays = scope.get_context_cloned(overlay_state());
                            let overlay = TooltipOverlay::new(
                                info.position + self.offset,
                                scope.read(&create_preview)(),
                            );

                            let handle = overlays.open(overlay);

                            info.tooltip = Some(CloseOnDropHandle::new(handle));
                        },
                    ));
                }

                Some(event)
            })
            .set(interactive(), ());

        Stack::new(self.widget).mount(scope);
    }
}

pub(crate) struct TooltipOverlay {
    pos: Vec2,
    widget: Box<dyn Send + Widget>,
}

impl TooltipOverlay {
    pub(crate) fn new(pos: Vec2, widget: impl 'static + Send + Widget) -> Self {
        Self {
            pos,
            widget: Box::new(widget),
        }
    }
}

impl Overlay for TooltipOverlay {
    fn create(self, scope: &mut Scope<'_>, _: super::overlay::OverlayHandle) {
        scope.set(offset(), Unit::px(self.pos)).set(opacity(), 0.9);
        self.widget.mount_boxed(scope);
    }
}
