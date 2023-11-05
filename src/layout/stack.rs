use flax::{Entity, World};
use glam::{vec2, Vec2};
use itertools::Itertools;

use crate::{
    components::{self, Edges, Rect},
    layout::{query_size, resolve_pos},
};

use super::{update_subtree, Block, CrossAlign, LayoutLimits, Sizing};

#[derive(Default, Debug)]
pub struct StackableBounds {
    inner: Rect,
    outer: Rect,
}

impl StackableBounds {
    fn new(rect: Rect, margin: Edges) -> Self {
        Self {
            inner: rect,
            outer: rect.pad(&margin),
        }
    }

    fn merge(&self, other: &Self) -> Self {
        Self {
            inner: self.inner.merge(other.inner),
            outer: self.outer.merge(other.outer),
        }
    }

    fn margin(&self) -> Edges {
        let min = self.outer.min - self.inner.min;
        let max = self.inner.max - self.outer.max;

        Edges {
            left: min.x,
            right: max.x,
            top: min.y,
            bottom: max.y,
        }
    }
}

/// The stack layout
pub struct Stack {
    pub horizontal_alignment: CrossAlign,
    pub vertical_alignment: CrossAlign,
}

impl Default for Stack {
    fn default() -> Self {
        Self {
            horizontal_alignment: CrossAlign::Center,
            vertical_alignment: CrossAlign::End,
        }
    }
}

impl Stack {
    pub(crate) fn apply(
        &self,
        world: &World,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        let _span = tracing::info_span!("Stack::apply").entered();

        tracing::info!(
            ?content_area,
            content_area_size=%content_area.size(),
            ?limits
        );

        // Reset to local
        let inner_rect = Rect {
            min: Vec2::ZERO,
            max: content_area.size(),
        };

        let mut bounds = StackableBounds::default();

        let blocks = children
            .iter()
            .map(|&child| {
                let entity = world.entity(child).expect("invalid child");

                // let pos = resolve_pos(&entity, content_area, preferred_size);

                let limits = LayoutLimits {
                    min_size: Vec2::ZERO,
                    max_size: limits.max_size,
                };

                let block = update_subtree(world, &entity, inner_rect, limits);

                bounds = bounds.merge(&StackableBounds {
                    inner: block.rect,
                    outer: block.rect.pad(&block.margin),
                });

                (entity, block)
            })
            .collect_vec();

        let size = bounds.inner.size();

        for (entity, block) in blocks {
            let block_size = block.rect.size();
            let offset = content_area.min
                + vec2(
                    self.horizontal_alignment.align_offset(size.x, block_size.x),
                    self.vertical_alignment.align_offset(size.y, block_size.y),
                );

            entity.update_dedup(components::rect(), block.rect);
            entity.update_dedup(components::local_position(), offset);
        }

        let margin = bounds.margin();
        let mut rect = bounds.inner;
        rect.min += content_area.min;
        rect.max += content_area.min;

        Block::new(rect, margin)
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        children: &[Entity],
        content_area: Rect,
    ) -> Sizing {
        // Reset to local
        let inner_rect = Rect {
            min: Vec2::ZERO,
            max: content_area.size(),
        };

        let (min_bounds, preferred_bounds) = children
            .iter()
            .map(|&child| {
                let entity = world.entity(child).expect("invalid child");

                let query = query_size(world, &entity, inner_rect);
                (
                    StackableBounds::new(query.min, query.margin),
                    StackableBounds::new(query.preferred, query.margin),
                )
            })
            .reduce(|a, b| (a.0.merge(&b.0), a.1.merge(&b.1)))
            .unwrap_or_default();

        let min_margin = min_bounds.margin();
        let preferred_margin = preferred_bounds.margin();

        if min_margin != preferred_margin {
            tracing::warn!("margin discrepency: {:?}", min_margin - preferred_margin);
        }

        Sizing {
            min: min_bounds.inner,
            preferred: preferred_bounds.inner,
            margin: min_margin.max(preferred_margin),
        }
    }
}
