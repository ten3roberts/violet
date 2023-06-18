use flax::{EntityRef, World};
use glam::{vec2, Vec2};

use crate::{
    components::{self, children, layout, padding, Edges, Rect},
    constraints::widget_outer_bounds,
};

#[derive(Debug, Clone)]
struct MarginCursor {
    pending_margin: f32,
    start: Vec2,
    cursor: Vec2,
    line_height: f32,
    axis: Vec2,
    cross_axis: Vec2,
}

impl MarginCursor {
    fn new(start: Vec2, axis: Vec2, cross_axis: Vec2) -> Self {
        Self {
            pending_margin: 0.0,
            start,
            cursor: start,
            line_height: 0.0,
            axis,
            cross_axis,
        }
    }

    fn put(&mut self, block: &Block) -> Vec2 {
        let (front_margin, back_margin) = block.margin.in_axis(self.axis);

        let advance = (self.pending_margin.max(0.0).max(back_margin.max(0.0))
            + self.pending_margin.min(0.0)
            + back_margin.min(0.0))
        .max(0.0);

        self.pending_margin = front_margin;

        self.cursor += advance * self.axis + block.rect.support(-self.axis) * self.axis;

        let (start_margin, end_margin) = block.margin.in_axis(self.cross_axis);
        let pos = self.cursor + start_margin * self.cross_axis;

        let extent = block.rect.support(self.axis);

        self.cursor += extent * self.axis;

        self.line_height = self
            .line_height
            .max(block.rect.size().dot(self.cross_axis) + start_margin + end_margin);

        pos
    }

    fn finish(&mut self) -> Rect {
        self.cursor += self.line_height * self.cross_axis;
        self.cursor += self.pending_margin * self.axis;

        self.pending_margin = 0.0;

        let line = Rect::from_two_points(self.start, self.cursor);
        self.start = self.start * self.axis + self.cursor + self.cross_axis;

        line
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Horizontal,
    Vertical,
    HorizontalReverse,
    VerticalReverse,
}

impl Direction {
    fn axis(&self) -> (Vec2, Vec2) {
        match self {
            Direction::Horizontal => (Vec2::X, Vec2::Y),
            Direction::Vertical => (Vec2::Y, Vec2::X),
            Direction::HorizontalReverse => (-Vec2::X, Vec2::Y),
            Direction::VerticalReverse => (-Vec2::Y, Vec2::X),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CrossAlign {
    /// Align items to the start of the cross axis
    Start,
    /// Align items to the center of the cross axis
    Center,
    /// Align items to the end of the cross axis
    End,
}

impl CrossAlign {
    fn align_offset(&self, total_size: f32, size: f32) -> f32 {
        match self {
            CrossAlign::Start => 0.0,
            CrossAlign::Center => (total_size - size) / 2.0,
            CrossAlign::End => total_size - size,
        }
    }
}

#[derive(Debug)]
pub struct Layout {
    pub cross_align: CrossAlign,
    pub direction: Direction,
}

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

        let (axis, cross_axis) = self.direction.axis();

        let mut cursor = MarginCursor::new(Vec2::ZERO, axis, cross_axis);

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

        let line_size = line.size();

        let start = match self.direction {
            Direction::Horizontal => inner_rect.min,
            Direction::Vertical => inner_rect.min,
            Direction::HorizontalReverse => vec2(inner_rect.max.x, inner_rect.min.y),
            Direction::VerticalReverse => vec2(inner_rect.min.x, inner_rect.max.y),
        };

        tracing::debug!(?axis, ?cross_axis, ?start,line=?line.size(), "Line");

        let mut cursor = MarginCursor::new(start, axis, cross_axis);

        tracing::debug!(?cross_axis, "cross_axis");
        for (entity, block) in blocks {
            // And move it all by the cursor position
            let height = (block.rect.size() + block.margin.size()).dot(cross_axis);

            let pos = cursor.put(&block)
                + self
                    .cross_align
                    .align_offset(line_size.dot(cross_axis), height)
                    * cross_axis;

            tracing::debug!(?pos);

            entity.update(components::rect(), |v| *v = block.rect);
            entity.update(components::local_position(), |v| *v = pos);
        }
        let line = cursor.finish();
        tracing::debug!(?line, "Final line");

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
