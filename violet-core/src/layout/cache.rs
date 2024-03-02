use std::num::NonZeroUsize;

use flax::{component, components::child_of, Entity, FetchExt, RelationExt, World};
use glam::{IVec2, Vec2};
use lru::LruCache;

use super::{flow::Row, Block, Direction, LayoutLimits, Sizing};

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
    pub(crate) query: [Option<CachedValue<Sizing>>; 2],
    pub(crate) query_row: Option<CachedValue<Row>>,
    pub(crate) layout: Option<CachedValue<Block>>,
    on_invalidated: Option<Box<dyn Fn(LayoutUpdate) + Send + Sync>>,
}

impl LayoutCache {
    pub fn new(on_invalidated: Option<Box<dyn Fn(LayoutUpdate) + Send + Sync>>) -> Self {
        Self {
            query: Default::default(),
            query_row: None,
            layout: None,
            on_invalidated,
        }
    }

    pub fn invalidate(&mut self) {
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdate::Explicit)
        }

        self.query = Default::default();
        self.query_row = None;
        self.layout = None;
    }

    pub(crate) fn insert_query(&mut self, direction: Direction, value: CachedValue<Sizing>) {
        self.query[direction as usize] = Some(value);
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdate::SizeQueryUpdate)
        }
    }

    pub(crate) fn insert_query_row(&mut self, value: CachedValue<Row>) {
        self.query_row = Some(value);
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

pub(crate) fn validate_cached_query(
    cache: &CachedValue<Sizing>,
    limits: LayoutLimits,
    content_area: Vec2,
) -> bool {
    let value = &cache.value;

    let min_size = value.min.size();
    let preferred_size = value.preferred.size();

    tracing::debug!( ?preferred_size, %cache.limits.max_size, %limits.max_size, "validate_cached_query");

    min_size.x >= limits.min_size.x - TOLERANCE
        && min_size.y >= limits.min_size.y - TOLERANCE
        // Min may be larger than preferred for the orthogonal optimization direction
        && min_size.x <= limits.max_size.x + TOLERANCE
        && min_size.y <= limits.max_size.y + TOLERANCE
        && preferred_size.x <= limits.max_size.x + TOLERANCE
        && preferred_size.y <= limits.max_size.y + TOLERANCE
        && ((cache.limits.max_size - preferred_size).abs().min_element() > TOLERANCE || cache.limits.max_size.abs_diff_eq(limits.max_size, TOLERANCE))
        && cache.content_area.abs_diff_eq(content_area, TOLERANCE)
}

pub(crate) fn validate_cached_layout(
    cache: &CachedValue<Block>,
    limits: LayoutLimits,
    content_area: Vec2,
) -> bool {
    let value = &cache.value;

    let size = value.rect.size();

    tracing::debug!( ?size, %cache.limits.max_size, %limits.max_size, "validate_cached_layout");

    size.x >= limits.min_size.x - TOLERANCE
        && size.y >= limits.min_size.y - TOLERANCE
        // Min may be larger than preferred for the orthogonal optimization direction
        && size.x <= limits.max_size.x + TOLERANCE
        && size.y <= limits.max_size.y + TOLERANCE
        // && ((cache.limits.max_size - size).abs().min_element() > TOLERANCE || cache.limits.max_size.abs_diff_eq(limits.max_size, TOLERANCE))
        && cache.content_area.abs_diff_eq(content_area, TOLERANCE)
}

pub(crate) fn validate_cached_row(
    cache: &CachedValue<Row>,
    limits: LayoutLimits,
    content_area: Vec2,
) -> bool {
    let value = &cache.value;

    let min_size = value.min.size();
    let preferred_size = value.preferred.size();

    tracing::debug!( ?preferred_size, %cache.limits.max_size, %limits.max_size, "validate_cached_row");

    min_size.x >= limits.min_size.x - TOLERANCE
        && min_size.y >= limits.min_size.y - TOLERANCE
        // Min may be larger than preferred for the orthogonal optimization direction
        && min_size.x <= limits.max_size.x + TOLERANCE
        && min_size.y <= limits.max_size.y + TOLERANCE
        && preferred_size.x <= limits.max_size.x + TOLERANCE
        && preferred_size.y <= limits.max_size.y + TOLERANCE
        && ((cache.limits.max_size - preferred_size).abs().min_element() > TOLERANCE || cache.limits.max_size.abs_diff_eq(limits.max_size, TOLERANCE))
        && cache.content_area.abs_diff_eq(content_area, TOLERANCE)
}

component! {
    pub(crate) layout_cache: LayoutCache,
}
