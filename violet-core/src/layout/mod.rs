pub mod cache;
mod flow;
mod stack;

use std::fmt::{Display, Formatter};

use flax::{Entity, EntityRef, FetchExt, World};
use glam::{vec2, BVec2, Vec2};

use crate::{
    components::{
        self, anchor, aspect_ratio, children, layout, max_size, maximize, min_size, offset,
        padding, size, size_resolver,
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
    Horizontal = 0,
    Vertical = 1,
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

    /// Returns `true` if the direction is [`Horizontal`].
    ///
    /// [`Horizontal`]: Direction::Horizontal
    #[must_use]
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Horizontal)
    }

    /// Returns `true` if the direction is [`Vertical`].
    ///
    /// [`Vertical`]: Direction::Vertical
    #[must_use]
    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Vertical)
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
        preferred_size: Vec2,
    ) -> Block {
        match self {
            Layout::Stack(v) => v.apply(
                world,
                entity,
                children,
                content_area,
                limits,
                preferred_size,
            ),
            Layout::Flow(v) => v.apply(
                world,
                entity,
                cache,
                children,
                content_area,
                limits,
                preferred_size,
            ),
        }
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        cache: &mut LayoutCache,
        children: &[Entity],
        args: QueryArgs,
        preferred_size: Vec2,
    ) -> Sizing {
        match self {
            Layout::Stack(v) => v.query_size(world, children, args, preferred_size),
            Layout::Flow(v) => v.query_size(world, cache, children, args, preferred_size),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QueryArgs {
    pub limits: LayoutLimits,
    pub content_area: Vec2,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy)]
pub struct Sizing {
    min: Rect,
    preferred: Rect,
    margin: Edges,
    pub hints: SizingHints,
    maximize: Vec2,
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

impl Display for LayoutLimits {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "min: {}, max: {}", self.min_size, self.max_size)
    }
}

/// A block is a rectangle and surrounding support such as margin
#[derive(Debug, Clone, Copy, Default)]
pub struct Block {
    pub(crate) rect: Rect,
    pub(crate) margin: Edges,
    /// See: [SizingHints::can_grow]
    pub can_grow: BVec2,
}

