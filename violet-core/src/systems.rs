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
    fetch::entity_refs,
    filter::Or,
    BoxedSystem, CommandBuffer, Dfs, DfsBorrow, Entity, EntityBuilder, EntityRef, Fetch, FetchExt,
    FetchItem, Query, QueryBorrow, System, World,
};
use glam::{Mat4, Vec2, Vec3, Vec3Swizzles};

use crate::{
    components::{
        self, children, clip_mask, layout_bounds, layout_limits, local_position, rect,
        screen_clip_mask, screen_transform, text, transform,
    },
    layout::{
        apply_layout,
        cache::{invalidate_widget, layout_cache, LayoutCache, LayoutUpdate},
        LayoutLimits,
    },
    Rect,
};

pub fn hydrate_text() -> BoxedSystem {
    System::builder()
        .with_cmd_mut()
        .with_query(Query::new(entity_ids()).with(text()))
        .build(|cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
            puffin::profile_scope!("hydrate_text");
            query.for_each(|id| {
                cmd.set_missing(id, layout_bounds(), Vec2::ONE * 100.0);
            })
        })
        .boxed()
}

pub fn widget_template(entity: &mut EntityBuilder, name: String) {
    entity
        .set(flax::components::name(), name)
        .set_default(screen_transform())
        .set_default(transform())
        .set_default(local_position())
        .set(clip_mask(), Rect::new(Vec2::MIN, Vec2::MAX))
        .set_default(layout_limits())
        .set_default(screen_clip_mask())
        .set_default(rect());
}

pub fn templating_system(layout_changes_tx: flume::Sender<(Entity, LayoutUpdate)>) -> BoxedSystem {
    let query = Query::new(entity_ids()).filter(Or((rect().with(), layout_cache().without())));

    System::builder()
        .with_name("templating_system")
        .with_query(query)
        .with_cmd_mut()
        .build(
            move |mut query: QueryBorrow<_, _>, cmd: &mut CommandBuffer| {
                puffin::profile_scope!("templating_system");
                for id in query.iter() {
                    puffin::profile_scope!("apply", format!("{id}"));
                    tracing::debug!(%id, "incomplete widget");

                    let layout_changes_tx = layout_changes_tx.clone();
                    cmd.set_missing(
                        id,
                        layout_cache(),
                        LayoutCache::new(Some(Box::new(move |layout| {
                            layout_changes_tx.send((id, layout)).ok();
                        }))),
                    );
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
            puffin::profile_scope!("invalidate_cached_layout_system");
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
pub fn layout_system(root: Entity) -> BoxedSystem {
    puffin::profile_function!();
    System::builder()
        .with_world()
        // .with_query(Query::new((rect(), children())).without_relation(child_of))
        .build(move |world: &World| {
            let Ok(entity) = world.entity(root) else {
                return;
            };
            let query = (rect().opt_or_default(), children().opt_or_default());
            let mut query = entity.query(&query);

            let (canvas_rect, children) = query.get().unwrap();

            puffin::profile_scope!("layout_system");

            for &child in children {
                let entity = world.entity(child).unwrap();

                let res = apply_layout(
                    world,
                    &entity,
                    canvas_rect.size(),
                    LayoutLimits {
                        min_size: Vec2::ZERO,
                        max_size: canvas_rect.size(),
                        overflow_limit: canvas_rect.size(),
                    },
                );

                entity.update_dedup(components::rect(), res.rect);
                entity.update_dedup(components::clip_mask(), res.rect);
            }
        })
        .boxed()
}

/// Updates the apparent screen position of entities based on the hierarchy
pub fn transform_system(root: Entity) -> BoxedSystem {
    System::builder()
        .with_query(
            Query::new((
                entity_refs(),
                screen_transform().as_mut(),
                screen_clip_mask().as_mut(),
                clip_mask(),
                local_position(),
                transform().opt_or_default(),
            ))
            .with_strategy(Dfs::new(child_of)),
        )
        .build(move |mut query: DfsBorrow<_>| {
            query.traverse_from(
                root,
                &(Mat4::IDENTITY, Rect::new(Vec2::MIN, Vec2::MAX)),
                |(entity, screen_trans, screen_mask, &mask, &local_pos, &trans): (
                    EntityRef,
                    &mut Mat4,
                    &mut Rect,
                    &Rect,
                    &Vec2,
                    &Mat4,
                ),
                 _,
                 &(parent, parent_mask)| {
                    let local_transform = Mat4::from_translation(local_pos.extend(0.0)) * trans;

                    let mask_offset = parent.transform_point3(Vec3::ZERO).xy();
                    *screen_mask = mask.translate(mask_offset).intersect(parent_mask);

                    // tracing::info!(%entity, %screen_mask);

                    *screen_trans = parent * local_transform;

                    (*screen_trans, *screen_mask)
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
