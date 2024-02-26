use std::num::NonZeroUsize;

use flax::{component, components::child_of, Entity, FetchExt, RelationExt, World};
use glam::{IVec2, Vec2};
use lru::LruCache;

use super::{Block, Direction, LayoutLimits, Sizing};

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

pub(crate) struct CachedQuery {
    pub(crate) min_size: Vec2,
    pub(crate) max_size: Vec2,
    pub(crate) content_area: Vec2,
    pub(crate) sizing: Sizing,
}

const TOLERANCE: f32 = 0.01;

impl CachedQuery {
    pub(crate) fn new(min_size: Vec2, max_size: Vec2, content_area: Vec2, sizing: Sizing) -> Self {
        Self {
            min_size,
            max_size,
            content_area,
            sizing,
        }
    }

    pub(crate) fn is_valid(&self, limits: LayoutLimits, content_area: Vec2) -> bool {
        self.min_size.abs_diff_eq(limits.min_size, TOLERANCE)
            && self.max_size.abs_diff_eq(limits.max_size, TOLERANCE)
            && self.content_area.abs_diff_eq(content_area, TOLERANCE)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CachedLayout {
    pub(crate) min_size: Vec2,
    pub(crate) max_size: Vec2,
    pub(crate) content_area: Vec2,
    pub(crate) block: Block,
}

impl CachedLayout {
    pub fn new(min_size: Vec2, max_size: Vec2, content_area: Vec2, block: Block) -> Self {
        Self {
            min_size,
            max_size,
            content_area,
            block,
        }
    }

    pub(crate) fn is_valid(&self, limits: LayoutLimits, content_area: Vec2) -> bool {
        self.min_size.abs_diff_eq(limits.min_size, TOLERANCE)
            // TODO: this is only applicable is
            // widgets size themselves to
            // maximum possible, maybe
            // re-evaluate
            && self.max_size.abs_diff_eq(limits.max_size, TOLERANCE)
            && self.content_area.abs_diff_eq(content_area, TOLERANCE)
    }
}

pub struct LayoutCache {
    pub(crate) query: LruCache<QueryKey, CachedQuery>,
    pub(crate) layout: Option<CachedLayout>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            query: LruCache::new(NonZeroUsize::new(64).unwrap()),
            layout: None,
        }
    }

    pub fn invalidate(&mut self) {
        self.query.clear();
        self.layout = None;
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
