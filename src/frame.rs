use flax::{Entity, World};

use crate::{
    assets::AssetCache,
    effect::Effect,
    executor::{Spawner, TaskHandle},
    scope::ScopedEffect,
    Scope, Widget,
};

pub struct Frame {
    pub world: World,
    pub spawner: Spawner<Self>,
    pub assets: AssetCache,
    pub delta_time: f32,
}

impl Frame {
    #[inline]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    #[inline]
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn new_root(&mut self, widget: impl Widget) -> Entity {
        let mut scope = Scope::new(self);
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
    pub(crate) fn scoped(&mut self, id: Entity) -> Option<Scope<'_>> {
        Scope::try_from_id(self, id)
    }
}
