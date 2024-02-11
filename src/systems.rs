use flax::{
    component::ComponentValue, components::child_of, entity_ids, filter::Or, BoxedSystem,
    CommandBuffer, Dfs, DfsBorrow, Entity, Fetch, FetchExt, FetchItem, Query, QueryBorrow, System,
    World,
};
use glam::Vec2;

use crate::{
    components::{
        self, children, layout_bounds, local_position, rect, screen_position, screen_rect, text,
        Rect,
    },
    layout::{update_subtree, LayoutLimits},
};

pub fn hydrate_text() -> BoxedSystem {
    System::builder()
        .with_cmd_mut()
        .with_query(Query::new(entity_ids()).with(text()))
        .build(|cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
            query.for_each(|id| {
                cmd.set_missing(id, layout_bounds(), Vec2::ONE * 100.0);
            })
        })
        .boxed()
}

pub fn templating_system(root: Entity) -> BoxedSystem {
    let query = Query::new(entity_ids())
        .filter(Or((
            screen_position().without(),
            local_position().without(),
            rect().without(),
            screen_rect().without(),
        )))
        .filter(root.traverse(child_of));

    System::builder()
        .with_query(query)
        .with_cmd_mut()
        .build(|mut query: QueryBorrow<_, _>, cmd: &mut CommandBuffer| {
            for id in &mut query {
                tracing::debug!(%id, "incomplete widget");

                cmd.set_missing(id, screen_position(), Vec2::ZERO)
                    .set_missing(id, local_position(), Vec2::ZERO)
                    .set_missing(id, screen_rect(), Rect::default())
                    .set_missing(id, rect(), Rect::default());
            }
        })
        .boxed()
}

/// Updates the layout for entities using the given constraints
pub fn layout_system() -> BoxedSystem {
    System::builder()
        .with_world()
        .with_query(Query::new((rect(), children())).without_relation(child_of))
        .build(move |world: &World, mut roots: QueryBorrow<_, _>| {
            (&mut roots)
                .into_iter()
                .for_each(|(canvas_rect, children): (&Rect, &Vec<_>)| {
                    for &child in children {
                        let entity = world.entity(child).unwrap();

                        let res = update_subtree(
                            world,
                            &entity,
                            canvas_rect.size(),
                            LayoutLimits {
                                min_size: Vec2::ZERO,
                                max_size: canvas_rect.size(),
                            },
                        );

                        entity.update_dedup(components::rect(), res.rect);
                    }
                });
        })
        .boxed()
}

/// Updates the apparent screen position of entities based on the hierarchy
pub fn transform_system() -> BoxedSystem {
    System::builder()
        .with_query(
            Query::new((
                screen_position().as_mut(),
                screen_rect().as_mut(),
                rect(),
                local_position(),
            ))
            .with_strategy(Dfs::new(child_of)),
        )
        .build(|mut query: DfsBorrow<_>| {
            query.traverse(
                &Vec2::ZERO,
                |(pos, screen_rect, rect, local_pos): (&mut Vec2, &mut Rect, &Rect, &Vec2),
                 _,
                 parent_pos| {
                    *pos = *parent_pos + *local_pos;
                    *screen_rect = rect.translate(*pos);
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
        .with_cmd_mut()
        .with_query(Query::new((entity_ids(), query)).filter(filter))
        .build(
            move |cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
                query.for_each(|(id, item)| hydrate(cmd, id, item))
            },
        )
        .boxed();
}
