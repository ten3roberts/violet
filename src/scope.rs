use std::{
    pin::Pin,
    task::{Context, Poll},
};

use flax::{Component, ComponentValue, Entity, EntityBuilder, EntityRef, EntityRefMut};
use pin_project::pin_project;

use crate::{components::children, effect::Effect, Frame, Widget};

/// The scope within a [`Widget`][crate::Widget] is mounted or modified
pub struct Scope<'a> {
    frame: &'a mut Frame,
    id: Entity,
    data: EntityBuilder,
}

impl<'a> std::fmt::Debug for Scope<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl<'a> Scope<'a> {
    pub(crate) fn new(frame: &'a mut Frame) -> Self {
        let id = frame.world_mut().spawn();
        Self {
            frame,
            id,
            data: EntityBuilder::new(),
        }
    }

    pub(crate) fn try_from_id(frame: &'a mut Frame, id: Entity) -> Option<Self> {
        if frame.world().is_alive(id) {
            Some(Self {
                frame,
                id,
                data: EntityBuilder::new(),
            })
        } else {
            None
        }
    }

    fn flush(&mut self) {
        tracing::debug!(?self.id, "Flushing scope");
        self.data
            .append_to(self.frame.world_mut(), self.id)
            .expect("Entity despawned while scope is alive");
    }

    /// Sets the component value
    pub fn set<T>(&mut self, component: Component<T>, value: T) -> &mut Self
    where
        T: ComponentValue,
    {
        self.data.set(component, value);
        self
    }

    pub fn entity(&mut self) -> EntityRef {
        self.flush();
        self.frame.world().entity(self.id).unwrap()
    }

    pub fn entity_mut(&mut self) -> EntityRefMut {
        self.flush();
        self.frame.world_mut().entity_mut(self.id).unwrap()
    }

    /// Attaches a widget in a sub-scope.
    pub fn attach(&mut self, widget: impl Widget) -> Entity {
        self.flush();
        let id = self.frame.world.spawn();
        self.frame
            .world_mut()
            .entry(self.id, children())
            .unwrap()
            .or_default()
            .push(id);

        let id = {
            let mut s = Scope::new(self.frame);

            widget.mount(&mut s);
            s.id
        };

        assert!(self.frame.world().is_alive(self.id));

        id
    }

    /// Spawns an effect scoped to the lifetime of this entity and scope
    pub fn spawn(&mut self, effect: impl 'static + for<'x> Effect<Scope<'x>>) {
        self.frame.spawn(ScopedEffect {
            id: self.id,
            effect,
        });
    }

    pub fn id(&self) -> Entity {
        self.id
    }
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        self.flush()
    }
}

#[pin_project]
struct ScopedEffect<E> {
    id: Entity,
    #[pin]
    effect: E,
}

impl<E: for<'x> Effect<Scope<'x>>> Effect<Frame> for ScopedEffect<E> {
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>, frame: &mut Frame) -> Poll<()> {
        let p = self.project();

        if let Some(mut scope) = Scope::try_from_id(frame, *p.id) {
            p.effect.poll(context, &mut scope)
        } else {
            Poll::Ready(())
        }
    }
}
