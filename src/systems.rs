use flax::{child_of, BoxedSystem, Dfs, DfsBorrow, Query, QueryBorrow, System, World};
use glam::Vec2;

use crate::{
    components::{self, children, local_position, position, rect, Rect},
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

pub fn transform_system() -> BoxedSystem {
    System::builder()
        .with(Query::new((position().as_mut(), local_position())).with_strategy(Dfs::new(child_of)))
        .build(|mut query: DfsBorrow<_>| {
            query.traverse(
                &Vec2::ZERO,
                |(pos, local_pos): (&mut Vec2, &Vec2), _, parent_pos| {
                    *pos = *parent_pos + *local_pos;
                    *pos
                },
            );
        })
        .boxed()
}
