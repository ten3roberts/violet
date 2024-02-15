mod flow;
mod stack;

use flax::{Entity, EntityRef, FetchExt, World};
use glam::{vec2, Vec2};

use crate::components::{
    self, anchor, aspect_ratio, children, layout, offset, padding, Edges, Rect,
};

pub use flow::{CrossAlign, FlowLayout};
pub use stack::StackLayout;

#[derive(Default, Debug, Clone, Copy)]
pub enum Direction {
    #[default]
    Horizontal,
    Vertical,
}

impl Direction {
    fn axis(&self, reverse: bool) -> (Vec2, Vec2) {
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
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        match self {
            Layout::Stack(v) => v.apply(world, entity, children, content_area, limits),
            Layout::Flow(v) => v.apply(world, entity, children, content_area, limits),
        }
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        children: &[Entity],
        inner_rect: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> Sizing {
        match self {
            Layout::Stack(v) => v.query_size(world, children, inner_rect, limits, squeeze),
            Layout::Flow(v) => v.query_size(world, children, inner_rect, limits, squeeze),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sizing {
    min: Rect,
    preferred: Rect,
    margin: Edges,
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

pub(crate) fn query_size(
    world: &World,
    entity: &EntityRef,
    content_area: Vec2,
    limits: LayoutLimits,
    squeeze: Direction,
) -> Sizing {
    let query = (
        components::margin().opt_or_default(),
        padding().opt_or_default(),
        children().opt(),
        layout().opt(),
    );
    let mut query = entity.query(&query);
    let (&margin, &padding, children, layout) = query.get().unwrap();

    let children = children.map(Vec::as_slice).unwrap_or(&[]);

    let (min_size, preferred_size) = query_constraints(entity, content_area, limits, squeeze);

    // Flow
    if let Some(layout) = layout {
        let sizing = layout.query_size(
            world,
            children,
            Rect::from_size(content_area).inset(&padding),
            LayoutLimits {
                min_size: limits.min_size.max(preferred_size),
                max_size: limits.max_size - padding.size(),
            },
            squeeze,
        );

        let margin = (sizing.margin - padding).max(margin);

        let min_size = sizing.min.pad(&padding);
        let preferred_size = sizing.preferred.pad(&padding);

        let min_offset = resolve_pos(entity, content_area, min_size.size());
        let preferred_offset = resolve_pos(entity, content_area, preferred_size.size());

        Sizing {
            min: min_size.translate(min_offset),
            preferred: preferred_size.translate(preferred_offset),
            margin,
        }
    } else {
        // Leaf

        let min_offset = resolve_pos(entity, content_area, min_size);
        let preferred_offset = resolve_pos(entity, content_area, preferred_size);

        Sizing {
            min: Rect::from_size_pos(min_size, min_offset),
            preferred: Rect::from_size_pos(preferred_size, preferred_offset),
            margin,
        }
    }
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
    limits: LayoutLimits,
) -> Block {
    // let _span = tracing::info_span!( "Updating subtree", %entity, ?constraints).entered();
    let _span = tracing::debug_span!("update_subtree", %entity).entered();

    let query = (
        components::margin().opt_or_default(),
        padding().opt_or_default(),
        children().opt(),
        layout().opt(),
    );

    let mut query = entity.query(&query);
    let (&margin, &padding, children, layout) = query.get().unwrap();

    let children = children.map(Vec::as_slice).unwrap_or(&[]);

    if let Some(layout) = layout {
        // For a given layout use the largest size that fits within the constraints and then
        // potentially shrink it down.
        let mut block = layout.apply(
            world,
            entity,
            children,
            Rect::from_size(content_area).inset(&padding),
            LayoutLimits {
                min_size: limits
                    .min_size
                    .max(apply_constraints(entity, content_area, limits)),
                max_size: limits.max_size - padding.size(),
            },
        );

        if block.rect.size().x > limits.max_size.x || block.rect.size().y > limits.max_size.y {
            tracing::error!(
                %entity, rect_size=%block.rect.size(), %limits.max_size,
                "Widget size exceeds constraints",
            );
        }

        block.rect = block.rect.pad(&padding);

        block.margin = (block.margin - padding).max(margin);

        block
    } else {
        assert_eq!(children, [], "Entity has children but no layout");
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
    }
}

pub trait SizeResolver: Send + Sync {
    fn query(
        &mut self,
        entity: &EntityRef,
        content_area: Vec2,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> (Vec2, Vec2);
    fn apply(&mut self, entity: &EntityRef, content_area: Vec2, limits: LayoutLimits) -> Vec2;
}

fn resolve_base_size(
    entity: &EntityRef,
    content_area: Vec2,
    limits: LayoutLimits,
) -> (Vec2, Vec2, Constraints) {
    let query = (
        components::min_size().opt_or_default(),
        components::size().opt_or_default(),
        aspect_ratio().opt(),
    );
    let mut query = entity.query(&query);
    let (min_size, size, aspect_ratio) = query.get().unwrap();

    let min_size = min_size.resolve(content_area);
    let size = size
        .resolve(content_area)
        .clamp(limits.min_size, limits.max_size)
        .max(min_size);

    (
        min_size,
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
            if size.x > size.y {
                size = vec2(size.y * aspect_ratio, size.y);
            } else {
                size = vec2(size.x, size.x / aspect_ratio);
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
    let (mut min_size, mut size, constraints) = resolve_base_size(entity, content_area, limits);
    if let Ok(mut resolver) = entity.get_mut(components::size_resolver()) {
        let (resolved_min, resolved_size) = resolver.query(entity, content_area, limits, squeeze);

        min_size = resolved_min.max(min_size);
        size = resolved_size.max(size);
    }

    (
        constraints.resolve(min_size),
        constraints.resolve(size.min(limits.max_size)),
    )
}

fn apply_constraints(entity: &EntityRef, content_area: Vec2, limits: LayoutLimits) -> Vec2 {
    let (_, mut size, constraints) = resolve_base_size(entity, content_area, limits);
    if let Ok(mut resolver) = entity.get_mut(components::size_resolver()) {
        let resolved_size = resolver.apply(entity, content_area, limits);

        size = resolved_size.max(size);
    }

    constraints.resolve(size.min(limits.max_size))
}

/// Resolves a widgets position relative to its own bounds
fn resolve_pos(entity: &EntityRef, parent_size: Vec2, self_size: Vec2) -> Vec2 {
    let query = (offset().opt_or_default(), anchor().opt_or_default());
    let mut query = entity.query(&query);
    let (offset, anchor) = query.get().unwrap();

    let offset = offset.resolve(parent_size);

    offset - anchor.resolve(self_size)
}
