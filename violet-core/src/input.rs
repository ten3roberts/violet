use flax::{component::ComponentValue, components::child_of, Component, Entity, EntityRef, World};
use flume::Receiver;
use glam::{Vec2, Vec3Swizzles};
/// NOTE: maybe redefine these types ourselves
pub use winit::{event, keyboard};
use winit::{
    event::{ElementState, MouseButton},
    keyboard::{Key, ModifiersState, SmolStr},
};

use crate::{
    components::{rect, screen_transform},
    declare_atom,
    hierarchy::find_widget_intersect,
    scope::ScopeRef,
    Frame,
};

pub struct Input {}

#[derive(Debug, Clone)]
pub struct FocusedEntity {
    id: Entity,
    sticky: bool,
}

pub struct InputState {
    root: Entity,
    focused: Option<FocusedEntity>,
    last_sticky: Option<Entity>,
    pos: Vec2,
    modifiers: ModifiersState,
    external_focus_rx: Receiver<Entity>,

    hovered_item: Option<Entity>,
}

impl InputState {
    pub fn new(root: Entity, pos: Vec2, external_focus_rx: Receiver<Entity>) -> Self {
        Self {
            focused: None,
            pos,
            modifiers: Default::default(),
            root,
            external_focus_rx,
            last_sticky: None,
            hovered_item: None,
        }
    }

