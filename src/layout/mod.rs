mod flow;
mod stack;

use flax::{Entity, EntityRef, World};
use glam::Vec2;

use crate::{
    components::{self, children, layout, padding, Edges, Rect},
    unit::Unit,
};

pub use flow::{CrossAlign, Direction, FlowLayout};
pub use stack::StackLayout;

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
        squeeze: Vec2,
    ) -> Sizing {
        match self {
            Layout::Stack(v) => v.query_size(world, children, inner_rect, squeeze),
            Layout::Flow(v) => v.query_size(world, children, inner_rect).sizing(),
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

pub fn query_size(world: &World, entity: &EntityRef, content_area: Rect, squeeze: Vec2) -> Sizing {
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
        let sizing = layout.query_size(world, children, content_area.inset(&padding), squeeze);
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
        let (min_size, preferred_size) = resolve_size(entity, content_area, None, squeeze);

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

        block.rect = block.rect.pad(&padding).max_size(limits.min_size);

        block.margin = (block.margin - padding).max(margin);

        block
    }
    // Text widgets height are influenced by their available width.
    else {
        let (_, size) = resolve_size(entity, content_area, Some(limits), Vec2::ZERO);

        let pos = resolve_pos(entity, content_area, size);
        let rect = Rect::from_size_pos(size, pos).clip(content_area);

        entity.update_dedup(components::layout_bounds(), size);

        Block { rect, margin }
    }
}

pub(crate) trait SizeResolver: Send + Sync {
    fn resolve(
        &mut self,
        entity: &EntityRef,
        content_area: Rect,
        limits: Option<LayoutLimits>,
        squeeze: Vec2,
    ) -> (Vec2, Vec2);
}

#[inline]
fn resolve_size(
    entity: &EntityRef,
    content_area: Rect,
    limits: Option<LayoutLimits>,
    squeeze: Vec2,
) -> (Vec2, Vec2) {
    let parent_size = content_area.size();

    let (min_size, size) = if let Ok(size) = entity.get(components::size()) {
        let min_size = entity
            .get(components::min_size())
            .as_deref()
            .unwrap_or(&Unit::ZERO)
            .resolve(parent_size);

        let mut size = size.resolve(parent_size).max(min_size);
        if let Some(limits) = limits {
            size = size.clamp(limits.min_size, limits.max_size);
        }
        (min_size, size)
        // else if let Some((text, font, &font_size)) =
        //     entity.query(&(text(), font_handle(), font_size())).get()
        // {
        //     let min_size = resolve_text_size(
        //         text,
        //         font,
        //         font_size,
        //         content_area,
        //         Some(LayoutLimits {
        //             min_size: Vec2::ZERO,
        //             max_size: Vec2::new(font_size, font_size),
        //         }),
        //     );
        //     let preferred = resolve_text_size(text, font, font_size, content_area, limits);

        //     (min_size, preferred)
        // }
    } else if let Ok(mut resolver) = entity.get_mut(components::size_resolver()) {
        resolver.resolve(entity, content_area, limits, squeeze)
    } else {
        // tracing::info!(%entity, "using intrinsic_size");
        (Vec2::ZERO, Vec2::ZERO)
    };

    (min_size, size)
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
