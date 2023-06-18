use flax::{EntityRef, World};
use glam::{vec2, Vec2};
use palette::convert::IntoColorUnclamped;

use crate::{
    components::{self, children, layout, local_position, padding, rect, Edges, Rect},
    constraints::widget_outer_bounds,
};

#[derive(Debug, Clone)]
struct MarginCursor {
    pending_margin: f32,
    start: Vec2,
    cursor: Vec2,
    line_height: f32,
}

impl MarginCursor {
    fn put(&mut self, block: &Block) -> Vec2 {
        let advance = (self.pending_margin.max(0.0).max(block.margin.left.max(0.0))
            + self.pending_margin.min(0.0)
            + block.margin.left.min(0.0))
        .max(0.0);

        self.pending_margin = block.margin.right;

        self.cursor.x += advance;

        let pos = self.cursor + vec2(0.0, block.margin.top);

        let size = block.rect.size();

        self.cursor.x += size.x;

        self.line_height = self
            .line_height
            .max(size.y + block.margin.top + block.margin.bottom);

        pos
    }

    fn finish(&mut self) -> Rect {
        self.cursor.y += self.line_height;
        self.cursor.x += self.pending_margin;

        self.pending_margin = 0.0;

        let line = Rect {
            min: self.start,
            max: self.cursor,
        };

        self.start = vec2(self.start.x, self.cursor.y);

        line
    }
}

#[derive(Debug)]
pub struct Layout {}

impl Layout {
    /// Position and size the children of the given entity using all the provided available space
    ///
    /// Returns the inner rect
    fn apply(
        &self,
        world: &World,
        entity: &EntityRef,
        padding: Edges,
        content_area: Rect,
        constraints: LayoutConstraints,
    ) -> Rect {
        let children = entity.get(children()).ok();
        let children = children.as_ref().map(|v| v.as_slice()).unwrap_or_default();

        let available_size = constraints.max - padding.size();

        // Start at the corner of the inner rect
        //
        // The inner rect is position relative to the layouts parent
        let inner_rect = content_area.inset(&padding);

        let mut cursor = MarginCursor {
            start: inner_rect.min,
            cursor: inner_rect.min,
            pending_margin: 0.0,
            line_height: 0.0,
        };

        // Reset to local
        let content_area = Rect {
            min: Vec2::ZERO,
            max: inner_rect.size(),
        };

        let mut blocks = Vec::new();

        for &child in children {
            let entity = world.entity(child).expect("Invalid child");

            // let local_rect = widget_outer_bounds(world, &child, size);
            let block = update_subtree(
                world,
                &entity,
                // Supply our whole inner content area
                content_area,
                LayoutConstraints {
                    min: Vec2::ZERO,
                    max: available_size,
                },
            );

            cursor.put(&block);

            blocks.push((entity, block));
        }

        let line = cursor.finish();

        tracing::debug!(line=?line.size(), "Line");

        let mut cursor = MarginCursor {
            start: inner_rect.min,
            cursor: inner_rect.min,
            pending_margin: 0.0,
            line_height: 0.0,
        };

        for (entity, block) in blocks {
            // And move it all by the cursor position
            let height = block.rect.size().y + block.margin.size().y;
            let offset = (line.size().y - height) / 2.0;

            tracing::debug!(?offset, ?height, inner_size=?block.rect.size(), margin=?block.margin, "block");

            let pos = cursor.put(&block) + vec2(0.0, offset);

            entity.update(components::rect(), |v| *v = block.rect);
            entity.update(components::local_position(), |v| *v = pos);
        }

        line.pad(&padding)
    }

    pub(crate) fn total_size(&self, world: &World, entity: &EntityRef, size: Vec2) -> Vec2 {
        let children = entity.get(children()).ok();
        let children = children.as_ref().map(|v| v.as_slice()).unwrap_or_default();

        let mut cursor = 0.0;
        let mut line_height = 0.0f32;

        for &child in children {
            let child = world.entity(child).expect("Invalid child");
            let rect = widget_outer_bounds(world, &child, size);

            cursor += rect.size().x + 10.0;
            line_height = line_height.max(rect.size().y);
        }

        vec2(cursor, line_height)
    }
}

/// Constraints for a child widget passed down from the parent.
///
/// Allows for the parent to control the size of the children, such as stretching
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutConstraints {
    pub min: Vec2,
    pub max: Vec2,
}

/// A block is a rectangle and surrounding support such as margin
#[derive(Debug, Clone)]
pub(crate) struct Block {
    pub(crate) rect: Rect,
    pub(crate) margin: Edges,
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
    constraints: LayoutConstraints,
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
    if let Ok(layout) = entity.get(layout()) {
        // For a given layout use the largest size that fits within the constraints and then
        // potentially shrink it down.
        assert_eq!(content_area.size(), constraints.max);
        tracing::info!(?padding, ?content_area, ?constraints, "Flowing {entity}");

        let rect = layout.apply(world, entity, padding, content_area, constraints);

        Block { rect, margin }
    }
    // Stack
    else if let Ok(children) = entity.get(children()) {
        let total_bounds = Rect {
            min: Vec2::ZERO,
            max: Vec2::ONE,
        };

        for &child in &*children {
            let entity = world.entity(child).unwrap();

            // let local_rect = widget_outer_bounds(world, &entity, inner_rect.size());

            assert_eq!(content_area.size(), constraints.max);
            tracing::debug!(?padding, ?content_area, ?constraints, "Stacking {entity}");
            let constraints = LayoutConstraints {
                min: Vec2::ZERO,
                max: constraints.max - padding.size(),
            };

            // We ask ourselves the question:
            //
            // Relative to ourselves, where can our children be placed without clipping.
            //
            // The answer is a origin bound rect of the same size as our content area, inset by the
            // imposed padding.
            let content_area = Rect {
                min: Vec2::ZERO,
                max: content_area.size(),
            }
            .inset(&padding);
            assert_eq!(content_area.size(), constraints.max);

            let res = update_subtree(world, &entity, content_area, constraints);

            entity.update(components::rect(), |v| *v = res.rect);
        }
        Block {
            rect: total_bounds,
            margin,
        }
    }
    // Leaf
    else if let Ok(v) = entity.get(components::constraints()) {
        let rect = v.apply(content_area, constraints);
        // tracing::info!("Constrained {rect:?}");

        Block { rect, margin }
    } else {
        tracing::warn!(%entity, "Widget is not positioned");
        Block {
            rect: Rect::default(),
            margin: Edges::default(),
        }
    }
}
