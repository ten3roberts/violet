use std::cell::RefCell;

use flax::{component, EntityRef};
use futures_signals::signal::{Mutable, MutableSignal, SignalExt};
use glam::Vec2;
use winit::event::ElementState;

use crate::{
    components::{offset, visible},
    hierarchy::find_widget_intersect,
    input::{interactive, on_cursor_move, on_mouse_input},
    unit::Unit,
    ScopeRef, Widget,
};

use super::overlay::{overlay_state, Overlay, OverlayHandle};

type OnDropFn = Box<dyn Fn(&ScopeRef<'_>, Option<EntityRef>)>;

component! {
    pub drop_target: (),
}

/// Makes the supplied widget draggable
pub struct Draggable<T, P> {
    widget: T,
    preview: Box<dyn Fn() -> P>,
    on_drop: OnDropFn,
    hide_on_drag: bool,
}

impl<T, P> Draggable<T, P> {
    pub fn new(
        widget: T,
        preview: impl 'static + Fn() -> P,
        on_drop: impl 'static + Fn(&ScopeRef<'_>, Option<EntityRef>),
    ) -> Self {
        Self {
            widget,
            preview: Box::new(preview),
            on_drop: Box::new(on_drop),
            hide_on_drag: true,
        }
    }

    /// Set the hide on drag
    pub fn with_hide_on_drag(mut self, hide_on_drag: bool) -> Self {
        self.hide_on_drag = hide_on_drag;
        self
    }
}

#[derive(Default)]
struct DragState {
    drag_start: Option<Vec2>,
    drag_offset: Vec2,
    dragging: bool,
    preview: Option<OverlayHandle>,
}

impl<T: Widget, P: 'static + Send + Widget> Widget for Draggable<T, P> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let drag = scope.store(RefCell::new(DragState::default()));
        let overlays = scope.get_context_cloned(overlay_state());

        let on_drop = scope.store(self.on_drop);
        let create_preview = scope.store(self.preview);
        let preview_position = Mutable::new(Vec2::ZERO);

        scope
            .on_event(on_cursor_move(), move |scope, event| {
                let mut drag = scope.read(drag).borrow_mut();
                if let Some(start) = drag.drag_start {
                    if start.distance(event.absolute_pos) > 2.0 {
                        preview_position.set(event.absolute_pos - drag.drag_offset);
                        if !drag.dragging {
                            drag.dragging = true;
                            drag.preview = Some(overlays.open(DragOverlay::new(
                                preview_position.signal(),
                                scope.read(create_preview)(),
                            )));
                        }
                        if self.hide_on_drag {
                            scope.update_dedup(visible(), false);
                        }
                    }
                    return None;
                }

                Some(event)
            })
            .on_event(on_mouse_input(), move |scope, input| {
                let mut drag = scope.read(drag).borrow_mut();
                if input.state == ElementState::Pressed {
                    drag.drag_start = Some(input.cursor.absolute_pos);
                    drag.drag_offset = input.cursor.local_pos;
                } else {
                    drag.drag_start = None;
                    let dragging = drag.dragging;
                    drag.dragging = false;

                    if let Some(preview) = drag.preview.take() {
                        preview.close();
                    }

                    if self.hide_on_drag {
                        scope.update_dedup(visible(), true);
                    }

                    if dragging {
                        let drop_target = find_widget_intersect(
                            scope.root(),
                            scope.frame(),
                            input.cursor.absolute_pos,
                            |v| v.has(drop_target()),
                        );

                        scope.read(on_drop)(scope, drop_target.map(|v| v.0))
                    }
                }

                None
            })
            .set(interactive(), ());

        self.widget.mount(scope);
    }
}

pub(crate) struct DragOverlay {
    position: MutableSignal<Vec2>,
    widget: Box<dyn Send + Widget>,
}

impl DragOverlay {
    pub(crate) fn new(position: MutableSignal<Vec2>, widget: impl 'static + Send + Widget) -> Self {
        Self {
            position,
            widget: Box::new(widget),
        }
    }
}

impl Overlay for DragOverlay {
    fn create(self, scope: &mut crate::Scope<'_>, _: super::overlay::OverlayHandle) {
        // Float::new(|scope: &mut Scope| {
        scope.spawn_stream(self.position.to_stream(), |scope, pos| {
            scope.set(offset(), Unit::px(pos));
        });

        self.widget.mount_boxed(scope);
        // })
        // .mount(scope);
    }
}
