use flax::{child_of, BoxedSystem, EntityRef, Query, QueryBorrow, System, World};
use glam::{vec2, Vec2};

use crate::{
    components::{self, children, constraints, padding, rect, Rect},
    constraints::entity_constraints,
    layout::update_subtree,
};

/// Updates the layout for entities using the given constraints
pub fn layout_system() -> BoxedSystem {
    System::builder()
        .read()
        .with(Query::new((rect(), children())).without_relation(child_of))
        .build(move |world: &World, mut roots: QueryBorrow<_, _>| {
            (&mut roots)
                .into_iter()
                .for_each(|(rect, children): (_, &Vec<_>)| {
                    for &child in children {
                        let entity = world.entity(child).unwrap();
                        let rect = entity_constraints(&entity, rect);

                        update_subtree(world, entity, rect);
                    }
                });
        })
        .boxed()
}
