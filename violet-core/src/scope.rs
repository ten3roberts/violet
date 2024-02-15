use std::{
    pin::Pin,
    task::{Context, Poll},
};

use flax::{
    component::ComponentValue,
    components::{child_of, name},
    Component, Entity, EntityBuilder, EntityRef, EntityRefMut,
};
use futures::{Future, Stream};
use pin_project::pin_project;

use crate::{
    assets::AssetCache, components::children, effect::Effect, stored::Handle, Frame, FutureEffect,
    StreamEffect, Widget,
};

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

    /// Sets the components default value
    pub fn set_default<T>(&mut self, component: Component<T>) -> &mut Self
    where
        T: ComponentValue + Default,
    {
        self.data.set(component, Default::default());
        self
    }

    /// Shorthand for:
    ///
    /// ```rust,ignore
    /// if let Some(val) = val {
    ///     scope.set(val)
    /// }
    /// ```
    pub fn set_opt<T>(&mut self, component: Component<T>, value: Option<T>) -> &mut Self
    where
        T: ComponentValue,
    {
        if let Some(value) = value {
            self.data.set(component, value);
        }
        self
    }

    pub fn entity(&self) -> EntityRef {
        assert_eq!(self.data.component_count(), 0);
        self.frame.world().entity(self.id).unwrap()
    }

    pub fn entity_mut(&mut self) -> EntityRefMut {
        self.flush();
        self.frame.world_mut().entity_mut(self.id).unwrap()
    }

    /// Attaches a widget in a sub-scope.
    pub fn attach<W: Widget>(&mut self, widget: W) -> Entity {
        self.flush();
        let id = self.frame.world.spawn();

        self.frame
            .world_mut()
            .entry(self.id, children())
            .unwrap()
            .or_default()
            .push(id);

        self.flush();

        let id = {
            let mut s = Scope::try_from_id(self.frame, id).unwrap();

            s.set(child_of(self.id), ());
            s.set(name(), tynm::type_name::<W>());

            widget.mount(&mut s);
            s.id
        };

        assert!(self.frame.world().is_alive(self.id));

        id
    }

    /// Detaches a child from the current scope
    pub fn detach(&mut self, id: Entity) {
        assert!(
            self.frame.world.has(id, child_of(self.id)),
            "Attempt to despawn a widget {id} that is not a child of the current scope {}",
            self.id
        );

        self.entity_mut()
            .get_mut(children())
            .unwrap()
            .retain(|&x| x != id);

        self.frame.world.despawn_recursive(id, child_of).unwrap();
    }

    /// Spawns an effect scoped to the lifetime of this entity and scope
    pub fn spawn_effect(&mut self, effect: impl 'static + for<'x> Effect<Scope<'x>>) {
        self.frame.spawn(ScopedEffect {
            id: self.id,
            effect,
        });
    }

    pub fn spawn(&mut self, fut: impl 'static + Future) {
        self.spawn_effect(FutureEffect::new(fut, |_: &mut Scope<'_>, _| {}))
    }

    pub fn spawn_stream(&mut self, stream: impl 'static + Stream) {
        self.spawn_effect(StreamEffect::new(stream, |_: &mut Scope<'_>, _| {}))
    }

    /// Spawns an effect which is *not* scoped to the widget
    pub fn spawn_unscoped(&mut self, effect: impl 'static + for<'x> Effect<Frame>) {
        self.frame.spawn(effect);
    }

    pub fn id(&self) -> Entity {
        self.id
    }

    pub fn assets_mut(&mut self) -> &mut AssetCache {
        &mut self.frame.assets
    }

    pub fn frame(&self) -> &Frame {
        self.frame
    }

    pub fn frame_mut(&mut self) -> &mut Frame {
        self.frame
    }

    /// Stores an arbitrary value and returns a handle to it.
    ///
    /// The value is stored for the duration of the returned handle, and *not* the widgets scope.
    ///
    /// A handle can be used to safely store state across multiple widgets and will not panic if
    /// the original widget is despawned.
    pub fn store<T: 'static>(&mut self, value: T) -> Handle<T> {
        self.frame.store_mut().insert(value)
    }

    pub fn read<T: 'static>(&self, handle: &Handle<T>) -> &T {
        self.frame.store().get(handle)
    }

    pub fn write<T: 'static>(&mut self, handle: &Handle<T>) -> &mut T {
        self.frame.store_mut().get_mut(handle)
    }

    pub fn monitor<T: ComponentValue>(
        &mut self,
        component: Component<T>,
        on_change: impl Fn(Option<&T>) + 'static,
    ) {
        self.frame.monitor(self.id, component, on_change);
    }
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        self.flush()
    }
}

#[pin_project]
pub(crate) struct ScopedEffect<E> {
    pub(crate) id: Entity,
    #[pin]
    pub(crate) effect: E,
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
