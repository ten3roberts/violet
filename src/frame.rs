use flax::{Entity, World};

use crate::{
    assets::AssetCache,
    effect::Effect,
    executor::{Spawner, TaskHandle},
    Scope, Widget,
};

pub struct Frame {
    pub world: World,
    pub spawner: Spawner<Self>,
    pub assets: AssetCache,
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

    /// Scope the frame to a particular *existing* entity
    pub(crate) fn scoped(&mut self, id: Entity) -> Option<Scope<'_>> {
        Scope::try_from_id(self, id)
    }
}
