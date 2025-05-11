use flax::{Entity, EntityRef, World};
use glam::{vec2, BVec2, Vec2};
use itertools::Itertools;

use super::{apply_layout, resolve_pos, ApplyLayoutArgs, Block, LayoutLimits, QueryArgs, Sizing};
use crate::{
    components::{self, item_align, LayoutAlignment},
    layout::{query_size, LayoutArgs, SizingHints},
    Edges, Rect,
};

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
///     content, as they are their own content)
/// - Centering widgets (this isn't HTML :P)
/// - Limiting and expanding size of widgets
///
/// Margins:
/// By default, the stack layout will inherit the margins of the inner children
#[derive(Default, Debug, Clone)]
pub struct StackLayout {
    pub alignment: LayoutAlignment,
    pub clip: BVec2,
}

impl StackLayout {
    pub(crate) fn apply(&self, world: &World, entity: &EntityRef, args: ApplyLayoutArgs) -> Block {
        puffin::profile_function!();
        let _span = tracing::debug_span!("StackLayout::apply", %self.clip, %entity).entered();

        let mut bounds = Rect {
            min: Vec2::MAX,
            max: Vec2::MIN,
        };

        let clip = vec2(self.clip.x as u32 as f32, self.clip.y as u32 as f32);
        let child_limits = LayoutLimits {
            min_size: args.limits.min_size,
            max_size: args.limits.max_size,
        };

        let blocks = args
            .children
            .iter()
            .map(|&child| {
                let entity = world.entity(child).expect("invalid child");

                let block = apply_layout(
                    world,
                    &entity,
                    LayoutArgs {
                        content_area: args.content_area,
                        limits: child_limits,
                    },
                );

                bounds = bounds.merge(block.rect.translate(args.offset));

                (entity, block)
            })
            .collect_vec();

        // The size used for alignment calculation
        let total_size = bounds
            .size()
            .max(args.preferred_size)
            .max(args.limits.min_size);

        let mut aligned_bounds =
            StackableBounds::from_rect(Rect::from_size_pos(args.preferred_size, args.offset));

        let mut can_grow = BVec2::FALSE;

        let offset = args.offset;
        let start_position = resolve_pos(entity, args.content_area, total_size);

        for (entity, block) in blocks {
            let block_size = block.rect.size();

            let offset = offset
                + entity
                    .get_copy(item_align())
                    .unwrap_or(self.alignment)
                    .align(total_size, block_size);

            let clip_mask = Rect::from_size(clip * args.limits.max_size + Vec2::MAX * (1.0 - clip));

            aligned_bounds = aligned_bounds.merge(&StackableBounds::new(
                block.rect.translate(offset),
                block.margin,
            ));

            can_grow |= block.can_grow;

            entity.update_dedup(components::rect(), block.rect).unwrap();
            entity
                .update_dedup(components::local_position(), offset + start_position)
                .unwrap();

            entity
                .update_dedup(components::clip_mask(), clip_mask)
                .unwrap();
        }

        let rect = aligned_bounds.inner;

        let mut rect = rect.max_size(args.limits.min_size);

        rect = rect.min_size(args.limits.max_size * clip + Vec2::MAX * (1.0 - clip));

        let margin = aligned_bounds.margin();

        Block::new(rect, margin, can_grow)
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        children: &[Entity],
        args: QueryArgs,
        preferred_size: Vec2,
    ) -> Sizing {
        puffin::profile_function!();
        let min_rect = Rect::from_size(args.limits.min_size);

        let mut min_bounds = StackableBounds::from_rect(min_rect);
        let mut preferred_bounds = StackableBounds::from_rect(min_rect);

        let mut hints = SizingHints::default();
        let mut maximize = Vec2::ZERO;

        let clip = vec2(self.clip.x as u32 as f32, self.clip.y as u32 as f32);
        let child_limits = LayoutLimits {
            min_size: args.limits.min_size,
            max_size: args.limits.max_size,
        };

        for &child in children.iter() {
            let entity = world.entity(child).expect("invalid child");

            let sizing = query_size(
                world,
                &entity,
                QueryArgs {
                    limits: child_limits,
                    content_area: args.content_area,
                    direction: args.direction,
                },
            );

            maximize += sizing.maximize;

            hints = hints.combine(sizing.hints);

            min_bounds = min_bounds.merge(&StackableBounds::new(sizing.min, sizing.margin));

            preferred_bounds =
                preferred_bounds.merge(&StackableBounds::new(sizing.preferred, sizing.margin));
        }

        let min_rect = min_bounds.inner;
        let preferred_rect = preferred_bounds.inner;

        let min_margin = min_bounds.margin();
        let preferred_margin = preferred_bounds.margin();

        // ensure size is not smaller than min
        let min = min_rect.max_size(args.limits.min_size);
        let preferred = preferred_rect.max_size(preferred_size);

        // if clip, clamp to limited max size, otherwise, clip to max
        let clamp_size = args.limits.max_size * clip + Vec2::MAX * (1.0 - clip);

        Sizing {
            min: min.min_size((1.0 - clip) * min.size()),
            preferred: preferred.min_size(clamp_size),
            margin: min_margin.max(preferred_margin),
            hints,
            maximize,
        }
    }
}

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
            outer: rect.pad(margin),
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
