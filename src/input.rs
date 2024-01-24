use flax::{
    component, components::child_of, entity_ids, fetch::Satisfied, filter::All, Component, Entity,
    EntityIds, EntityRef, Fetch, FetchExt, Query, Topo,
};
use glam::Vec2;
use winit::event::{ElementState, KeyboardInput, MouseButton};

use crate::{
    components::{rect, screen_position, Rect},
    Frame,
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
}

impl InputState {
    pub fn new(pos: Vec2) -> Self {
        Self {
            focused: None,
            pos,
            intersect_query: Query::new(IntersectQuery::new()).topo(child_of),
        }
    }

    pub fn on_cursor_move(&mut self, pos: Vec2) {
        self.pos = pos;
    }

    pub fn on_mouse_input(&mut self, frame: &mut Frame, state: ElementState, input: MouseButton) {
        let cursor_pos = self.pos;

        let intersect = self
            .intersect_query
            .borrow(frame.world())
            .iter()
            .filter_map(|item| {
                let local_pos = cursor_pos - *item.screen_pos;
                if item.rect.contains_point(local_pos) {
                    Some(item.id)
                } else {
                    None
                }
            })
            .last();

        match (state, &self.focused, intersect) {
            // Focus changed
            (ElementState::Pressed, _, new) => self.set_focused(frame, new),
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

        if let Some(id) = intersect {
            let entity = frame.world().entity(id).unwrap();

            tracing::info!(%entity, "sending input event");
            if let Ok(mut on_input) = entity.get_mut(on_mouse_input()) {
                on_input(frame, &entity, state, input);
            }
        }
    }

    pub fn on_keyboard_input(&mut self, frame: &mut Frame, input: KeyboardInput) {
        if let Some(cur) = &self.focused {
            tracing::info!(?cur, "sending keyboard input event");
            let entity = frame.world.entity(cur.id).unwrap();

            if let Ok(mut on_input) = entity.get_mut(on_keyboard_input()) {
                on_input(frame, &entity, input);
            }
        }
    }

    pub fn on_char_input(&mut self, frame: &mut Frame, input: char) {
        if let Some(cur) = &self.focused {
            tracing::info!(?input, "input char");
            let entity = frame.world.entity(cur.id).unwrap();

            if let Ok(mut on_input) = entity.get_mut(on_char_typed()) {
                on_input(frame, &entity, input);
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

pub type OnMouseInput = Box<dyn FnMut(&Frame, &EntityRef, ElementState, MouseButton) + Send + Sync>;
pub type OnFocus = Box<dyn FnMut(&Frame, &EntityRef, bool) + Send + Sync>;
pub type OnKeyboardInput = Box<dyn FnMut(&Frame, &EntityRef, KeyboardInput) + Send + Sync>;
pub type OnCharTyped = Box<dyn FnMut(&Frame, &EntityRef, char) + Send + Sync>;

component! {
    pub focus_sticky: (),
    pub focusable: (),
    pub on_focus: OnFocus,
    pub on_mouse_input: OnMouseInput,
    pub on_keyboard_input: OnKeyboardInput,
    pub on_char_typed: OnCharTyped,
}
