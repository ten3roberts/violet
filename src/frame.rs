use std::ops::{Deref, DerefMut};

use flax::World;

use crate::Spawner;

pub struct Frame {
    pub world: World,
    pub spawner: Spawner<Self>,
}

impl Deref for Frame {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.world
    }
}

impl DerefMut for Frame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.world
    }
}
