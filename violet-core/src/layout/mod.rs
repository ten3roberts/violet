pub mod cache;
mod float;
mod flow;
mod stack;

use std::fmt::{Display, Formatter};

use flax::{Component, ComponentMut, Entity, EntityRef, Fetch, FetchExt, Opt, World};
pub use float::FloatLayout;
pub use flow::{Align, FlowLayout};
use glam::{vec2, BVec2, Vec2};
pub use stack::StackLayout;

use self::cache::{layout_cache, LayoutCache};
use crate::{
    components::{
        self, anchor, aspect_ratio, children, layout, max_size, maximize, min_size, offset,
        padding, size_resolver,
    },
    layout::cache::{validate_cached_layout, validate_cached_query, CachedValue},
    unit::Unit,
    Edges, Rect,
};

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

pub(crate) struct ApplyLayoutArgs<'a> {
    cache: &'a mut LayoutCache,
    children: &'a [Entity],
    content_area: Vec2,
    limits: LayoutLimits,
    desired_size: Vec2,
    offset: Vec2,
}

#[derive(Debug, Clone)]
pub enum Layout {
    Stack(StackLayout),
    Flow(FlowLayout),
    Float(FloatLayout),
}

impl Layout {
    pub(crate) fn apply(
        &self,
        world: &World,
        entity: &EntityRef,
        ctx: ApplyLayoutArgs,
    ) -> LayoutBlock {
        match self {
            Layout::Stack(v) => v.apply(world, entity, ctx),
            Layout::Flow(v) => v.apply(world, entity, ctx),
            Layout::Float(v) => v.apply(world, entity, ctx),
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
            Layout::Float(v) => v.query_size(world, children, args, preferred_size),
        }
    }
}

/// Arguments for querying the possible layout and size of a widget
#[derive(Debug, Clone, Copy)]
pub struct QueryArgs {
    /// Enforce limits on the layout
    pub limits: LayoutLimits,

    /// The size of the potentially available space for the subtree
    pub content_area: Vec2,
    /// The direction in which the layout is being queried
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy)]
pub struct Sizing {
    min: Rect,
    desired: Rect,
    margin: Edges,
    pub hints: SizingHints,
    maximize: Vec2,
}

impl Sizing {
    pub fn preferred(&self) -> Rect {
        self.desired
    }
}

impl Display for Sizing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "min: {}, preferred: {}, margin: {}",
            self.min.size(),
            self.desired.size(),
            self.margin
        )
    }
}

/// Constraints for a child widget passed down from the parent.
///
/// Allows for the parent to control the size of the children, such as stretching
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutLimits {
    pub min_size: Vec2,
    pub max_size: Vec2,
}

impl Default for LayoutLimits {
    fn default() -> Self {
        Self {
            min_size: Vec2::ZERO,
            max_size: Vec2::MAX,
        }
    }
}

impl Display for LayoutLimits {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "min: {}, max: {}", self.min_size, self.max_size)
    }
}

/// A block is a rectangle and surrounding support such as margin
#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutBlock {
    pub(crate) rect: Rect,
    pub(crate) margin: Edges,
    /// See: [`SizingHints::can_grow`]
    pub can_grow: BVec2,
    pub maximize: Vec2,
}