    fn find_intersect<'a>(
        &self,
        frame: &'a Frame,
        pos: Vec2,
        mut filter: impl FnMut(&EntityRef) -> bool,
    ) -> Option<(EntityRef<'a>, Vec2)> {
        find_widget_intersect(self.root, frame, pos, |v| v.has(interactive()) && filter(v))
    }

    pub fn on_mouse_input(
        &mut self,
        frame: &mut Frame,
        state: ElementState,
        button: MouseButton,
    ) -> bool {
        let cursor_pos = self.pos;
        let intersect = self.find_intersect(frame, cursor_pos, |_| true);
        let id = match (state, &self.focused, intersect) {
            // Focus changed
            (ElementState::Pressed, _, new) => {
                let focused = new.map(|v| v.0.id());
                self.set_focused(frame, focused);
                focused
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

            Self::propagate_event(
                entity,
                frame,
                on_mouse_input(),
                MouseInput {
                    modifiers: self.modifiers,
                    state,
                    cursor,
                    button,
                },
            );

            return true;
        }

        false
    }

    fn propagate_event<'a, T: ComponentValue>(
        mut entity: EntityRef<'a>,
        frame: &'a Frame,
        event: Component<InputEventHandler<T>>,
        event_value: T,
    ) -> bool {
        let mut value = Some(event_value);
        loop {
            if let Ok(mut on_input) = entity.get_mut(event) {
                let s = ScopeRef::new(frame, entity);
                if let Some(v) = (on_input)(&s, value.take().unwrap()) {
                    value = Some(v);
                } else {
                    break;
                }
            }

            let Some((parent, _)) = entity.relations(child_of).next() else {
                return false;
            };

            entity = frame.world().entity(parent).unwrap();
        }

        true
    }

    pub fn on_cursor_move(&mut self, frame: &mut Frame, pos: Vec2) -> bool {
        self.pos = pos;

        let target = self.get_focused_or_intersecting(frame, pos);

        let new_hover = self.hovered_item != target.map(|v| v.id());
        if new_hover {
            if let Some(hovered) = self.hovered_item.and_then(|v| frame.world.entity(v).ok()) {
                let transform = hovered.get_copy(screen_transform()).unwrap_or_default();
                let rect = hovered.get_copy(rect()).unwrap_or_default();

                Self::propagate_event(
                    hovered,
                    frame,
                    on_cursor_hover(),
                    CursorOver {
                        state: HoverState::Exited,
                        absolute_pos: pos,
                        local_pos: transform.inverse().transform_point3(pos.extend(0.0)).xy()
                            - rect.min,
                    },
                );
            }
        }

        if let Some(hovered) = target {
            let transform = hovered.get_copy(screen_transform()).unwrap_or_default();
            let rect = hovered.get_copy(rect()).unwrap_or_default();

            Self::propagate_event(
                hovered,
                frame,
                on_cursor_hover(),
                CursorOver {
                    state: if new_hover {
                        HoverState::Entered
                    } else {
                        HoverState::Moved
                    },
                    absolute_pos: pos,
                    local_pos: transform.inverse().transform_point3(pos.extend(0.0)).xy()
                        - rect.min,
                },
            );
        }

        self.hovered_item = target.map(|v| v.id());

        if let &Some(entity) = &self.get_focused(&frame.world) {
            let transform = entity.get_copy(screen_transform()).unwrap_or_default();
            let rect = entity.get_copy(rect()).unwrap_or_default();

            return Self::propagate_event(
                entity,
                frame,
                on_cursor_move(),
                CursorMove {
                    modifiers: self.modifiers,
                    absolute_pos: pos,
                    local_pos: transform.inverse().transform_point3(pos.extend(0.0)).xy()
                        - rect.min,
                },
            );
        }

        false
    }

    pub fn on_scroll(&mut self, frame: &mut Frame, delta: Vec2) -> bool {
        let intersect = self.find_intersect(frame, self.pos, |v| v.has(interactive()));

        if let Some((entity, _)) = intersect {
            let entity = frame.world().entity(entity.id()).unwrap();
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

            return true;
        }

        false
    }

    pub fn on_modifiers_change(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn on_keyboard_input(
        &mut self,
        frame: &mut Frame,
        key: Key,
        state: ElementState,
        text: Option<SmolStr>,
    ) -> bool {
        if let &Some(entity) = &self.get_focused(frame.world()) {
            return Self::propagate_event(
                entity,
                frame,
                on_keyboard_input(),
                KeyboardInput {
                    modifiers: self.modifiers,
                    state,
                    key,
                    text,
                },
            );
        }

        false
    }

    pub fn focused(&self) -> Option<&FocusedEntity> {
        self.focused.as_ref()
    }

    pub fn update_external_focus(&mut self, frame: &Frame) {
        let new_focus = self
            .external_focus_rx
            .drain()
            .filter(|&id| frame.world.is_alive(id))
            .last();

        if let Some(new_focus) = new_focus {
            self.set_focused(frame, Some(new_focus));
        }
    }

    pub fn get_focused<'a>(&self, world: &'a World) -> Option<EntityRef<'a>> {
        self.focused.as_ref().and_then(|v| world.entity(v.id).ok())
    }

    pub fn get_focused_or_intersecting<'a>(
        &self,
        frame: &'a Frame,
        pos: Vec2,
    ) -> Option<EntityRef<'a>> {
        self.get_focused(&frame.world).or_else(|| {
            self.find_intersect(frame, pos, |v| v.has(interactive()))
                .map(|v| v.0)
        })
    }

    fn set_focused(&mut self, frame: &Frame, focused: Option<Entity>) {
        let cur = self.get_focused(&frame.world);

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

            let sticky = entity.has(keep_focus());
            self.focused = Some(FocusedEntity { id: new, sticky });
            if sticky {
                self.last_sticky = Some(new);
            }
        } else {
            self.focused = self
                .last_sticky
                .and_then(|v| frame.world.entity(v).ok())
                .map(|v| FocusedEntity {
                    id: v.id(),
                    sticky: true,
                });
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HoverState {
    Entered,
    Moved,
    Exited,
}

#[derive(Debug, Clone, Copy)]
pub struct CursorOver {
    pub state: HoverState,
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
    pub state: ElementState,
    pub key: Key,
    pub text: Option<SmolStr>,
}

pub type InputEventHandler<T> = Box<dyn Send + Sync + FnMut(&ScopeRef<'_>, T) -> Option<T>>;

declare_atom! {
    pub request_focus_sender: flume::Sender<Entity>,
}

flax::component! {
    pub keep_focus: (),
    pub interactive: (),
    pub on_focus: InputEventHandler<bool>,
    pub on_cursor_move: InputEventHandler<CursorMove>,
    pub on_cursor_hover: InputEventHandler<CursorOver>,
    pub on_mouse_input: InputEventHandler<MouseInput>,
    pub on_keyboard_input: InputEventHandler<KeyboardInput>,
    pub on_scroll: InputEventHandler<Scroll>,
}
