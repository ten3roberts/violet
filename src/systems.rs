use flax::{child_of, BoxedSystem, Query, QueryBorrow, System, World};
use glam::Vec2;

use crate::{
    components::{self, children, rect, Rect},
    layout::{update_subtree, LayoutConstraints},
};

/// Updates the layout for entities using the given constraints
pub fn layout_system() -> BoxedSystem {
    System::builder()
        .read()
        .with(Query::new((rect(), children())).without_relation(child_of))
        .build(move |world: &World, mut roots: QueryBorrow<_, _>| {
            (&mut roots)
                .into_iter()
                .for_each(|(rect, children): (&Rect, &Vec<_>)| {
                    for &child in children {
                        let entity = world.entity(child).unwrap();

                        let rect = update_subtree(
                            world,
                            &entity,
                            *rect,
                            LayoutConstraints {
                                min: Vec2::ZERO,
                                max: rect.size(),
                            },
                        );

                        entity.update(components::rect(), |v| *v = rect.unwrap_or_default());
                    }
                });
        })
        .boxed()
}
