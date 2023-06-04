use std::ops::{Deref, DerefMut};

use flax::World;

use crate::executor::Spawner;

pub struct Frame {
    pub world: World,
    pub spawner: Spawner<Self>,
}
