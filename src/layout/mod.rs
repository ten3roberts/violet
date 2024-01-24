mod flow;
mod stack;

use flax::{Entity, EntityRef, FetchExt, World};
use glam::Vec2;

use crate::{
    components::{self, children, layout, padding, Edges, Rect},
    unit::Unit,
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

    fn rotate(self) -> Self {
        match self {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal,
        }
    }

    pub(crate) fn to_axis(&self) -> Vec2 {
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
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        match self {
            Layout::Stack(v) => v.apply(world, children, content_area, limits),
            Layout::Flow(v) => v.apply(world, children, content_area, limits),
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
pub(crate) struct LayoutLimits {
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

pub fn query_size(
    world: &World,
    entity: &EntityRef,
    content_area: Rect,
    limits: LayoutLimits,
    squeeze: Direction,
) -> Sizing {
    let margin = entity
        .get(components::margin())
        .ok()
        .as_deref()
        .copied()
        .unwrap_or_default();

    let padding = entity
        .get(padding())
        .ok()
        .as_deref()
        .copied()
        .unwrap_or_default();

    // Flow
    if let Some((children, layout)) = entity.query(&(children(), layout())).get() {
        let sizing = layout.query_size(
            world,
            children,
            content_area.inset(&padding),
            LayoutLimits {
                min_size: limits.min_size,
                max_size: limits.max_size - padding.size(),
            },
            squeeze,
        );
        let margin = (sizing.margin - padding).max(margin);

        Sizing {
            min: sizing.min.pad(&padding),
            preferred: sizing.preferred.pad(&padding),
            margin,
        }
        // }
        // else if let Ok(layout) = entity.get(flow()) {
        //     // For a given layout use the largest size that fits within the constraints and then
        //     // potentially shrink it down.

        //     let row = layout.query_size(world, entity, content_area.inset(&padding));
        //     let margin = (row.margin - padding).max(margin);

        //     Sizing {
        //         min: row.min.pad(&padding),
        //         preferred: row.preferred.pad(&padding),
        //         margin,
        //     }
        // }
        // Stack
        // else if let Some((children, stack)) = entity
        //     .query(&(children(), components::stack().opt_or_default()))
        //     .get()
        // {
        //     let query = stack.query_size(world, children, content_area.inset(&padding));

        //     // rect: block.rect.pad(&padding),
        //     let margin = (query.margin - padding).max(Edges::even(0.0)).max(margin);
        //     Sizing {
        //         min: query.min.pad(&padding),
        //         preferred: query.preferred.pad(&padding),
        //         margin,
        //     }
        // }
    } else {
        let (min_size, preferred_size) = query_contraints(entity, content_area, limits, squeeze);

        let min_offset = resolve_pos(entity, content_area, min_size);
        let preferred_offset = resolve_pos(entity, content_area, preferred_size);

        // Leaf

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
    // The area in which children can be placed without clipping
    content_area: Rect,
    limits: LayoutLimits,
) -> Block {
    // let _span = tracing::info_span!( "Updating subtree", %entity, ?constraints).entered();
    let _span = tracing::debug_span!("update_subtree", %entity).entered();

    let margin = entity
        .get(components::margin())
        .ok()
        .as_deref()
        .copied()
        .unwrap_or_default();

    let padding = entity
        .get(padding())
        .ok()
        .as_deref()
        .copied()
        .unwrap_or_default();

    // Layout
    if let Some((children, layout)) = entity.query(&(children(), layout())).get() {
        // For a given layout use the largest size that fits within the constraints and then
        // potentially shrink it down.

        let mut block = layout.apply(
            world,
            children,
            content_area.inset(&padding),
            LayoutLimits {
                min_size: limits.min_size,
                max_size: limits.max_size - padding.size(),
            },
        );

        // assert!(block.rect.size().x <= limits.max_size.x);
        // assert!(block.rect.size().y <= limits.max_size.y);

        block.rect = block.rect.pad(&padding).max_size(limits.min_size);

        block.margin = (block.margin - padding).max(margin);

        block
    }
    // Text widgets height are influenced by their available width.
    else {
        assert_eq!(
            entity
                .get(children())
                .as_deref()
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            &[],
            "Widgets with no layout may not have children"
        );

        let size = apply_contraints(entity, content_area, limits);

        if size.x > limits.max_size.x || size.y > limits.max_size.y {
            tracing::error!(
                %entity, %size, %limits.max_size,
                "Widget size exceeds constraints",
            );
        }

        let pos = resolve_pos(entity, content_area, size);
        let rect = Rect::from_size_pos(size, pos);

        entity.update_dedup(components::layout_bounds(), size);

        Block { rect, margin }
    }
}

pub trait SizeResolver: Send + Sync {
    fn query(
        &mut self,
        entity: &EntityRef,
        content_area: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> (Vec2, Vec2);
    fn apply(&mut self, entity: &EntityRef, content_area: Rect, limits: LayoutLimits) -> Vec2;
}

fn query_contraints(
    entity: &EntityRef,
    content_area: Rect,
    limits: LayoutLimits,
    squeeze: Direction,
) -> (Vec2, Vec2) {
    let query = (
        components::min_size().opt_or_default(),
        components::size().opt_or_default(),
        components::size_resolver().as_mut().opt(),
    );
    let mut query = entity.query(&query);
    let (min_size, size, resolver) = query.get().unwrap();

    let mut min_size = min_size.resolve(content_area.size());
    let mut size = size.resolve(content_area.size()).max(min_size);

    if let Some(resolver) = resolver {
        let (resolved_min, resolved_size) = resolver.query(entity, content_area, limits, squeeze);

        min_size = resolved_min.max(min_size);
        size = resolved_size.max(size);
    }

    (min_size, size.min(limits.max_size))
}

fn apply_contraints(entity: &EntityRef, content_area: Rect, limits: LayoutLimits) -> Vec2 {
    let query = (
        components::min_size().opt_or_default(),
        components::size().opt_or_default(),
        components::size_resolver().as_mut().opt(),
    );
    let mut query = entity.query(&query);
    let (min_size, size, resolver) = query.get().unwrap();

    let mut size = size
        .resolve(content_area.size())
        .max(min_size.resolve(content_area.size()));

    if let Some(resolver) = resolver {
        let resolved_size = resolver.apply(entity, content_area, limits);

        size = resolved_size.max(size);
    }

    size.min(limits.max_size)
}

fn resolve_pos(entity: &EntityRef, content_area: Rect, self_size: Vec2) -> Vec2 {
    let offset = entity.get(components::offset());
    let anchor = entity.get(components::anchor());

    let offset = offset
        .as_deref()
        .unwrap_or(&Unit::ZERO)
        .resolve(content_area.size());

    let pos =
        content_area.pos() + offset - anchor.as_deref().unwrap_or(&Unit::ZERO).resolve(self_size);
    pos
}
