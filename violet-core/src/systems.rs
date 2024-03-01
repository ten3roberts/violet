use std::{
    collections::HashSet,
    sync::{Arc, Weak},
};

use atomic_refcell::AtomicRefCell;
use flax::{
    archetype::Storage,
    component::ComponentValue,
    components::child_of,
    entity_ids,
    events::{EventData, EventSubscriber},
    filter::Or,
    sink::Sink,
    BoxedSystem, CommandBuffer, Dfs, DfsBorrow, Entity, Fetch, FetchExt, FetchItem, Query,
    QueryBorrow, System, World,
};
use glam::Vec2;

use crate::{
    components::{
        self, children, layout_bounds, local_position, rect, screen_position, screen_rect, text,
    },
    layout::{
        cache::{invalidate_widget, layout_cache, LayoutCache, LayoutUpdate},
        update_subtree, LayoutLimits,
    },
    Rect,
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

pub fn templating_system(
    root: Entity,
    layout_changes_tx: flume::Sender<(Entity, LayoutUpdate)>,
) -> BoxedSystem {
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
        .build(
            move |mut query: QueryBorrow<_, _>, cmd: &mut CommandBuffer| {
                for id in &mut query {
                    tracing::debug!(%id, "incomplete widget");

                    let layout_changes_tx = layout_changes_tx.clone();
                    cmd.set_missing(id, screen_position(), Vec2::ZERO)
                        .set_missing(id, local_position(), Vec2::ZERO)
                        .set_missing(id, screen_rect(), Rect::default())
                        .set_missing(
                            id,
                            layout_cache(),
                            LayoutCache::new(Some(Box::new(move |layout| {
                                layout_changes_tx.send((id, layout)).ok();
                            }))),
                        )
                        .set_missing(id, rect(), Rect::default());
                }
            },
        )
        .boxed()
}

/// Invalidates layout caches
pub fn invalidate_cached_layout_system(world: &mut World) -> BoxedSystem {
    let components = [
        components::min_size().key(),
        components::size().key(),
        components::max_size().key(),
        components::offset().key(),
        components::anchor().key(),
        components::aspect_ratio().key(),
        components::padding().key(),
        components::margin().key(),
        components::children().key(),
        components::text().key(),
        components::layout().key(),
    ];

    let dirty = Arc::new(AtomicRefCell::new(HashSet::new()));

    let invalidator = QueryInvalidator {
        dirty: Arc::downgrade(&dirty),
    };

    world.subscribe(invalidator.filter_components(components));

    System::builder()
        .with_world_mut()
        .build(move |world: &mut World| {
            for id in dirty.borrow_mut().drain() {
                if world.is_alive(id) {
                    invalidate_widget(world, id);
                }
            }
        })
        .boxed()
}

struct QueryInvalidator {
    dirty: Weak<AtomicRefCell<HashSet<Entity>>>,
}

impl QueryInvalidator {
    pub fn mark_dirty(&self, ids: &[Entity]) {
        if let Some(dirty) = self.dirty.upgrade() {
            dirty.borrow_mut().extend(ids);
        }
    }
}

impl EventSubscriber for QueryInvalidator {
    fn on_added(&self, _: &Storage, event: &EventData) {
        self.mark_dirty(event.ids);
    }

    fn on_modified(&self, event: &EventData) {
        self.mark_dirty(event.ids);
    }

    fn on_removed(&self, _: &Storage, event: &EventData) {
        self.mark_dirty(event.ids);
    }

    fn is_connected(&self) -> bool {
        self.dirty.upgrade().is_some()
    }
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
