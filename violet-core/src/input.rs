use std::str::FromStr;

use flax::{
    component, components::child_of, entity_ids, fetch::Satisfied, filter::All, Component, Entity,
    EntityIds, EntityRef, Fetch, FetchExt, Mutable, Query, Topo,
};
use glam::Vec2;

/// NOTE: maybe redefine these types ourselves
pub use winit::{event, keyboard};
use winit::{
    event::{ElementState, KeyEvent, MouseButton},
    keyboard::ModifiersState,
};

use crate::{
    components::{rect, screen_position, screen_rect},
    Frame, Rect,
};

pub struct Input {}

#[derive(Fetch)]
struct IntersectQuery {
    id: EntityIds,
    rect: Component<Rect>,
    screen_pos: Component<Vec2>,
    sticky: Satisfied<Component<()>>,
    focusable: Component<()>,
}

impl IntersectQuery {
    pub fn new() -> Self {
        Self {
            id: entity_ids(),
            rect: rect(),
            screen_pos: screen_position(),
            sticky: focus_sticky().satisfied(),
            focusable: focusable(),
        }
    }
}

#[derive(Debug, Clone)]
struct FocusedEntity {
    id: Entity,
    sticky: bool,
}

pub struct InputState {
    focused: Option<FocusedEntity>,
    pos: Vec2,
    intersect_query: Query<IntersectQuery, All, Topo>,
    modifiers: ModifiersState,
}

impl InputState {
    pub fn new(pos: Vec2) -> Self {
        Self {
            focused: None,
            pos,
            intersect_query: Query::new(IntersectQuery::new()).topo(child_of),
            modifiers: Default::default(),
        }
    }

    pub fn on_mouse_input(&mut self, frame: &mut Frame, state: ElementState, button: MouseButton) {
        let cursor_pos = self.pos;

        let intersect = self
            .intersect_query
            .borrow(frame.world())
            .iter()
            .filter_map(|item| {
                let local_pos = cursor_pos - *item.screen_pos;
                if item.rect.contains_point(local_pos) {
                    Some((item.id, (*item.screen_pos + item.rect.min)))
                } else {
                    None
                }
            })
            .last();

        match (state, &self.focused, intersect) {
            // Focus changed
            (ElementState::Pressed, _, new) => self.set_focused(frame, new.map(|v| v.0)),
            // Released after focusing a widget
            (ElementState::Released, Some(cur), _) => {
                if !cur.sticky {
                    tracing::info!(?cur, "focus lost on release");
                    self.set_focused(frame, None);
                }
            }
            (ElementState::Released, _, _) => {}
        }

        // Send the event to the intersected entity

        if let Some((id, origin)) = intersect {
            let entity = frame.world().entity(id).unwrap();

            tracing::info!(%entity, "sending input event");
            let cursor = CursorMove {
                modifiers: self.modifiers,
                absolute_pos: self.pos,
                local_pos: self.pos - origin,
            };
            if let Ok(mut on_input) = entity.get_mut(on_mouse_input()) {
                on_input(
                    frame,
                    &entity,
                    MouseInput {
                        modifiers: self.modifiers,
                        state,
                        cursor,
                        button,
                    },
                );
            }
        }
    }

    pub fn on_cursor_move(&mut self, frame: &mut Frame, pos: Vec2) {
        self.pos = pos;

        if let Some(cur) = &self.focused {
            let entity = frame.world.entity(cur.id).unwrap();

            let screen_rect = entity.get_copy(screen_rect()).unwrap_or_default();
            if let Ok(mut on_input) = entity.get_mut(on_cursor_move()) {
                on_input(
                    frame,
                    &entity,
                    CursorMove {
                        modifiers: self.modifiers,
                        absolute_pos: pos,
                        local_pos: pos - screen_rect.min,
                    },
                );
            }
        }
    }

    pub fn on_modifiers_change(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn on_keyboard_input(&mut self, frame: &mut Frame, event: KeyEvent) {
        if let Some(cur) = &self.focused {
            tracing::info!(?cur, "sending keyboard input event");
            let entity = frame.world.entity(cur.id).unwrap();

            if let Ok(mut on_input) = entity.get_mut(on_keyboard_input()) {
                on_input(
                    frame,
                    &entity,
                    KeyboardInput {
                        modifiers: self.modifiers,
                        event,
                    },
                );
            }
        }
    }

    fn set_focused(&mut self, frame: &Frame, focused: Option<Entity>) {
        let cur = self.focused.as_ref().map(|v| v.id);

        if cur == focused {
            return;
        }

        if let Some(cur) = &self.focused {
            let entity = frame.world().entity(cur.id).unwrap();

            if let Ok(mut on_focus) = entity.get_mut(on_focus()) {
                on_focus(frame, &entity, false);
            }
        }

        if let Some(new) = focused {
            let entity = frame.world().entity(new).unwrap();

            if let Ok(mut on_focus) = entity.get_mut(on_focus()) {
                on_focus(frame, &entity, true);
            }

            let sticky = entity.has(focus_sticky());
            self.focused = Some(FocusedEntity { id: new, sticky });
        } else {
            self.focused = None;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MouseInput {
    pub modifiers: ModifiersState,
    pub state: ElementState,
    pub cursor: CursorMove,
    pub button: MouseButton,
}

#[derive(Debug, Clone, Copy)]
pub struct CursorMove {
    pub modifiers: ModifiersState,
    /// Mouse cursor relative to the screen
    pub absolute_pos: Vec2,
    /// Mouse cursor relative to the bounds of the widget
    pub local_pos: Vec2,
}

pub struct KeyboardInput {
    pub modifiers: ModifiersState,

    pub event: KeyEvent,
}

pub type InputEventHandler<T> = Box<dyn Send + Sync + FnMut(&Frame, &EntityRef, T)>;

component! {
    pub focus_sticky: (),
    pub focusable: (),
    pub on_focus: InputEventHandler<bool>,
    pub on_cursor_move: InputEventHandler<CursorMove>,
    pub on_mouse_input: InputEventHandler<MouseInput>,
    pub on_keyboard_input: InputEventHandler<KeyboardInput>,
}
