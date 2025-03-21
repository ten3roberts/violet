use std::future::ready;

use atomic_refcell::AtomicRef;
use flax::{
    component::ComponentValue,
    events::{Event, EventSubscriber},
    Component, Entity, World,
};
use futures::StreamExt;

use crate::{
    assets::AssetCache,
    atom::Atom,
    components::atoms,
    effect::Effect,
    executor::{Spawner, TaskHandle},
    scope::ScopedEffect,
    stored::DynamicStore,
    Scope, StreamEffect, Widget,
};

/// Thread local runtime state of the application.
///
/// Contains the ECS world, asset system, and a thread local store
///
/// Is accessible during mutation events of the ECS world.
pub struct Frame {
    pub store: DynamicStore,
    pub world: World,
    pub spawner: Spawner<Self>,
    pub assets: AssetCache,
}

impl Frame {
    pub fn new(spawner: Spawner<Self>, assets: AssetCache, world: World) -> Self {
        Self {
            store: DynamicStore::default(),
            world,
            spawner,
            assets,
        }
    }

    #[inline]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    #[inline]
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn new_root<W: Widget>(&mut self, widget: W) -> Entity {
        let mut scope = Scope::new(self, tynm::type_name::<W>());
        widget.mount(&mut scope);
        scope.id()
    }

    #[inline]
    pub fn spawn(&self, effect: impl 'static + Effect<Frame>) -> TaskHandle {
        self.spawner.spawn(effect)
    }

    #[inline]
    pub fn spawn_scoped(
        &self,
        id: Entity,
        effect: impl 'static + for<'x> Effect<Scope<'x>>,
    ) -> TaskHandle {
        self.spawner.spawn(ScopedEffect { id, effect })
    }

    /// Scope the frame to a particular *existing* entity
    pub fn scoped(&mut self, id: Entity) -> Option<Scope<'_>> {
        Scope::try_from_id(self, id)
    }

    pub fn store(&self) -> &DynamicStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut DynamicStore {
        &mut self.store
    }

    pub fn monitor<T: ComponentValue>(
        &mut self,
        id: Entity,
        component: Component<T>,
        mut on_change: impl FnMut(Option<&T>) + 'static,
    ) {
        let (tx, rx) = flume::unbounded();

        self.world.subscribe(
            tx.filter_components([component.key()])
                .filter(move |_, v| v.ids.contains(&id)),
        );

        self.spawn_scoped(
            id,
            StreamEffect::new(
                rx.into_stream().filter(move |v| ready(v.id == id)),
                move |scope: &mut Scope<'_>, value: Event| {
                    assert_eq!(scope.id(), value.id);
                    on_change(scope.entity().get(component).ok().as_deref());
                },
            ),
        );
    }

    pub fn set_atom<T: ComponentValue>(&mut self, atom: Atom<T>, value: T) {
        self.world.set(atoms(), atom.0, value).unwrap();
    }

    /// Retrieves the value of an atom.
    ///
    /// Returns `None` if the atom does not exist.
    pub fn get_atom<T: ComponentValue>(&self, atom: Atom<T>) -> Option<AtomicRef<T>> {
        self.world.get(atoms(), atom.0).ok()
    }

    pub fn monitor_atom<T: ComponentValue>(
        &mut self,
        atom: Atom<T>,
        on_change: impl Fn(Option<&T>) + 'static,
    ) {
        self.monitor(atoms(), atom.0, on_change)
    }
}
