use flax::{Entity, EntityRef, FetchExt, World};
use glam::{Vec2, Vec3, Vec3Swizzles};

use crate::{
    components::{children, rect, screen_clip_mask, screen_transform},
    input::interactive,
    Frame,
};

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

pub fn find_widget_intersect(
    root: Entity,
    frame: &Frame,
    pos: Vec2,
    mut filter: impl FnMut(&EntityRef) -> bool,
) -> Option<(EntityRef, Vec2)> {
    let query = (screen_transform(), rect(), screen_clip_mask());
    OrderedDfsIterator::new(&frame.world, frame.world.entity(root).unwrap())
        .filter_map(|entity| {
            if !filter(&entity) {
                return None;
            }

            let mut query = entity.query(&query);
            let (transform, rect, clip_mask) = query.get()?;

            let translation = transform.transform_point3(Vec3::ZERO).xy();
            let clipped_rect = rect
                .translate(translation)
                .clip(*clip_mask)
                .translate(-translation);

            let local_pos = transform.inverse().transform_point3(pos.extend(0.0)).xy();

            if clipped_rect.contains_point(local_pos) {
                Some((entity, local_pos - rect.min))
            } else {
                None
            }
        })
        .last()
}
