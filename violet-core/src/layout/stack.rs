use flax::{Entity, EntityRef, World};
use glam::{vec2, Vec2};
use itertools::Itertools;

use crate::{
    components::{self, Edges, Rect},
    layout::query_size,
};

use super::{resolve_pos, update_subtree, Alignment, Block, Direction, LayoutLimits, Sizing};

#[derive(Debug)]
pub struct StackableBounds {
    inner: Rect,
    outer: Rect,
}

impl Default for StackableBounds {
    fn default() -> Self {
        Self {
            inner: Rect {
                min: Vec2::MAX,
                max: Vec2::MIN,
            },
            outer: Rect {
                min: Vec2::MAX,
                max: Vec2::MIN,
            },
        }
    }
}

impl StackableBounds {
    fn new(rect: Rect, margin: Edges) -> Self {
        Self {
            inner: rect,
            outer: rect.pad(&margin),
        }
    }

    fn from_rect(rect: Rect) -> Self {
        Self {
            inner: rect,
            outer: rect,
        }
    }

    fn merge(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.merge(other.inner),
            outer: self.outer.merge(other.outer),
        }
    }

    fn margin(&self) -> Edges {
        let min = self.inner.min - self.outer.min;
        let max = self.outer.max - self.inner.max;

        Edges {
            left: min.x,
            right: max.x,
            top: min.y,
            bottom: max.y,
        }
    }
}

/// The stack layout
///
/// A stack layout is the Swiss army knife of layouts.
///
/// It can be used to create a stacked arrangement of widgets, aligning widgets in a horizontal or
/// vertical direction, or constraining and offsetting widgets within.
///
/// In short, this layout can works as one of the following:
/// - Stack
/// - Overlaying widgets
/// - Horizontal or vertical alignment
/// - Padding and margin with background colors (widgets don't inherently have a concept of "inner"
/// content, as they are their own content)
/// - Centering widgets (this isn't HTML :P)
/// - Limiting and expanding size of widgets
#[derive(Debug, Clone)]
pub struct StackLayout {
    pub horizontal_alignment: Alignment,
    pub vertical_alignment: Alignment,
}

impl Default for StackLayout {
    fn default() -> Self {
        Self {
            horizontal_alignment: Alignment::Center,
            vertical_alignment: Alignment::Center,
        }
    }
}

impl StackLayout {
    pub(crate) fn apply(
        &self,
        world: &World,
        entity: &EntityRef,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        let _span = tracing::info_span!("StackLayout::apply").entered();

        let mut bounds = Rect {
            min: Vec2::MAX,
            max: Vec2::MIN,
        };

        let blocks = children
            .iter()
            .map(|&child| {
                let entity = world.entity(child).expect("invalid child");

                // let pos = resolve_pos(&entity, content_area, preferred_size);

                let limits = LayoutLimits {
                    min_size: Vec2::ZERO,
                    max_size: limits.max_size,
                };

                let block = update_subtree(world, &entity, content_area.size(), limits);

                bounds = bounds.merge(block.rect.translate(content_area.min));

                (entity, block)
            })
            .collect_vec();

        // The size used for alignment calculation
        let size = bounds.size().clamp(limits.min_size, limits.max_size);

        let mut aligned_bounds =
            StackableBounds::from_rect(Rect::from_size_pos(limits.min_size, content_area.min));

        let offset = resolve_pos(entity, content_area.size(), size);
        for (entity, block) in blocks {
            let block_size = block.rect.size();
            let offset = content_area.min
                + offset
                + vec2(
                    self.horizontal_alignment.align_offset(size.x, block_size.x),
                    self.vertical_alignment.align_offset(size.y, block_size.y),
                );

            tracing::debug!(?offset, %entity);

            aligned_bounds = aligned_bounds.merge(&StackableBounds::new(
                block.rect.translate(offset),
                block.margin,
            ));

            // entity.update_dedup(components::rect(), block.rect.translate(offset));
            entity.update_dedup(components::rect(), block.rect);
            entity.update_dedup(components::local_position(), offset);
        }

        // tracing::info!(?aligned_bounds);

        // aligned_bounds.inner = aligned_bounds.inner.max_size(limits.min_size);
        let rect = aligned_bounds.inner;
        let margin = aligned_bounds.margin();

        // rect.min += content_area.min;
        // rect.max += content_area.min;

        Block::new(rect, margin)
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> Sizing {
        let min_rect = Rect::from_size_pos(limits.min_size, content_area.min);
        let mut min_bounds = StackableBounds::from_rect(min_rect);
        let mut preferred_bounds = StackableBounds::from_rect(min_rect);

        for &child in children.iter() {
            let entity = world.entity(child).expect("invalid child");

            let query = query_size(
                world,
                &entity,
                content_area.size(),
                LayoutLimits {
                    min_size: Vec2::ZERO,
                    max_size: limits.max_size,
                },
                squeeze,
            );

            min_bounds = min_bounds.merge(&StackableBounds::new(
                query.min.translate(content_area.min),
                query.margin,
            ));

            preferred_bounds = preferred_bounds.merge(&StackableBounds::new(
                query.preferred.translate(content_area.min),
                query.margin,
            ));
        }

        // min_bounds.inner = min_bounds.inner.max_size(limits.min_size);
        // preferred_bounds.inner = preferred_bounds.inner.max_size(limits.min_size);

        let min_margin = min_bounds.margin();
        let preferred_margin = preferred_bounds.margin();

        // tracing::info!(?min_margin, ?preferred_margin);

        // if min_margin != preferred_margin {
        //     tracing::warn!("margin discrepancy: {:?}", min_margin - preferred_margin);
        // }
        // tracing::info!(?min_margin, ?preferred_margin);

        Sizing {
            min: min_bounds
                .inner
                .clamp_size(limits.min_size, limits.max_size),
            preferred: preferred_bounds
                .inner
                .clamp_size(limits.min_size, limits.max_size),
            margin: min_margin.max(preferred_margin),
        }
    }
}
