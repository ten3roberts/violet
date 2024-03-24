use flax::{EntityRef, World};

use crate::components::children;

pub struct OrderedDfsIterator<'a> {
    world: &'a World,
    // queue: VecDeque<EntityRef<'a>>,
    stack: Vec<EntityRef<'a>>,
}

impl<'a> OrderedDfsIterator<'a> {
    pub fn new(world: &'a World, stack: EntityRef<'a>) -> Self {
        Self {
            world,
            stack: vec![stack],
        }
    }
}

impl<'a> Iterator for OrderedDfsIterator<'a> {
    type Item = EntityRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.stack.pop()?;
        if let Ok(children) = entity.get(children()) {
            self.stack.extend(
                children
                    .iter()
                    .rev()
                    .map(|&id| self.world.entity(id).unwrap()),
            );
        }

        Some(entity)
    }
}