impl LayoutBlock {
    pub(crate) fn new(rect: Rect, margin: Edges, can_grow: BVec2, maximize: Vec2) -> Self {
        Self {
            rect,
            margin,
            can_grow,
            maximize,
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

#[derive(Fetch)]
struct LayoutQueryOptions {
    layout_cache: ComponentMut<LayoutCache>,
    margin: Component<Edges>,
    padding: Component<Edges>,
    min_size: Component<Unit<Vec2>>,
    max_size: Component<Unit<Vec2>>,
    desired_size: Component<Unit<Vec2>>,
    size_resolver: Opt<ComponentMut<Box<dyn SizeResolver>>>,
    children: Opt<Component<Vec<Entity>>>,
    layout: Opt<Component<Layout>>,
}

impl LayoutQueryOptions {
    fn new() -> Self {
        Self {
            layout_cache: layout_cache().as_mut(),
            margin: components::margin(),
            padding: padding(),
            min_size: min_size(),
            max_size: max_size(),
            desired_size: components::size(),
            //
            size_resolver: size_resolver().as_mut().opt(),
            children: children().opt(),
            layout: layout().opt(),
        }
    }
}

pub(crate) fn query_layout_size(world: &World, entity: &EntityRef, args: QueryArgs) -> Sizing {
    puffin::profile_function!(format!("{entity} {args:?}"));

    let query = LayoutQueryOptions::new();
    let mut query = entity.query(&query);
    let mut query = query
        .get()
        .expect("Missing items on widget for layout query");

    let fixed_boundary_size = query.min_size.is_relative() | query.max_size.is_relative();

    let min_size_px = query.min_size.resolve(args.content_area);
    let max_size_px = query.max_size.resolve(args.content_area);

    let mut limits = LayoutLimits {
        // Minimum size is *always* respected, even if that entails overflowing
        min_size: args.limits.min_size.max(min_size_px),
        max_size: args.limits.max_size.clamp(min_size_px, max_size_px),
    };

    // Check if cache is valid
    for cached in query.layout_cache.get_query(args.direction) {
        if validate_cached_query(cached, limits, args.content_area) {
            return cached.value;
        }
    }

    let children = query.children.map(Vec::as_slice).unwrap_or(&[]);

    let desired_size_px = query.desired_size.resolve(args.content_area);

    let maximized = entity.get_copy(maximize()).unwrap_or_default();
    let mut hints = SizingHints {
        relative_size: fixed_boundary_size | query.desired_size.is_relative(),
        can_grow: BVec2::new(
            desired_size_px.x > args.limits.max_size.x,
            desired_size_px.y > args.limits.max_size.y,
        ) | maximized.cmpgt(Vec2::ZERO),
        coupled_size: false,
    };

    // if hints != Default::default() {
    // tracing::debug!(%entity, ?resolved_size, ?external_max_size, "can grow");
    // }

    // Clamp max size here since we ensure it is > min_size
    let clamped_size_px = desired_size_px.clamp(limits.min_size, limits.max_size);

    // Flow
    let mut sizing = if let Some(layout) = query.layout {
        // Account for padding eating into the max size
        let padded_min_size = (limits.min_size - query.padding.size()).max(Vec2::ZERO);
        let padded_max_size = (limits.max_size - query.padding.size()).max(Vec2::ZERO);

        let sizing = layout.query_size(
            world,
            query.layout_cache,
            children,
            QueryArgs {
                limits: LayoutLimits {
                    min_size: padded_min_size,
                    max_size: padded_max_size,
                },
                content_area: args.content_area - query.padding.size(),
                direction: args.direction,
            },
            // Content area
            clamped_size_px - query.padding.size(),
        );

        Sizing {
            // Allow padding to accommodate the margin, and only return what margin is outside
            //
            // Then combine it with the widgets own margin
            margin: (sizing.margin - *query.padding).max(*query.margin),
            min: sizing.min.pad(*query.padding),
            desired: sizing.desired.pad(*query.padding),
            hints: sizing.hints.combine(hints),
            maximize: sizing.maximize + entity.get_copy(maximize()).unwrap_or_default(),
        }
    }
    // Allow single children to pass through directly, without need for layout
    else if let [child] = children {
        let child = world.entity(*child).unwrap();
        query_layout_size(world, &child, args)
    }
    // Customizable size resolution, such as querying text size
    else if let Some(size_resolver) = &mut query.size_resolver {
        // Handle leaf nodes with dynamic size resolution
        let (min_size, intrinsic_size, intrinsic_hints) = size_resolver.query_size(entity, args);

        // If intrinsic_min_size > max_size we overflow, but respect the minimum size nonetheless
        limits.min_size = limits.min_size.max(min_size);

        let intrinsic_size = intrinsic_size.max(clamped_size_px);

        Sizing {
            min: Rect::from_size(min_size),
            desired: Rect::from_size(intrinsic_size),
            margin: *query.margin,
            hints: intrinsic_hints.combine(hints),
            maximize: entity.get_copy(maximize()).unwrap_or_default(),
        }
    }
    // Default property based sizing
    else {
        Sizing {
            min: Rect::from_size(min_size_px),
            desired: Rect::from_size(clamped_size_px),
            margin: *query.margin,
            hints: hints,
            maximize: entity.get_copy(maximize()).unwrap_or_default(),
        }
    };

    // Enforce additional constraints over the final size, such as aspect ratio
    constrain_sizing(
        &mut sizing,
        entity,
        args.content_area,
        &mut query,
        args,
        limits,
    );

    sizing
}

// Function to apply constraints and translate sizing
fn constrain_sizing(
    sizing: &mut Sizing,
    entity: &EntityRef,
    content_area: Vec2,
    query: &mut LayoutQueryOptionsItem<'_>,
    args: QueryArgs,
    limits: LayoutLimits,
) {
    let constraints = Constraints::from_entity(entity);

    if constraints.aspect_ratio.is_some() {
        sizing.hints.coupled_size = true;
    }

    sizing.min = sizing.min.with_size(constraints.apply(sizing.min.size()));
    sizing.desired = sizing
        .desired
        .with_size(constraints.apply(sizing.desired.size()));

    let min_offset = resolve_pos(entity, content_area, sizing.min.size());
    let offset = resolve_pos(entity, content_area, sizing.desired.size());

    sizing.min = sizing.min.translate(min_offset);
    sizing.desired = sizing.desired.translate(offset);

    if IGNORE_ZERO_SIZE_MARGINS
        && sizing.desired.size() == Vec2::ZERO
        && sizing.maximize == Vec2::ZERO
    {
        sizing.margin = Edges::ZERO
    }

    query.layout_cache.insert_query_result(
        args.direction,
        CachedValue::new(limits, args.content_area, *sizing),
    );
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutArgs {
    /// Enforce limits on the layout
    pub limits: LayoutLimits,
    // The size of the potentially available space for the subtree
    pub content_area: Vec2,
}

impl Default for LayoutArgs {
    fn default() -> Self {
        Self {
            content_area: Vec2::ZERO,
            limits: LayoutLimits::default(),
        }
    }
}

const IGNORE_ZERO_SIZE_MARGINS: bool = true;

/// Updates the layout of the given subtree given the passes constraints.
///
/// Returns the outer bounds of the subtree.
#[must_use = "This function does not mutate the entity"]
pub(crate) fn apply_layout(world: &World, entity: &EntityRef, args: LayoutArgs) -> LayoutBlock {
    puffin::profile_function!(format!("{entity}"));

    let query = LayoutQueryOptions::new();
    let mut query = entity.query(&query);
    let query = query.get().expect("Missing items on widget for layout");

    let min_size_px = query.min_size.resolve(args.content_area);
    let max_size_px = query.max_size.resolve(args.content_area);

    let limits = LayoutLimits {
        // Minimum size is *always* respected, even if that entails overflowing
        min_size: args.limits.min_size.max(min_size_px),
        max_size: args.limits.max_size.clamp(min_size_px, max_size_px),
    };

    // Check if cache is still valid

    if let Some(value) = &query.layout_cache.layout {
        if validate_cached_layout(
            value,
            limits,
            args.content_area,
            query.layout_cache.hints.relative_size,
        ) {
            tracing::debug!(%entity, %value.value.rect, %value.value.can_grow, %args.limits, "found valid cached layout");

            return value.value;
        }
    }

    let children = query.children.map(Vec::as_slice).unwrap_or(&[]);

    let mut resolved_size = query.desired_size.resolve(args.content_area);

    let maximized = entity.get_copy(maximize()).unwrap_or_default();

    // Use all the size we can
    if maximized.x > 0.0 {
        resolved_size.x = limits.max_size.x;
    }

    if maximized.y > 0.0 {
        resolved_size.y = limits.max_size.y;
    }

    let can_maximize = maximized.cmpgt(Vec2::ZERO);

    let can_grow = BVec2::new(
        resolved_size.x > args.limits.max_size.x,
        resolved_size.y > args.limits.max_size.y,
    ) | can_maximize;

    let clamped_size_px = resolved_size.clamp(limits.min_size, limits.max_size);

    let mut block = if let Some(layout) = query.layout {
        let padded_min_size = (limits.min_size - query.padding.size()).max(Vec2::ZERO);
        let padded_max_size = (limits.max_size - query.padding.size()).max(Vec2::ZERO);

        let block = layout.apply(
            world,
            entity,
            ApplyLayoutArgs {
                cache: query.layout_cache,
                children,
                content_area: args.content_area,
                limits: LayoutLimits {
                    min_size: padded_min_size,
                    max_size: padded_max_size,
                },
                desired_size: clamped_size_px - query.padding.size(),
                // start of inner content
                offset: vec2(query.padding.left, query.padding.top),
            },
        );

        LayoutBlock {
            rect: block.rect.pad(*query.padding),
            margin: (block.margin - *query.padding).max(*query.margin),
            can_grow: block.can_grow | can_grow,
            maximize: (block.maximize + maximized).min(Vec2::ONE),
        }
    } else if let [child] = children {
        let child = world.entity(*child).unwrap();
        let block = apply_layout(world, &child, args);

        child.update_dedup(components::rect(), block.rect);
        block
    } else if let Some(size_resolver) = query.size_resolver {
        assert_eq!(children, [], "Widget with children must have a layout");
        // Handle leaf nodes with dynamic size resolution
        let (intrinsic_size, instrinsic_can_grow) = size_resolver.apply_layout(entity, args);

        let intrinsic_size = intrinsic_size.max(clamped_size_px);

        LayoutBlock {
            rect: Rect::from_size(intrinsic_size),
            margin: *query.margin,
            can_grow: instrinsic_can_grow | can_grow,
            maximize: maximized,
        }
    } else {
        assert_eq!(children, [], "Widget with children must have a layout");

        LayoutBlock {
            rect: Rect::from_size(clamped_size_px),
            margin: *query.margin,
            can_grow: can_grow,
            maximize: maximized,
        }
    };

    let constraints = Constraints::from_entity(entity);
    block.rect = block.rect.with_size(constraints.apply(block.rect.size()));

    let offset = resolve_pos(entity, args.content_area, block.rect.size());
    block.rect = block.rect.translate(offset);

    if IGNORE_ZERO_SIZE_MARGINS && block.rect.size() == Vec2::ZERO && block.maximize == Vec2::ZERO {
        block.margin = Edges::ZERO
    }

    entity.update_dedup(components::layout_bounds(), block.rect.size());
    entity
        .update_dedup(components::layout_args(), args)
        .unwrap();

    query
        .layout_cache
        .insert_layout(CachedValue::new(limits, args.content_area, block));

    block
}

/// Used to resolve dynamically determined sizes of widgets. This is most commonly used for text
/// elements or other widgets whose size depends on the current sizing limits.
pub trait SizeResolver: Send + Sync {
    /// Query the size of the widget given the current constraints
    ///
    /// Returns a minimum possible size optimized for the given direction, and the preferred
    /// size
    fn query_size(&mut self, entity: &EntityRef, args: QueryArgs) -> (Vec2, Vec2, SizingHints);

    /// Uses the current constraints to determine the size of the widget
    fn apply_layout(&mut self, entity: &EntityRef, args: LayoutArgs) -> (Vec2, BVec2);
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
