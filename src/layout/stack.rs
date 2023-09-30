use flax::{Entity, World};
use glam::Vec2;

use crate::{
    components::{self, Edges, Rect},
    layout::query_size,
};

use super::{update_subtree, Block, LayoutLimits, SizeQuery};

// #[derive(Debug)]
// struct StackCursor {
//     inner: Rect,
//     outer: Rect,
// }

// impl StackCursor {
//     fn new(block: Block) -> Self {
//         Self {
//             inner: Rect::ZERO,
//             outer: Rect::ZERO,
//         }
//     }

//     fn put(&mut self, block: Block) {
//         self.inner = self.inner.merge(block.rect);

//         self.outer = self.outer.merge(block.rect.pad(&block.margin));
//     }

//     fn margin(&self) -> Edges {
//         let min = self.outer.min - self.inner.min;
//         let max = self.inner.max - self.outer.max;

//         Edges {
//             left: min.x,
//             right: max.x,
//             top: min.y,
//             bottom: max.y,
//         }
//     }
// }

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
pub struct Stack {}

impl Stack {
    pub(crate) fn apply(
        &self,
        world: &World,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        // Reset to local
        let inner_rect = Rect {
            min: Vec2::ZERO,
            max: content_area.size(),
        };

        let bounds = children
            .iter()
            .map(|&child| {
                let entity = world.entity(child).expect("invalid child");

                let limits = LayoutLimits {
                    min: Vec2::ZERO,
                    max: limits.max,
                };

                let block = update_subtree(world, &entity, inner_rect, limits);

                entity.update_dedup(components::rect(), block.rect);
                entity.update_dedup(components::local_position(), content_area.min);

                StackableBounds {
                    inner: block.rect,
                    outer: block.rect.pad(&block.margin),
                }
            })
            .reduce(|a, b| a.merge(&b))
            .unwrap_or_default();

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
    ) -> SizeQuery {
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

        tracing::debug!("margin discrepency: {:?}", min_margin - preferred_margin);

        SizeQuery {
            min: min_bounds.inner,
            preferred: preferred_bounds.inner,
            margin: min_margin.max(preferred_margin),
        }
    }
}
