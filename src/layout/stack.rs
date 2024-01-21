use flax::{Entity, World};
use glam::{vec2, Vec2};
use itertools::Itertools;

use crate::{
    components::{self, Edges, Rect},
    layout::query_size,
};

use super::{update_subtree, Block, CrossAlign, LayoutLimits, Sizing};

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
#[derive(Debug, Clone)]
pub struct StackLayout {
    pub horizontal_alignment: CrossAlign,
    pub vertical_alignment: CrossAlign,
}

impl Default for StackLayout {
    fn default() -> Self {
        Self {
            horizontal_alignment: CrossAlign::Center,
            vertical_alignment: CrossAlign::Center,
        }
    }
}

impl StackLayout {
    pub(crate) fn apply(
        &self,
        world: &World,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        // tracing::info!(
        //     ?content_area,
        //     content_area_size=%content_area.size(),
        //     ?limits
        // );

        // Reset to local
        let inner_rect = Rect {
            min: Vec2::ZERO,
            max: content_area.size(),
        };

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

                let block = update_subtree(world, &entity, inner_rect, limits);

                bounds = bounds.merge(block.rect.translate(content_area.min));

                (entity, block)
            })
            .collect_vec();

        let size = bounds.size();

        let mut aligned_bounds = StackableBounds::default();

        for (entity, block) in blocks {
            let block_size = block.rect.size();
            let offset = content_area.min
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

        let rect = aligned_bounds.inner; //.max_size(limits.min_size);
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
        squeeze: Vec2,
    ) -> Sizing {
        // Reset to local
        let inner_rect = Rect {
            min: Vec2::ZERO,
            max: content_area.size(),
        };

        let mut min_bounds = StackableBounds::default();
        let mut preferred_bounds = StackableBounds::default();

        for &child in children.iter() {
            let entity = world.entity(child).expect("invalid child");

            let query = query_size(world, &entity, inner_rect, squeeze);

            min_bounds = min_bounds.merge(&StackableBounds::new(
                query.min.translate(content_area.min),
                query.margin,
            ));

            preferred_bounds = preferred_bounds.merge(&StackableBounds::new(
                query.preferred.translate(content_area.min),
                query.margin,
            ));
        }

        let min_margin = min_bounds.margin();
        let preferred_margin = preferred_bounds.margin();

        // if min_margin != preferred_margin {
        //     tracing::warn!("margin discrepency: {:?}", min_margin - preferred_margin);
        // }
        // tracing::info!(?min_margin, ?preferred_margin);

        Sizing {
            min: min_bounds.inner,
            preferred: preferred_bounds.inner,
            margin: min_margin.max(preferred_margin),
        }
    }
}
