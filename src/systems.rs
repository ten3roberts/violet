use flax::{
    child_of, entity_ids, BoxedSystem, CommandBuffer, ComponentValue, Dfs, DfsBorrow, Entity,
    Fetch, FetchItem, Query, QueryBorrow, System, World,
};
use glam::Vec2;
use image::buffer::ConvertBuffer;

use crate::{
    components::{self, children, local_position, rect, screen_position, Rect},
    layout::{update_subtree, LayoutLimits},
};

/// Updates the layout for entities using the given constraints
pub fn layout_system() -> BoxedSystem {
    System::builder()
        .read()
        .with(Query::new((rect(), children())).without_relation(child_of))
        .build(move |world: &World, mut roots: QueryBorrow<_, _>| {
            (&mut roots)
                .into_iter()
                .for_each(|(canvas_rect, children): (&Rect, &Vec<_>)| {
                    for &child in children {
                        let entity = world.entity(child).unwrap();

                        let res = update_subtree(
                            world,
                            &entity,
                            *canvas_rect,
                            LayoutLimits {
                                min: Vec2::ZERO,
                                max: canvas_rect.size(),
                            },
                        );

                        entity.update(components::rect(), |v| *v = res.rect);
                    }
                });
        })
        .boxed()
}

pub fn transform_system() -> BoxedSystem {
    System::builder()
        .with(
            Query::new((screen_position().as_mut(), local_position()))
                .with_strategy(Dfs::new(child_of)),
        )
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

pub fn hydrate<Q, F, Func>(query: Q, filter: F, mut hydrate: Func)
where
    Q: ComponentValue + for<'x> Fetch<'x>,
    F: ComponentValue + for<'x> Fetch<'x>,
    Func: ComponentValue + for<'x> FnMut(&mut CommandBuffer, Entity, <Q as FetchItem<'x>>::Item),
{
    System::builder()
        .write::<CommandBuffer>()
        .with(Query::new((entity_ids(), query)).filter(filter))
        .build(
            move |cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
                query.for_each(|(id, item)| hydrate(cmd, id, item))
            },
        )
        .boxed();
}
