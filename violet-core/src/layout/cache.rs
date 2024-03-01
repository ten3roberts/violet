use std::num::NonZeroUsize;

use flax::{component, components::child_of, Entity, FetchExt, RelationExt, World};
use glam::{IVec2, Vec2};
use lru::LruCache;

use super::{flow::Row, Block, Direction, LayoutLimits, Sizing};

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub(crate) struct QueryKey {
    min_size: IVec2,
    max_size: IVec2,
    content_area: IVec2,
    direction: Direction,
}

impl QueryKey {
    pub fn new(content_area: Vec2, limits: LayoutLimits, direction: Direction) -> Self {
        Self {
            min_size: limits.min_size.as_ivec2(),
            max_size: limits.max_size.as_ivec2(),
            content_area: content_area.as_ivec2(),
            direction,
        }
    }
}

#[derive(Debug)]
pub(crate) struct CachedValue<T> {
    pub(crate) limits: LayoutLimits,
    pub(crate) content_area: Vec2,
    pub(crate) value: T,
}

const TOLERANCE: f32 = 0.1;

impl<T> CachedValue<T> {
    pub(crate) fn new(limits: LayoutLimits, content_area: Vec2, value: T) -> Self {
        Self {
            limits,
            content_area,
            value,
        }
    }

    pub(crate) fn is_valid(&self, limits: LayoutLimits, content_area: Vec2) -> bool {
        self.limits.min_size.abs_diff_eq(limits.min_size, TOLERANCE)
            && self.limits.max_size.abs_diff_eq(limits.max_size, TOLERANCE)
            && self.content_area.abs_diff_eq(content_area, TOLERANCE)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayoutUpdate {
    SizeQueryUpdate,
    LayoutUpdate,
    Explicit,
}

pub struct LayoutCache {
    pub(crate) query: LruCache<QueryKey, CachedValue<Sizing>>,
    pub(crate) query_row: LruCache<QueryKey, CachedValue<Row>>,
    pub(crate) layout: Option<CachedValue<Block>>,
    on_invalidated: Option<Box<dyn Fn(LayoutUpdate) + Send + Sync>>,
}

impl LayoutCache {
    pub fn new(on_invalidated: Option<Box<dyn Fn(LayoutUpdate) + Send + Sync>>) -> Self {
        Self {
            query: LruCache::new(NonZeroUsize::new(64).unwrap()),
            query_row: LruCache::new(NonZeroUsize::new(64).unwrap()),
            layout: None,
            on_invalidated,
        }
    }

    pub fn invalidate(&mut self) {
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdate::Explicit)
        }

        self.query.clear();
        self.query_row.clear();
        self.layout = None;
    }

    pub(crate) fn insert_query(&mut self, key: QueryKey, value: CachedValue<Sizing>) {
        self.query.put(key, value);
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdate::SizeQueryUpdate)
        }
    }

    pub(crate) fn insert_query_row(&mut self, key: QueryKey, value: CachedValue<Row>) {
        self.query_row.put(key, value);
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdate::SizeQueryUpdate)
        }
    }

    pub(crate) fn insert_layout(&mut self, value: CachedValue<Block>) {
        self.layout = Some(value);
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdate::LayoutUpdate)
        }
    }
}

/// Invalidates a widgets layout cache along with its ancestors
pub(crate) fn invalidate_widget(world: &World, id: Entity) {
    let entity = world.entity(id).unwrap();

    let query = (layout_cache().as_mut(), child_of.first_relation().opt());
    let mut query = entity.query(&query);
    let (cache, parent) = query.get().unwrap();

    cache.invalidate();

    if let Some((parent, &())) = parent {
        invalidate_widget(world, parent);
    }
}

component! {
    pub(crate) layout_cache: LayoutCache,
}
