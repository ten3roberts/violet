pub mod cache;
mod flow;
mod stack;

use std::fmt::{Display, Formatter};

use flax::{Entity, EntityRef, FetchExt, World};
use glam::{vec2, Vec2};

use crate::{
    components::{
        self, anchor, aspect_ratio, children, layout, max_size, min_size, offset, padding,
    },
    layout::cache::{validate_cached_layout, validate_cached_query, CachedValue},
    Edges, Rect,
};

pub use flow::{Alignment, FlowLayout};
pub use stack::StackLayout;

use self::cache::{layout_cache, LayoutCache};

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd, Hash, Ord, Eq)]
pub enum Direction {
    #[default]
    Horizontal,
    Vertical,
}

impl Direction {
    fn as_main_and_cross(&self, reverse: bool) -> (Vec2, Vec2) {
        match (self, reverse) {
            (Direction::Horizontal, false) => (Vec2::X, Vec2::Y),
            (Direction::Vertical, false) => (Vec2::Y, Vec2::X),
            (Direction::Horizontal, true) => (-Vec2::X, Vec2::Y),
            (Direction::Vertical, true) => (-Vec2::Y, Vec2::X),
        }
    }

    fn to_edges(self, main: (f32, f32), cross: (f32, f32), reverse: bool) -> Edges {
        match (self, reverse) {
            (Direction::Horizontal, false) => Edges::new(main.0, main.1, cross.0, cross.1),
            (Direction::Vertical, false) => Edges::new(cross.0, cross.1, main.0, main.1),
            (Direction::Horizontal, true) => Edges::new(main.1, main.0, cross.0, cross.1),
            (Direction::Vertical, true) => Edges::new(cross.1, cross.0, main.0, main.1),
        }
    }