impl Block {
    pub(crate) fn new(rect: Rect, margin: Edges, can_grow: BVec2) -> Self {
        Self {
            rect,
            margin,
            can_grow,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizingHints {
    /// Size does not depend on the size of the parent
    pub can_grow: BVec2,
    /// The widget size is clamped given the provided size limits, and could be larger.
    ///
    /// If this is true, giving *more* space to a widget may cause it to grow.
    ///
    /// This is used for an optimization to avoid invalidating the layout when the available size
    /// increases
    pub relative_size: BVec2,
    /// Changes to width affect the height and vice versa.
    ///
    /// This is used to optimize the layout query as not full distribution queries are needed
    pub coupled_size: bool,
}

impl Default for SizingHints {
    fn default() -> Self {
        Self {
            can_grow: BVec2::FALSE,
            relative_size: BVec2::FALSE,
            coupled_size: false,
        }
    }
}

impl SizingHints {
    pub fn combine(self, other: Self) -> Self {
        Self {
            can_grow: self.can_grow | other.can_grow,
            relative_size: self.relative_size | other.relative_size,
            coupled_size: self.coupled_size | other.coupled_size,
        }
    }
}

pub(crate) fn query_size(world: &World, entity: &EntityRef, args: QueryArgs) -> Sizing {
    puffin::profile_function!(format!("{entity} {args:?}"));
    // assert!(limits.min_size.x <= limits.max_size.x);
    // assert!(limits.min_size.y <= limits.max_size.y);
    let _span =
        tracing::debug_span!("query_size", name=entity.name().as_deref(), ?args.limits, %args.content_area)
            .entered();

    // tracing::info!(name=entity.name().as_deref(), ?limits, %content_area, ?direction, "query_size");
    let query = (
        layout_cache().as_mut(),
        components::margin().opt_or_default(),
        padding().opt_or_default(),
        min_size().opt_or_default(),
        max_size().opt(),
        size().opt_or_default(),
        size_resolver().as_mut().opt(),
        children().opt(),
        layout().opt(),
    );

    let mut query = entity.query(&query);
    let (cache, &margin, &padding, min_size, max_size, size, size_resolver, children, layout) =
        query.get().unwrap();

    let fixed_boundary_size =
        min_size.is_relative() | max_size.map(|v| v.is_relative()).unwrap_or(BVec2::FALSE);

    let min_size = min_size.resolve(args.content_area);
    let max_size = max_size
        .map(|v| v.resolve(args.content_area))
        .unwrap_or(Vec2::INFINITY);

    let mut limits = LayoutLimits {
        // Minimum size is *always* respected, even if that entails overflowing
        min_size: args.limits.min_size.max(min_size),
        max_size: args.limits.max_size.min(max_size),
    };

    // Check if cache is valid
    if let Some(cache) = cache.get_query(args.direction) {
        if validate_cached_query(cache, limits, args.content_area) {
            return cache.value;
        }
    }

    // tracing::info!(%entity, "query cache miss");

    // assert!(limits.min_size.x <= limits.max_size.x);
    // assert!(limits.min_size.y <= limits.max_size.y);

    let children = children.map(Vec::as_slice).unwrap_or(&[]);

    let resolved_size = size.resolve(args.content_area);

    let maximized = entity.get_copy(maximize()).unwrap_or_default();
    let mut hints = SizingHints {
        relative_size: fixed_boundary_size | size.is_relative(),
        can_grow: BVec2::new(
            resolved_size.x > args.limits.max_size.x,
            resolved_size.y > args.limits.max_size.y,
        ) | maximized.cmpgt(Vec2::ZERO),
        coupled_size: false,
    };

    // if hints != Default::default() {
    // tracing::info!(%entity, ?resolved_size, ?external_max_size, "can grow");
    // }

    // Clamp max size here since we ensure it is > min_size
    let resolved_size = resolved_size.clamp(limits.min_size, limits.max_size);

    // Flow
    let mut sizing = if let Some(layout) = layout {
        let sizing = layout.query_size(
            world,
            cache,
            children,
            QueryArgs {
                limits: LayoutLimits {
                    min_size: (limits.min_size - padding.size()).max(Vec2::ZERO),
                    max_size: (limits.max_size - padding.size()).max(Vec2::ZERO),
                },
                content_area: args.content_area - padding.size(),
                ..args
            },
            resolved_size - padding.size(),
        );

        Sizing {
            margin: (sizing.margin).max(margin),
            min: sizing.min.pad(&padding),
            preferred: sizing.preferred.pad(&padding),
            hints: sizing.hints.combine(hints),
            maximize: sizing.maximize + entity.get_copy(maximize()).unwrap_or_default(),
        }
    } else if let [child] = children {
        let child = world.entity(*child).unwrap();
        query_size(world, &child, args)
    } else {
        let (instrisic_min_size, intrinsic_size, intrinsic_hints) = size_resolver
            .map(|v| v.query(entity, args))
            .unwrap_or((Vec2::ZERO, Vec2::ZERO, SizingHints::default()));

        // If intrinsic_min_size > max_size we overflow, but respect the minimum size nonetheless
        limits.min_size = limits.min_size.max(instrisic_min_size);

        let size = intrinsic_size.max(resolved_size);

        let min_size = instrisic_min_size.max(limits.min_size);

        Sizing {
            min: Rect::from_size(min_size),
            preferred: Rect::from_size(size),
            margin,
            hints: intrinsic_hints.combine(hints),
            maximize: entity.get_copy(maximize()).unwrap_or_default(),
        }
    };

    let constraints = Constraints::from_entity(entity);

    if constraints.aspect_ratio.is_some() {
        hints.coupled_size = true;
    }

    sizing.min = sizing.min.with_size(constraints.apply(sizing.min.size()));
    sizing.preferred = sizing
        .preferred
        .with_size(constraints.apply(sizing.preferred.size()));

    let min_offset = resolve_pos(entity, args.content_area, sizing.min.size());
    let offset = resolve_pos(entity, args.content_area, sizing.preferred.size());

    sizing.min = sizing.min.translate(min_offset);
    sizing.preferred = sizing.preferred.translate(offset);

    // // Widget size is limited by itself and is not affected by the size of the parent
    // if let Some(max_size) = max_size {
    //     if sizing
    //         .preferred
    //         .size()
    //         .abs_diff_eq(max_size, LAYOUT_TOLERANCE)
    //     {
    //         sizing.hints.can_grow = false;
    //     }
    // }

    // validate_sizing(entity, &sizing, limits);

    cache.insert_query(
        args.direction,
        CachedValue::new(limits, args.content_area, sizing),
    );

    sizing
}

/// Updates the layout of the given subtree given the passes constraints.
///
/// Returns the outer bounds of the subtree.
#[must_use = "This function does not mutate the entity"]
pub(crate) fn apply_layout(
    world: &World,
    entity: &EntityRef,
    // The size of the potentially available space for the subtree
    content_area: Vec2,
    limits: LayoutLimits,
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
        components::size().opt_or_default(),
        size_resolver().as_mut().opt(),
        children().opt(),
        layout().opt(),
    );

    let mut query = entity.query(&query);
    let (cache, &margin, &padding, min_size, max_size, size, size_resolver, children, layout) =
        query.get().unwrap();

    let min_size = min_size.resolve(content_area);
    let max_size = max_size
        .map(|v| v.resolve(content_area))
        .unwrap_or(Vec2::INFINITY);

    let external_limits = limits;
    let limits = LayoutLimits {
        // Minimum size is *always* respected, even if that entails overflowing
        min_size: limits.min_size.max(min_size),
        max_size: limits.max_size.min(max_size),
    };

    // Check if cache is still valid

    if let Some(value) = &cache.layout {
        if validate_cached_layout(value, limits, content_area, cache.hints.relative_size) {
            tracing::debug!(%entity, %value.value.rect, %value.value.can_grow, "found valid cached layout");

            return value.value;
        }
    }

    // tracing::info!(%entity, ?cache.layout, "layout cache miss");

    // limits.min_size = limits.min_size.min(limits.max_size);

    // assert!(limits.min_size.x <= limits.max_size.x);
    // assert!(limits.min_size.y <= limits.max_size.y);

    let children = children.map(Vec::as_slice).unwrap_or(&[]);

    let mut resolved_size = size.resolve(content_area);

    let maximized = entity.get_copy(maximize()).unwrap_or_default();

    if maximized.x > 0.0 {
        resolved_size.x = limits.max_size.x;
    }

    if maximized.y > 0.0 {
        resolved_size.y = limits.max_size.y;
    }

    let can_maximize = maximized.cmpgt(Vec2::ZERO);

    let can_grow = BVec2::new(
        resolved_size.x > external_limits.max_size.x,
        resolved_size.y > external_limits.max_size.y,
    ) | can_maximize;

    // tracing::trace!(%entity, ?resolved_size, ?external_max_size, %can_grow);

    let resolved_size = resolved_size.clamp(limits.min_size, limits.max_size);

    let mut block = if let Some(layout) = layout {
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
            resolved_size - padding.size(),
        );

        block.rect = block.rect.pad(&padding);

        block.margin = (block.margin - padding).max(margin);

        block
    } else if let [child] = children {
        let child = world.entity(*child).unwrap();
        let block = apply_layout(world, &child, content_area, limits);

        child.update_dedup(components::rect(), block.rect);
        block
    } else {
        assert_eq!(children, [], "Widget with children must have a layout");

        let (intrinsic_size, instrinsic_can_grow) = size_resolver
            .map(|v| v.apply(entity, content_area, limits))
            .unwrap_or((Vec2::ZERO, BVec2::FALSE));

        let size = intrinsic_size.max(resolved_size);

        let rect = Rect::from_size(size);

        Block {
            rect,
            margin,
            can_grow: instrinsic_can_grow | can_grow,
        }
    };

    // if block.rect.size().x > limits.max_size.x || block.rect.size().y > limits.max_size.y {
    //     tracing::error!(
    //         %entity,
    //         rect_size = %block.rect.size(),
    //         %limits.max_size,
    //         "Widget size exceeds constraints",
    //     );
    //     panic!("");
    // }

    let constraints = Constraints::from_entity(entity);
    block.rect = block.rect.with_size(constraints.apply(block.rect.size()));

    let offset = resolve_pos(entity, content_area, block.rect.size());
    block.rect = block.rect.translate(offset);

    entity.update_dedup(components::layout_bounds(), block.rect.size());

    // Widget size is limited by itself and is not affected by the size of the parent
    // if let Some(max_size) = max_size {
    //     if block.rect.size().abs_diff_eq(max_size, LAYOUT_TOLERANCE) {
    //         block.can_grow = BVec2::FALSE;
    //     }
    // }

    // if block.rect.size().x > limits.max_size.x || block.rect.size().y > limits.max_size.y {
    //     tracing::error!(
    //         %entity, rect_size=%block.rect.size(), %limits.max_size,
    //         "Widget size exceeds constraints",
    //     );
    // }

    // validate_block(entity, &block, limits);

    tracing::debug!(%limits, %content_area, %block.can_grow, %block.rect, "caching layout");
    cache.insert_layout(CachedValue::new(limits, content_area, block));

    block
}

/// Used to resolve dynamically determined sizes of widgets. This is most commonly used for text
/// elements or other widgets whose size depends on the current sizing limits.
pub trait SizeResolver: Send + Sync {
    /// Query the size of the widget given the current constraints
    ///
    /// Returns a minimum possible size optimized for the given direction, and the preferred
    /// size
    fn query(&mut self, entity: &EntityRef, args: QueryArgs) -> (Vec2, Vec2, SizingHints);

