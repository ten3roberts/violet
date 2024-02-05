use flax::{Entity, EntityRef, World};
use glam::{vec2, Vec2};
use itertools::Itertools;

use crate::{
    components::{self, margin, padding, Edges, Rect},
    layout::query_size,
};

use super::{
    query_constraints, resolve_pos, update_subtree, Block, CrossAlign, Direction, LayoutLimits,
    Sizing,
};

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
        entity: &EntityRef,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        let padding = entity.get_copy(padding()).unwrap_or_default();
        let margin = entity.get_copy(margin()).unwrap_or_default();

        let _span = tracing::info_span!("StackLayout::apply").entered();
        // tracing::info!(
        //     ?content_area,
        //     content_area_size=%content_area.size(),
        //     ?limits
        // );

        // Reset to local
        let inner_rect = content_area.inset(&padding);

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

                let block =
                    update_subtree(world, &entity, Rect::from_size(inner_rect.size()), limits);

                bounds = bounds.merge(block.rect.translate(content_area.min));

                (entity, block)
            })
            .collect_vec();

        let size = bounds.size().max(limits.min_size);

        let mut aligned_bounds = StackableBounds::default();

        let offset = resolve_pos(entity, content_area, size);
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

        let rect = aligned_bounds.inner.max_size(limits.min_size).pad(&padding);
        let offset = resolve_pos(entity, content_area.size(), rect.size());

        let margin = (aligned_bounds.margin() - padding).max(margin);

        // rect.min += content_area.min;
        // rect.max += content_area.min;

        Block::new(rect.translate(offset), margin)
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        entity: &EntityRef,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> Sizing {
        let padding = entity.get_copy(padding()).unwrap_or_default();
        let margin = entity.get_copy(margin()).unwrap_or_default();

        // Reset to local
        let inner_rect = content_area.inset(&padding);

        let mut min_bounds = StackableBounds::default();
        let mut preferred_bounds = StackableBounds::default();

        for &child in children.iter() {
            let entity = world.entity(child).expect("invalid child");

            let query = query_size(
                world,
                &entity,
                Rect::from_size(inner_rect.size()),
                limits,
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

        let min_margin = min_bounds.margin();
        let preferred_margin = preferred_bounds.margin();

        let min = min_bounds.inner.pad(&padding);
        let preferred = preferred_bounds.inner.pad(&padding);

        let min_offset = resolve_pos(entity, content_area.size(), min.size());
        let preferred_offset = resolve_pos(entity, content_area.size(), preferred.size());

        // let (min_size, preferred_size) = query_constraints(entity, content_area, limits, squeeze);

        tracing::info!(%entity, ?min_offset, ?preferred_offset);
        Sizing {
            min,
            preferred,
            margin: (min_margin.max(preferred_margin) - padding).max(margin),
        }
    }
}
