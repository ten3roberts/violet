use std::{
    collections::HashSet,
    sync::{Arc, Weak},
};

use atomic_refcell::AtomicRefCell;
use flax::{
    archetype::ArchetypeStorage,
    component::ComponentValue,
    components::child_of,
    entity_ids,
    events::{EventData, EventSubscriber},
    filter::Or,
    BoxedSystem, CommandBuffer, Component, ComponentMut, Dfs, DfsBorrow, Entity, EntityBuilder,
    Fetch, FetchExt, FetchItem, Query, QueryBorrow, System, World,
};
use glam::{Mat4, Vec2, Vec3, Vec3Swizzles};

use crate::{
    components::{
        self, children, clip_mask, computed_opacity, computed_visible, layout_args, layout_bounds,
        local_position, opacity, rect, screen_clip_mask, screen_transform, text, transform,
        visible,
    },
    layout::{
        apply_layout,
        cache::{invalidate_widget, layout_cache, LayoutCache, LayoutUpdateEvent},
        query_size, Direction, LayoutArgs, LayoutLimits, QueryArgs,
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
        .set(visible(), true)
        .set(computed_visible(), true)
        .set(opacity(), 1.0)
        .set(computed_opacity(), 1.0)
        .set_default(transform())
        .set_default(local_position())
        .set(clip_mask(), Rect::new(Vec2::MIN, Vec2::MAX))
        .set_default(layout_args())
        .set_default(screen_clip_mask())
        .set_default(rect());
}

pub fn templating_system(
    layout_changes_tx: flume::Sender<(Entity, LayoutUpdateEvent)>,
) -> BoxedSystem {
    let query = Query::new(entity_ids()).with_filter(Or((rect().with(), layout_cache().without())));

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

/// Invalidates layout caches when own properties change
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
    // name_map: BTreeMap<ComponentKey, ComponentDesc>,
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
    fn on_added(&self, _: &ArchetypeStorage, event: &EventData) {
        self.mark_dirty(event.ids);
    }

    fn on_modified(&self, event: &EventData) {
        self.mark_dirty(event.ids);
    }

    fn on_removed(&self, _: &ArchetypeStorage, event: &EventData) {
        self.mark_dirty(event.ids);
    }

    fn is_connected(&self) -> bool {
        self.dirty.upgrade().is_some()
    }
}
/// Updates the layout for entities using the given constraints
pub fn layout_system(root: Entity, update_canvas_size: bool) -> BoxedSystem {
    puffin::profile_function!();
    System::builder()
        .with_world()
        .build(move |world: &World| {
            let Ok(entity) = world.entity(root) else {
                return;
            };

            let query = (rect().as_mut(), children().opt_or_default());
            let mut query = entity.query(&query);

            let (canvas_rect, children) = query.get().unwrap();

            puffin::profile_scope!("layout_system");

            let mut total_rect = Rect::ZERO;

            for &child in children {
                let entity = world.entity(child).unwrap();

                if update_canvas_size {
                    let sizing = query_size(
                        world,
                        &entity,
                        QueryArgs {
                            content_area: canvas_rect.size(),
                            limits: LayoutLimits {
                                min_size: Vec2::ZERO,
                                max_size: Vec2::MAX,
                            },
                            direction: Direction::Horizontal,
                        },
                    );

                    total_rect = total_rect.merge(sizing.preferred());
                }

                let res = apply_layout(
                    world,
                    &entity,
                    LayoutArgs {
                        content_area: canvas_rect.size(),
                        limits: LayoutLimits {
                            min_size: Vec2::ZERO,
                            max_size: canvas_rect.size(),
                        },
                    },
                );

                entity.update_dedup(components::rect(), res.rect);
                entity.update_dedup(components::clip_mask(), res.rect);
            }

            if update_canvas_size {
                *canvas_rect = total_rect;
            }
        })
        .boxed()
}

#[derive(Fetch)]
struct TreeUpdateQuery {
    screen_transform: ComponentMut<Mat4>,
    screen_clip_mask: ComponentMut<Rect>,
    clip_mask: Component<Rect>,
    local_position: Component<Vec2>,
    transform: Component<Mat4>,
    visible: Component<bool>,
    computed_visible: ComponentMut<bool>,
    opacity: Component<f32>,
    computed_opacity: ComponentMut<f32>,
}

impl TreeUpdateQuery {
    pub fn new() -> Self {
        Self {
            screen_transform: screen_transform().as_mut(),
            screen_clip_mask: screen_clip_mask().as_mut(),
            clip_mask: clip_mask(),
            local_position: local_position(),
            transform: transform(),
            visible: visible(),
            computed_visible: computed_visible().as_mut(),
            opacity: opacity(),
            computed_opacity: computed_opacity().as_mut(),
        }
    }
}

/// Updates the apparent screen position of entities based on the hierarchy
pub fn transform_system(root: Entity) -> BoxedSystem {
    System::builder()
        .with_query(Query::new(TreeUpdateQuery::new()).with_strategy(Dfs::new(child_of)))
        .build(move |mut query: DfsBorrow<_>| {
            query.traverse_from(
                root,
                &(Mat4::IDENTITY, Rect::new(Vec2::MIN, Vec2::MAX), true, 1.0),
                |item: TreeUpdateQueryItem,
                 _,
                 &(parent, parent_mask, parent_visible, parent_opacity)| {
                    let local_transform =
                        Mat4::from_translation(item.local_position.extend(0.0)) * *item.transform;

                    let mask_offset = parent.transform_point3(Vec3::ZERO).xy();
                    *item.screen_clip_mask =
                        item.clip_mask.translate(mask_offset).intersect(parent_mask);

                    *item.screen_transform = parent * local_transform;
                    *item.computed_visible = *item.visible && parent_visible;
                    *item.computed_opacity = item.opacity * parent_opacity;

                    (
                        *item.screen_transform,
                        *item.screen_clip_mask,
                        *item.computed_visible,
                        *item.computed_opacity,
                    )
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
        .with_query(Query::new((entity_ids(), query)).with_filter(filter))
        .build(
            move |cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
                query.for_each(|(id, item)| hydrate(cmd, id, item))
            },
        )
        .boxed();
}