    pub fn to_axis(self) -> Vec2 {
        match self {
            Direction::Horizontal => Vec2::X,
            Direction::Vertical => Vec2::Y,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Layout {
    Stack(StackLayout),
    Flow(FlowLayout),
}

impl Layout {
    pub(crate) fn apply(
        &self,
        world: &World,
        entity: &EntityRef,
        cache: &mut LayoutCache,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        match self {
            Layout::Stack(v) => v.apply(world, entity, children, content_area, limits),
            Layout::Flow(v) => v.apply(world, entity, cache, children, content_area, limits),
        }
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        entity: &EntityRef,
        cache: &mut LayoutCache,
        children: &[Entity],
        inner_rect: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> Sizing {
        match self {
            Layout::Stack(v) => v.query_size(world, children, inner_rect, limits, squeeze),
            Layout::Flow(v) => v.query_size(world, cache, children, inner_rect, limits, squeeze),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sizing {
    min: Rect,
    preferred: Rect,
    margin: Edges,
}

impl Display for Sizing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "min: {}, preferred: {}, margin: {}",
            self.min.size(),
            self.preferred.size(),
            self.margin
        )
    }
}

/// Constraints for a child widget passed down from the parent.
///
/// Allows for the parent to control the size of the children, such as stretching
#[derive(Debug, Clone, Copy)]
pub struct LayoutLimits {
    pub min_size: Vec2,
    pub max_size: Vec2,
}

/// A block is a rectangle and surrounding support such as margin
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Block {
    pub(crate) rect: Rect,
    pub(crate) margin: Edges,
}

impl Block {
    pub(crate) fn new(rect: Rect, margin: Edges) -> Self {
        Self { rect, margin }
    }
}

fn validate_sizing(entity: &EntityRef, sizing: &Sizing, limits: LayoutLimits) {
    const TOLERANCE: f32 = 0.2;
    if sizing.min.size().x > limits.max_size.x + TOLERANCE
        || sizing.min.size().y > limits.max_size.y + TOLERANCE
    {
        tracing::error!(
            %entity,
            min_size = %sizing.min.size(),
            max_size = %limits.max_size,
            "Minimum size exceeds size limit",
        );
    }

    if sizing.preferred.size().x > limits.max_size.x + TOLERANCE
        || sizing.preferred.size().y > limits.max_size.y + TOLERANCE
    {
        tracing::error!(
            %entity,
            preferred_size = %sizing.preferred.size(),
            ?limits,
            "Preferred size exceeds size limit",
        );
    }

    if sizing.min.size().x + TOLERANCE < limits.min_size.x
        || sizing.min.size().y + TOLERANCE < limits.min_size.y
    {
        tracing::error!(
            %entity,
            min_size = %sizing.min.size(),
            ?limits,
            "Minimum size is less than size limit",
        );
    }
}

fn validate_block(entity: &EntityRef, block: &Block, limits: LayoutLimits) {
    const TOLERANCE: f32 = 0.2;
    if block.rect.size().x > limits.max_size.x + TOLERANCE
        || block.rect.size().y > limits.max_size.y + TOLERANCE
    {
        tracing::error!(
            %entity,
            rect_size = %block.rect.size(),
            max_size = %limits.max_size,
            "Widget size exceeds size limit",
        );
    }

    if block.rect.size().x + TOLERANCE < limits.min_size.x
        || block.rect.size().y + TOLERANCE < limits.min_size.y
    {
        tracing::error!(
            %entity,
            rect_size = %block.rect.size(),
            min_size = %limits.min_size,
            "Widget size is less than size limit",
        );
    }
}

pub(crate) fn query_size(
    world: &World,
    entity: &EntityRef,
    content_area: Vec2,
    mut limits: LayoutLimits,
    direction: Direction,
) -> Sizing {
    puffin::profile_function!(format!("{entity}"));
    // assert!(limits.min_size.x <= limits.max_size.x);
    // assert!(limits.min_size.y <= limits.max_size.y);
    let _span =
        tracing::debug_span!("query_size", name=entity.name().as_deref(), ?limits, %content_area)
            .entered();

    let query = (
        layout_cache().as_mut(),
        components::margin().opt_or_default(),
        padding().opt_or_default(),
        min_size().opt_or_default(),
        max_size().opt(),
        children().opt(),
        layout().opt(),
    );

    let mut query = entity.query(&query);
    let (cache, &margin, &padding, min_size, max_size, children, layout) = query.get().unwrap();

    limits.min_size = limits.min_size.max(min_size.resolve(content_area));

    if let Some(max_size) = max_size {
        limits.max_size = limits.max_size.min(max_size.resolve(content_area));
    }

    // Check if cache is valid
    if let Some(cache) = cache.query[direction as usize].as_ref() {
        if validate_cached_query(cache, limits, content_area) {
            // if cache.is_valid(limits, content_area) {
            let _span = tracing::trace_span!("cached").entered();
            validate_sizing(entity, &cache.value, limits);
            tracing::debug!(%entity, "found valid cached query");
            return cache.value;
            // }
        }
    }

    // tracing::info!(%entity, "query cache miss");

    // assert!(limits.min_size.x <= limits.max_size.x);
    // assert!(limits.min_size.y <= limits.max_size.y);

    let children = children.map(Vec::as_slice).unwrap_or(&[]);

    // Flow
    let sizing = if let Some(layout) = layout {
        let mut sizing = layout.query_size(
            world,
            entity,
            cache,
            children,
            Rect::from_size(content_area).inset(&padding),
            LayoutLimits {
                min_size: (limits.min_size - padding.size()).max(Vec2::ZERO),
                max_size: (limits.max_size - padding.size()).max(Vec2::ZERO),
            },
            direction,
        );

        sizing.margin = (sizing.margin - padding).max(margin);
        sizing.min = sizing.min.pad(&padding);
        sizing.preferred = sizing.preferred.pad(&padding);

        let min_offset = resolve_pos(entity, content_area, sizing.min.size());
        let preferred_offset = resolve_pos(entity, content_area, sizing.preferred.size());

        sizing.min = sizing.min.translate(min_offset);
        sizing.preferred = sizing.preferred.translate(preferred_offset);

        sizing
    } else {
        // Leaf
        let (min_size, preferred_size) = query_constraints(entity, content_area, limits, direction);

        let min_offset = resolve_pos(entity, content_area, min_size);
        let preferred_offset = resolve_pos(entity, content_area, preferred_size);

        Sizing {
            min: Rect::from_size_pos(min_size, min_offset),
            preferred: Rect::from_size_pos(preferred_size, preferred_offset),
            margin,
        }
    };

    validate_sizing(entity, &sizing, limits);

    tracing::debug!(%sizing);
    cache.insert_query(direction, CachedValue::new(limits, content_area, sizing));

    sizing
}

/// Updates the layout of the given subtree given the passes constraints.
///
/// Returns the outer bounds of the subtree.
#[must_use = "This function does not mutate the entity"]
pub(crate) fn update_subtree(
    world: &World,
    entity: &EntityRef,
    // The size of the potentially available space for the subtree
    content_area: Vec2,
    mut limits: LayoutLimits,
) -> Block {
    puffin::profile_function!(format!("{entity}"));
    // assert!(limits.min_size.x <= limits.max_size.x);
    // assert!(limits.min_size.y <= limits.max_size.y);
    // let _span = tracing::info_span!( "Updating subtree", %entity, ?constraints).entered();
    let _span = tracing::debug_span!("update_subtree", %entity).entered();

    let query = (
        layout_cache().as_mut(),
        components::margin().opt_or_default(),
        padding().opt_or_default(),
        min_size().opt_or_default(),
        max_size().opt(),
        children().opt(),
        layout().opt(),
    );

    let mut query = entity.query(&query);
    let (cache, &margin, &padding, min_size, max_size, children, layout) = query.get().unwrap();

    limits.min_size = limits.min_size.max(min_size.resolve(content_area));

    if let Some(max_size) = max_size {
        limits.max_size = limits.max_size.min(max_size.resolve(content_area));
    }

    // Check if cache is still valid

    if let Some(cache) = cache.layout.as_ref() {
        if validate_cached_layout(cache, limits, content_area) {
            tracing::debug!(%entity, ?cache, "found valid cached layout");
            validate_block(entity, &cache.value, limits);
            return cache.value;
        }
    }

    // tracing::info!(%entity, ?cache.layout, "layout cache miss");

    // limits.min_size = limits.min_size.min(limits.max_size);

    // assert!(limits.min_size.x <= limits.max_size.x);
    // assert!(limits.min_size.y <= limits.max_size.y);

    let children = children.map(Vec::as_slice).unwrap_or(&[]);

    let block = if let Some(layout) = layout {
        let mut block = layout.apply(
            world,
            entity,
            cache,
            children,
            Rect::from_size(content_area).inset(&padding),
            LayoutLimits {
                min_size: (limits.min_size - padding.size()).max(Vec2::ZERO),
                max_size: (limits.max_size - padding.size()).max(Vec2::ZERO),
            },
        );

        if block.rect.size().x > limits.max_size.x || block.rect.size().y > limits.max_size.y {
            tracing::error!(
                %entity, rect_size=%block.rect.size(), %limits.max_size,
                "Widget size exceeds constraints",
            );
        }

        block.rect = block.rect.pad(&padding);

        let offset = resolve_pos(entity, content_area, block.rect.size());
        block.rect = block.rect.translate(offset);

        block.margin = (block.margin - padding).max(margin);

        block
    } else {
        assert_eq!(children, [], "Widget with children must have a layout");
        let size = apply_constraints(entity, content_area, limits);

        if size.x > limits.max_size.x || size.y > limits.max_size.y {
            tracing::error!(
                %entity, %size, %limits.max_size,
                "Widget size exceeds constraints",
            );
        }

        let offset = resolve_pos(entity, content_area, size);
        let rect = Rect::from_size_pos(size, offset);

        entity.update_dedup(components::layout_bounds(), size);

        Block { rect, margin }
    };

    validate_block(entity, &block, limits);

    cache.insert_layout(CachedValue::new(limits, content_area, block));

    block
}

/// Used to resolve dynamically determined sizes of widgets. This is most commonly used for text
/// elements or other widgets whose size depends on the current sizing limits.
pub trait SizeResolver: Send + Sync {
    /// Query the size of the widget given the current constraints
    ///
    /// Returns a minimum possible size optimized for the `optimize` direction, and the preferred
    /// size
    fn query(
        &mut self,
        entity: &EntityRef,
        content_area: Vec2,
        limits: LayoutLimits,
        optimize: Direction,
    ) -> (Vec2, Vec2);

    /// Uses the current constraints to determine the size of the widget
    fn apply(&mut self, entity: &EntityRef, content_area: Vec2, limits: LayoutLimits) -> Vec2;
}

fn resolve_base_size(entity: &EntityRef, content_area: Vec2) -> (Vec2, Constraints) {
    let query = (components::size().opt_or_default(), aspect_ratio().opt());
    let mut query = entity.query(&query);
    let (size, aspect_ratio) = query.get().unwrap();

    let size = size.resolve(content_area);

    (
        size,
        Constraints {
            aspect_ratio: aspect_ratio.copied(),
        },
    )
}

#[derive(Debug)]
struct Constraints {
    aspect_ratio: Option<f32>,
}

impl Constraints {
    fn resolve(&self, mut size: Vec2) -> Vec2 {
        if let Some(aspect_ratio) = self.aspect_ratio {
            if aspect_ratio > 0.0 {
                if size.x > size.y {
                    size = vec2(size.y * aspect_ratio, size.y);
                } else {
                    size = vec2(size.x, size.x / aspect_ratio);
                }
            }
        }

        size
    }
}

fn query_constraints(
    entity: &EntityRef,
    content_area: Vec2,
    limits: LayoutLimits,
    squeeze: Direction,
) -> (Vec2, Vec2) {
    let (mut size, constraints) = resolve_base_size(entity, content_area);

    let mut min_size = limits.min_size;

    if let Ok(mut resolver) = entity.get_mut(components::size_resolver()) {
        let (resolved_min, resolved_size) = resolver.query(entity, content_area, limits, squeeze);

        let optimize_axis = squeeze.to_axis();
        if resolved_min.dot(optimize_axis) > resolved_size.dot(optimize_axis) {
            panic!("Size resolver returned a minimum size that is larger than the preferred size for the given optimization\n\nmin: {}, size: {}, widget: {}", resolved_min.dot(optimize_axis), resolved_size.dot(optimize_axis), entity);
        }

        min_size = resolved_min;
        size = resolved_size.max(size);
    }

    // tracing::info!(?min_size, ?size, ?limits, "query_constraints");

    (
        constraints.resolve(min_size.clamp(limits.min_size, limits.max_size)),
        constraints.resolve(size.clamp(limits.min_size, limits.max_size)),
    )
}

fn apply_constraints(entity: &EntityRef, content_area: Vec2, limits: LayoutLimits) -> Vec2 {
    let (mut size, constraints) = resolve_base_size(entity, content_area);

    if let Ok(mut resolver) = entity.get_mut(components::size_resolver()) {
        let resolved_size = resolver.apply(entity, content_area, limits);

        size = resolved_size.max(size);
    }

    constraints.resolve(size.clamp(limits.min_size, limits.max_size))
}

/// Resolves a widgets position relative to its own bounds
fn resolve_pos(entity: &EntityRef, parent_size: Vec2, self_size: Vec2) -> Vec2 {
    let query = (offset().opt_or_default(), anchor().opt_or_default());
    let mut query = entity.query(&query);
    let (offset, anchor) = query.get().unwrap();

    let offset = offset.resolve(parent_size);

    offset - anchor.resolve(self_size)
}
