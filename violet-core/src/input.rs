use flax::{component, Entity, EntityRef, FetchExt, World};
use glam::{Vec2, Vec3Swizzles};

/// NOTE: maybe redefine these types ourselves
pub use winit::{event, keyboard};
use winit::{
    event::{ElementState, KeyEvent, MouseButton},
    keyboard::ModifiersState,
};

use crate::{
    components::{rect, screen_transform},
    hierarchy::OrderedDfsIterator,
    scope::ScopeRef,
    Frame,
};

pub struct Input {}

#[derive(Debug, Clone)]
struct FocusedEntity {
    id: Entity,
    sticky: bool,
}

pub struct InputState {
    root: Entity,
    focused: Option<FocusedEntity>,
    pos: Vec2,
    modifiers: ModifiersState,
}

impl InputState {
    pub fn new(root: Entity, pos: Vec2) -> Self {
        Self {
            focused: None,
            pos,
            modifiers: Default::default(),
            root,
        }
    }

    fn find_intersect(
        &self,
        frame: &Frame,
        pos: Vec2,
        mut filter: impl FnMut(&EntityRef) -> bool,
    ) -> Option<(Entity, Vec2)> {
        let query = (screen_transform(), rect()).filtered(focusable().with());
        OrderedDfsIterator::new(&frame.world, frame.world.entity(self.root).unwrap())
            .filter_map(|entity| {
                if !filter(&entity) {
                    return None;
                }

                let mut query = entity.query(&query);
                let (transform, rect) = query.get()?;

                let local_pos = transform.inverse().transform_point3(pos.extend(0.0)).xy();

                if rect.contains_point(local_pos) {
                    Some((entity.id(), local_pos - rect.min))
                } else {
                    None
                }
            })
            .last()
    }

    pub fn on_mouse_input(&mut self, frame: &mut Frame, state: ElementState, button: MouseButton) {
        let cursor_pos = self.pos;
        let intersect = self.find_intersect(frame, cursor_pos, |_| true);

        let id = match (state, &self.focused, intersect) {
            // Focus changed
            (ElementState::Pressed, _, new) => {
                self.set_focused(frame, new.map(|v| v.0));
                new.map(|v| v.0)
            }
            // Released after focusing a widget
            (ElementState::Released, Some(cur), _) => {
                let id = cur.id;
                if !cur.sticky {
                    self.set_focused(frame, None);
                }
                Some(id)
            }
            (ElementState::Released, _, _) => None,
        };

        // Send the event to the intersected entity
        if let Some(entity) = id.and_then(|id| frame.world().entity(id).ok()) {
            let screen_transform = entity.get_copy(screen_transform()).unwrap_or_default();
            let rect = entity.get_copy(rect()).unwrap_or_default();
            let local_pos = screen_transform
                .inverse()
                .transform_point3(cursor_pos.extend(0.0))
                .xy()
                - rect.min;

            let cursor = CursorMove {
                modifiers: self.modifiers,
                absolute_pos: self.pos,
                local_pos,
            };
            if let Ok(mut on_input) = entity.get_mut(on_mouse_input()) {
                let s = ScopeRef::new(frame, entity);
                on_input(
                    &s,
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

        if let Some(entity) = &self.focused(&frame.world) {
            let transform = entity.get_copy(screen_transform()).unwrap_or_default();
            let rect = entity.get_copy(rect()).unwrap_or_default();
            if let Ok(mut on_input) = entity.get_mut(on_cursor_move()) {
                let s = ScopeRef::new(frame, *entity);
                on_input(
                    &s,
                    CursorMove {
                        modifiers: self.modifiers,
                        absolute_pos: pos,
                        local_pos: transform.inverse().transform_point3(pos.extend(0.0)).xy()
                            - rect.min,
                    },
                );
            }
        }
    }

    pub fn on_scroll(&mut self, frame: &mut Frame, delta: Vec2) {
        let intersect = self.find_intersect(frame, self.pos, |v| v.has(on_scroll()));

        if let Some((id, _)) = intersect {
            let entity = frame.world().entity(id).unwrap();
            if let Ok(mut on_input) = entity.get_mut(on_scroll()) {
                let s = ScopeRef::new(frame, entity);
                on_input(
                    &s,
                    Scroll {
                        delta,
                        modifiers: self.modifiers,
                    },
                );
            }
        }
    }

    pub fn on_modifiers_change(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn on_keyboard_input(&mut self, frame: &mut Frame, event: KeyEvent) {
        if let Some(entity) = &self.focused(frame.world()) {
            if let Ok(mut on_input) = entity.get_mut(on_keyboard_input()) {
                let s = ScopeRef::new(frame, *entity);
                on_input(
                    &s,
                    KeyboardInput {
                        modifiers: self.modifiers,
                        event,
                    },
                );
            }
        }
    }

    fn focused<'a>(&self, world: &'a World) -> Option<EntityRef<'a>> {
        self.focused.as_ref().and_then(|v| world.entity(v.id).ok())
    }

    fn set_focused(&mut self, frame: &Frame, focused: Option<Entity>) {
        let cur = self.focused(&frame.world);

        if cur.map(|v| v.id()) == focused {
            return;
        }

        if let Some(cur) = cur {
            if let Ok(mut on_focus) = cur.get_mut(on_focus()) {
                let s = ScopeRef::new(frame, cur);
                on_focus(&s, false);
            }
        }

        if let Some(new) = focused {
            let entity = frame.world().entity(new).unwrap();
            let s = ScopeRef::new(frame, entity);

            if let Ok(mut on_focus) = entity.get_mut(on_focus()) {
                on_focus(&s, true);
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

#[derive(Debug, Clone)]
pub struct Scroll {
    pub delta: Vec2,
    pub modifiers: ModifiersState,
}

pub struct KeyboardInput {
    pub modifiers: ModifiersState,

    pub event: KeyEvent,
}

pub type InputEventHandler<T> = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>, T)>;

component! {
    pub focus_sticky: (),
    pub focusable: (),
    pub on_focus: InputEventHandler<bool>,
    pub on_cursor_move: InputEventHandler<CursorMove>,
    pub on_mouse_input: InputEventHandler<MouseInput>,
    pub on_keyboard_input: InputEventHandler<KeyboardInput>,
    pub on_scroll: InputEventHandler<Scroll>,
}
