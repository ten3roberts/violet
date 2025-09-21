use flax::{component, components::child_of, Entity, FetchExt, RelationExt, World};
use glam::{BVec2, Vec2};

use super::{flow::Row, Direction, LayoutBlock, LayoutLimits, Sizing, SizingHints};

#[derive(Debug)]
pub struct CachedValue<T> {
    pub(crate) limits: LayoutLimits,
    pub(crate) content_area: Vec2,
    pub value: T,
}

const ENABLE: bool = true;
pub const LAYOUT_TOLERANCE: f32 = 0.01;

impl<T> CachedValue<T> {
    pub(crate) fn new(limits: LayoutLimits, content_area: Vec2, value: T) -> Self {
        Self {
            limits,
            content_area,
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayoutUpdateEvent {
    SizeQueryUpdate,
    LayoutUpdate,
    Explicit,
}

pub struct LayoutCache {
    pub(crate) query: [Vec<CachedValue<Sizing>>; 2],
    pub(crate) query_row: Option<CachedValue<Row>>,
    pub(crate) layout: Option<CachedValue<LayoutBlock>>,
    on_invalidated: Option<Box<dyn Fn(LayoutUpdateEvent) + Send + Sync>>,
    pub(crate) hints: SizingHints,
}

impl LayoutCache {
    pub fn new(on_invalidated: Option<Box<dyn Fn(LayoutUpdateEvent) + Send + Sync>>) -> Self {
        Self {
            query: Default::default(),
            query_row: None,
            layout: None,
            on_invalidated,
            hints: Default::default(),
        }
    }

    pub fn invalidate(&mut self) {
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdateEvent::Explicit)
        }

        self.query = Default::default();
        self.query_row = None;
        self.layout = None;
    }

    pub(crate) fn insert_query_result(&mut self, direction: Direction, value: CachedValue<Sizing>) {
        if !ENABLE {
            return;
        }

        self.hints = value.value.hints;
        let v = &mut self.query[direction as usize];
        if v.len() >= 16 {
            v.pop();
        }

        v.insert(0, value);

        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdateEvent::SizeQueryUpdate)
        }
    }

    pub(crate) fn insert_query_row(&mut self, value: CachedValue<Row>) {
        if !ENABLE {
            return;
        }

        self.query_row = Some(value);
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdateEvent::SizeQueryUpdate)
        }
    }

    pub(crate) fn insert_layout(&mut self, value: CachedValue<LayoutBlock>) {
        if !ENABLE {
            return;
        }

        self.layout = Some(value);
        if let Some(f) = self.on_invalidated.as_ref() {
            f(LayoutUpdateEvent::LayoutUpdate)
        }
    }

    pub fn layout(&self) -> Option<&CachedValue<LayoutBlock>> {
        self.layout.as_ref()
    }

    pub fn get_query(&self, direction: Direction) -> &[CachedValue<Sizing>] {
        self.query[direction as usize].as_ref()
    }

    pub fn hints(&self) -> SizingHints {
        self.hints
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
    let preferred_size = value.desired.size();

    // tracing::debug!( ?preferred_size, %cache.limits.max_size, %limits.max_size, "validate_cached_query");

    let hints = &value.hints;
    #[allow(clippy::nonminimal_bool)]
    if hints.can_grow.x && cache.limits.max_size.x < limits.max_size.x
        || (hints.can_grow.x && cache.limits.max_size.y < limits.max_size.y)
    {
        // tracing::info!(%hints.can_grow, ?cache.limits.max_size, %limits.max_size, "invalidated by can_grow");
    }

    min_size.x >= limits.min_size.x - LAYOUT_TOLERANCE
        && min_size.y >= limits.min_size.y - LAYOUT_TOLERANCE
        // Min may be larger than preferred for the orthogonal optimization direction
        && min_size.x <= limits.max_size.x + LAYOUT_TOLERANCE
        && min_size.y <= limits.max_size.y + LAYOUT_TOLERANCE
        && preferred_size.x <= limits.max_size.x + LAYOUT_TOLERANCE
        && preferred_size.y <= limits.max_size.y + LAYOUT_TOLERANCE
        && (!hints.can_grow.x || cache.limits.max_size.x >= limits.max_size.x - LAYOUT_TOLERANCE)
        && (!hints.can_grow.y || cache.limits.max_size.y >= limits.max_size.y - LAYOUT_TOLERANCE)
        && (!hints.relative_size.x || (cache.content_area.x - content_area.x).abs() < LAYOUT_TOLERANCE)
        && (!hints.relative_size.y || (cache.content_area.y - content_area.y).abs() < LAYOUT_TOLERANCE)
}

pub(crate) fn validate_cached_layout(
    cache: &CachedValue<LayoutBlock>,
    limits: LayoutLimits,
    content_area: Vec2,
    // Calculated from the query stage
    relative_size: BVec2,
) -> bool {
    let value = &cache.value;

    let size = value.rect.size().min(cache.limits.max_size);

    size.x >= limits.min_size.x - LAYOUT_TOLERANCE
        && size.y >= limits.min_size.y - LAYOUT_TOLERANCE
        // Min may be larger than preferred for the orthogonal optimization direction
        && size.x <= limits.max_size.x + LAYOUT_TOLERANCE
        && size.y <= limits.max_size.y + LAYOUT_TOLERANCE
        && (!value.can_grow.x || cache.limits.max_size.x >= limits.max_size.x - LAYOUT_TOLERANCE)
        && (!value.can_grow.y || cache.limits.max_size.y >= limits.max_size.y - LAYOUT_TOLERANCE)
        && (!relative_size.x || (cache.content_area.x - content_area.x).abs() < LAYOUT_TOLERANCE)
        && (!relative_size.y || (cache.content_area.y - content_area.y).abs() < LAYOUT_TOLERANCE)
}

pub(crate) fn validate_cached_row(
    cache: &CachedValue<Row>,
    limits: LayoutLimits,
    content_area: Vec2,
) -> bool {
    let value = &cache.value;

    let min_size = value.min.size();
    let preferred_size = value.preferred.size();
    let hints = value.hints;

    // tracing::debug!( ?preferred_size, %cache.limits.max_size, %limits.max_size, "validate_cached_row");

    min_size.x >= limits.min_size.x - LAYOUT_TOLERANCE
        && min_size.y >= limits.min_size.y - LAYOUT_TOLERANCE
        // Min may be larger than preferred for the orthogonal optimization direction
        && min_size.x <= limits.max_size.x + LAYOUT_TOLERANCE
        && min_size.y <= limits.max_size.y + LAYOUT_TOLERANCE
        && preferred_size.x <= limits.max_size.x + LAYOUT_TOLERANCE
        && preferred_size.y <= limits.max_size.y + LAYOUT_TOLERANCE
        && (!hints.can_grow.x || cache.limits.max_size.x >= limits.max_size.x - LAYOUT_TOLERANCE)
        && (!hints.can_grow.y || cache.limits.max_size.y >= limits.max_size.y - LAYOUT_TOLERANCE)
        && (!hints.relative_size.x || (cache.content_area.x - content_area.x).abs() < LAYOUT_TOLERANCE)
        && (!hints.relative_size.y || (cache.content_area.y - content_area.y).abs() < LAYOUT_TOLERANCE)
}

component! {
    pub layout_cache: LayoutCache,
}
