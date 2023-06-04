use std::ops::{Deref, DerefMut};

use flax::World;

pub struct Frame {
    world: World,
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
