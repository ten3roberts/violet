use flax::{EntityRef, World};
use glam::vec2;

use crate::{
    components::{self, children, layout, padding, Rect},
    constraints::entity_constraints,
};

#[derive(Debug)]
pub struct Layout {}

impl Layout {
    fn apply(&self, world: &World, entity: &EntityRef, mut inner_rect: Rect) {
        let children = entity.get(children()).ok();
        let children = children.as_ref().map(|v| v.as_slice()).unwrap_or_default();

        for &child in children {
            let child = world.entity(child).expect("Invalid child");
            let rect = entity_constraints(&child, &inner_rect);
            update_subtree(world, child, rect);

            inner_rect.min.x = rect.max.x + 10.0;
        }
    }
}

pub(crate) fn update_subtree(world: &World, entity: EntityRef, rect: Rect) -> Option<()> {
    *entity.get_mut(components::rect()).ok()? = rect;

    let padding = entity
        .get(padding())
        .as_deref()
        .copied()
        .unwrap_or_default();

    let inner_rect = Rect {
        min: rect.min + vec2(padding.left, padding.top),
        max: rect.max - vec2(padding.right, padding.bottom),
    };
    if let Ok(layout) = entity.get(layout()) {
        // Managed
        layout.apply(world, &entity, inner_rect)
    } else if let Ok(children) = entity.get(children()) {
        for &child in &*children {
            let entity = world.entity(child).unwrap();

            let rect = entity_constraints(&entity, &inner_rect);

            update_subtree(world, entity, rect);
        }
    }

    Some(())
}