    /// Uses the current constraints to determine the size of the widget
    fn apply(
        &mut self,
        entity: &EntityRef,
        content_area: Vec2,
        limits: LayoutLimits,
    ) -> (Vec2, BVec2);
}

#[derive(Debug)]
struct Constraints {
    aspect_ratio: Option<f32>,
}

impl Constraints {
    fn from_entity(entity: &EntityRef) -> Self {
        let query = (aspect_ratio().copied().opt(),);
        let mut query = entity.query(&query);
        let (aspect_ratio,) = query.get().unwrap();
        Self { aspect_ratio }
    }

    fn apply(&self, mut size: Vec2) -> Vec2 {
        if let Some(aspect_ratio) = self.aspect_ratio {
            if aspect_ratio > 0.0 {
                // > 1.0 means width > height
                if aspect_ratio > 1.0 {
                    size = vec2(size.x, size.y / aspect_ratio);
                } else {
                    size = vec2(size.y * aspect_ratio, size.y);
                }
            }
        }

        size
    }
}

/// Resolves a widgets position relative to its own bounds
fn resolve_pos(entity: &EntityRef, parent_size: Vec2, self_size: Vec2) -> Vec2 {
    let query = (offset().opt_or_default(), anchor().opt_or_default());
    let mut query = entity.query(&query);
    let (offset, anchor) = query.get().unwrap();

    let offset = offset.resolve(parent_size);

    offset - anchor.resolve(self_size)
}
