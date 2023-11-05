mod flow;
mod stack;

use flax::{EntityRef, World};
use fontdue::{layout::TextStyle, Font};
use glam::{vec2, Vec2};

use crate::{
    components::{self, children, flow, font_size, intrinsic_size, padding, text, Edges, Rect},
    unit::Unit,
    wgpu::components::font,
};

pub use flow::{CrossAlign, Direction, Flow};
pub use stack::Stack;

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

pub fn query_size(world: &World, entity: &EntityRef, content_area: Rect) -> Sizing {
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
    if let Ok(layout) = entity.get(flow()) {
        // For a given layout use the largest size that fits within the constraints and then
        // potentially shrink it down.

        let row = layout.query_size(world, entity, content_area.inset(&padding));
        let margin = (row.margin - padding).max(Edges::even(0.0)).max(margin);

        Sizing {
            min: row.min.pad(&padding),
            preferred: row.preferred.pad(&padding),
            margin,
        }
    }
    // Stack
    else if let Ok(children) = entity.get(children()) {
        let query = Stack::default().query_size(world, &children, content_area.inset(&padding));

        // rect: block.rect.pad(&padding),
        let margin = (query.margin - padding).max(Edges::even(0.0)).max(margin);
        Sizing {
            min: query.min.pad(&padding),
            preferred: query.preferred.pad(&padding),
            margin,
        }
    } else {
        let (min_size, preferred_size) = resolve_size(entity, content_area, None);

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
    if let Ok(flow) = entity.get(flow()) {
        // For a given layout use the largest size that fits within the constraints and then
        // potentially shrink it down.

        let mut block = flow.apply(
            world,
            entity,
            content_area.inset(&padding),
            LayoutLimits {
                min_size: limits.min_size,
                max_size: limits.max_size - padding.size(),
            },
        );

        block.rect = block.rect.pad(&padding).max_size(limits.min_size);

        block.margin = (block.margin - padding).max(Edges::even(0.0)).max(margin);

        block
    }
    // Stack
    else if let Ok(children) = entity.get(children()) {
        let block = Stack::default().apply(
            world,
            &children,
            content_area.inset(&padding),
            LayoutLimits {
                min_size: limits.min_size,
                max_size: limits.max_size - padding.size(),
            },
        );

        Block {
            rect: block.rect.pad(&padding),
            margin: (block.margin - padding).max(Edges::even(0.0)).max(margin),
        }
        // for &child in &*children {
        //     let entity = world.entity(child).unwrap();

        //     // let local_rect = widget_outer_bounds(world, &entity, inner_rect.size());

        //     assert_eq!(content_area.size(), limits.max);
        //     let constraints = LayoutLimits {
        //         min: Vec2::ZERO,
        //         max: limits.max - padding.size(),
        //     };

        //     // We ask ourselves the question:
        //     //
        //     // Relative to ourselves, where can our children be placed without clipping.
        //     //
        //     // The answer is a origin bound rect of the same size as our content area, inset by the
        //     // imposed padding.
        //     let content_area = Rect {
        //         min: Vec2::ZERO,
        //         max: content_area.size(),
        //     }
        //     .inset(&padding);

        //     assert_eq!(content_area.size(), constraints.max);

        //     let res = update_subtree(world, &entigy, content_area, constraints);

        //     entity.update_dedup(components::rect(), res.rect);
        // }
        // Block {
        //     rect: total_bounds,
        //     margin,
        // }
    }
    // Text widgets height are influenced by their available width.
    else {
        let (_, size) = resolve_size(entity, content_area, Some(limits));

        let pos = resolve_pos(entity, content_area, size);
        let rect = Rect::from_size_pos(size, pos).clip(content_area);

        Block { rect, margin }
    }
}

#[inline]
fn resolve_size(
    entity: &EntityRef,
    content_area: Rect,
    limits: Option<LayoutLimits>,
) -> (Vec2, Vec2) {
    let parent_size = content_area.size();
    let min_size = entity
        .get(components::min_size())
        .as_deref()
        .unwrap_or(&Unit::ZERO)
        .resolve(parent_size);

    let mut size = if let Ok(size) = entity.get(components::size()) {
        size.resolve(parent_size)
    } else if let Some((text, font, &font_size)) =
        entity.query(&(text(), font(), font_size())).get()
    {
        resolve_text_size(text, font, font_size, limits)
    } else {
        // tracing::info!(%entity, "using intrinsic_size");
        entity
            .get_copy(intrinsic_size())
            .expect("intrinsic size required")
    }
    .max(min_size);

    if let Some(limits) = limits {
        size = size.clamp(limits.min_size, limits.max_size);
    }

    (min_size, size)
}

fn resolve_text_size(
    text: &str,
    font: &Font,
    font_size: f32,
    limits: Option<LayoutLimits>,
) -> Vec2 {
    let _span = tracing::debug_span!("resolve_text_size", ?font, font_size).entered();
    let mut layout =
        fontdue::layout::Layout::<()>::new(fontdue::layout::CoordinateSystem::PositiveYDown);

    let size = match limits {
        Some(v) => (Some(v.max_size.x), Some(v.max_size.y)),
        None => (None, None),
    };

    layout.reset(&fontdue::layout::LayoutSettings {
        x: 0.0,
        y: 0.0,
        max_width: size.0,
        max_height: size.1,
        horizontal_align: fontdue::layout::HorizontalAlign::Left,
        vertical_align: fontdue::layout::VerticalAlign::Top,
        line_height: 1.0,
        wrap_style: fontdue::layout::WrapStyle::Word,
        wrap_hard_breaks: true,
    });

    layout.append(
        &[font],
        &TextStyle {
            text,
            px: font_size,
            font_index: 0,
            user_data: (),
        },
    );

    layout
        .glyphs()
        .iter()
        .map(|v| vec2(v.x + v.width as f32, v.y + v.height as f32))
        .fold(Vec2::ZERO, |acc, v| acc.max(v))
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
