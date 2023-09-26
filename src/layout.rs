use flax::{EntityRef, World};
use glam::{vec2, Vec2};
use itertools::Itertools;

use crate::{
    components::{self, children, intrinsic_size, layout, margin, padding, Edges, Rect},
    unit::Unit,
};

#[derive(Debug, Clone)]
struct MarginCursor {
    /// Margin of the last widget to be merged before the next
    pending_margin: f32,
    start: Vec2,
    main_cursor: f32,
    cross_cursor: f32,

    line_height: f32,
    axis: Vec2,
    cross_axis: Vec2,
    main_margin: (f32, f32),
    cross_margin: (f32, f32),
    contain_margins: bool,
}

impl MarginCursor {
    fn new(start: Vec2, axis: Vec2, cross_axis: Vec2, contain_margins: bool) -> Self {
        Self {
            // Setting this to -inf will cause the marging to leak out of the container. This would
            // be akin to having no back support for the widget to be placed against.
            pending_margin: if contain_margins { 0.0 } else { f32::MIN },
            start,

            main_cursor: start.dot(axis),
            cross_cursor: start.dot(cross_axis),
            line_height: 0.0,
            axis,
            cross_axis,
            main_margin: (0.0, 0.0),
            cross_margin: (0.0, 0.0),
            contain_margins,
        }
    }

    fn put(&mut self, block: &Block) -> Vec2 {
        let (front_margin, back_margin) = block.margin.in_axis(self.axis);

        let advance = (self.pending_margin.max(0.0).max(back_margin.max(0.0))
            + self.pending_margin.min(0.0)
            + back_margin.min(0.0))
        .max(0.0);

        if self.main_cursor - back_margin < 0.0 {
            self.main_margin.0 = self.main_margin.0.max(back_margin - self.main_cursor);
        }

        self.pending_margin = front_margin;

        self.main_cursor += advance + block.rect.support(-self.axis);

        let (start_margin, end_margin) = block.margin.in_axis(self.cross_axis);
        let placement_pos;

        if self.contain_margins {
            placement_pos =
                self.main_cursor * self.axis + (self.cross_cursor + start_margin) * self.cross_axis;

            self.line_height = self
                .line_height
                .max(block.rect.size().dot(self.cross_axis) + start_margin + end_margin);
        } else {
            placement_pos = self.main_cursor * self.axis + self.cross_cursor * self.cross_axis;

            self.cross_margin.0 = self.cross_margin.0.max(start_margin);
            self.cross_margin.1 = self.cross_margin.1.max(end_margin);
            self.line_height = self.line_height.max(block.rect.size().dot(self.cross_axis));
        }

        let extent = block.rect.support(self.axis);

        self.main_cursor += extent;

        placement_pos
    }

    /// Finishes the current line and moves the cursor to the next
    fn finish(&mut self) -> Rect {
        self.cross_cursor += self.line_height;

        self.main_margin.1 = self.main_margin.1.max(self.pending_margin);

        if self.contain_margins {
            self.main_cursor += self.pending_margin
        }

        self.pending_margin = 0.0;

        Rect::from_two_points(
            self.start,
            self.main_cursor * self.axis + self.cross_cursor * self.cross_axis,
        )
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Direction {
    #[default]
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

    fn to_edges(self, main: (f32, f32), cross: (f32, f32)) -> Edges {
        match self {
            Direction::Horizontal => Edges::new(main.0, main.1, cross.0, cross.1),
            Direction::Vertical => Edges::new(cross.0, cross.1, main.0, main.1),
            Direction::HorizontalReverse => Edges::new(main.1, main.0, cross.0, cross.1),
            Direction::VerticalReverse => Edges::new(cross.1, cross.0, main.0, main.1),
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum CrossAlign {
    #[default]
    /// Align items to the start of the cross axis
    Start,
    /// Align items to the center of the cross axis
    Center,
    /// Align items to the end of the cross axis
    End,

    /// Fill the cross axis
    Stretch,
}

impl CrossAlign {
    fn align_offset(&self, total_size: f32, size: f32) -> f32 {
        match self {
            CrossAlign::Start => 0.0,
            CrossAlign::Center => (total_size - size) / 2.0,
            CrossAlign::End => total_size - size,
            CrossAlign::Stretch => 0.0,
        }
    }
}

#[derive(Default, Debug)]
pub struct Layout {
    pub cross_align: CrossAlign,
    pub direction: Direction,
    pub contain_margins: bool,
}

impl Layout {
    /// Position and size the children of the given entity using all the provided available space
    ///
    /// Returns the inner rect
    fn apply(
        &self,
        world: &World,
        entity: &EntityRef,
        content_area: Rect,
        constraints: LayoutLimits,
    ) -> Block {
        let (axis, cross_axis) = self.direction.axis();

        let (_, total_preferred_size, _, blocks) = self.query_size(world, entity, content_area);

        // Size remaining if everything got at least its preferred size
        let total_preferred_size = total_preferred_size.size().dot(axis);
        // let preferred_remaining =
        //     (constraints.max.dot(axis) - preferred_size.size().dot(axis)).max(0.0);
        //
        // // Size remaining if everything got at least its min size
        // let min_remaining =
        // (constraints.max.dot(axis) - min_size.size().dot(axis) - preferred_remaining).max(0.0);

        // tracing::debug!(total_preferred_size, "remaining sizes");

        let available_size = constraints.max;

        // Start at the corner of the inner rect
        //
        // The inner rect is position relative to the layouts parent
        let inner_rect = content_area;

        let mut cursor = MarginCursor::new(inner_rect.min, axis, cross_axis, self.contain_margins);

        // Reset to local
        let content_area = Rect {
            min: Vec2::ZERO,
            max: inner_rect.size(),
        };

        let blocks = blocks
            .into_iter()
            .map(|(entity, block)| {
                // The size required to go from min to preferred size
                let min_size = block.min.size().dot(axis);
                let preferred_size = block.preferred.size().dot(axis);

                let to_preferred = preferred_size - min_size;
                let axis_sizing = (min_size
                    + (constraints.max.dot(axis) * (to_preferred / total_preferred_size)))
                    * axis;

                // let axis_sizing = block.preferred.rect.size() * axis;

                let child_constraints = if let CrossAlign::Stretch = self.cross_align {
                    let margin = entity.get_copy(margin()).unwrap_or_default();

                    let size = inner_rect.size().min(constraints.max) - margin.size();
                    LayoutLimits {
                        min: size * cross_axis,
                        max: size * cross_axis + axis_sizing,
                    }
                } else {
                    LayoutLimits {
                        min: Vec2::ZERO,
                        max: available_size * cross_axis + axis_sizing,
                    }
                };

                // let local_rect = widget_outer_bounds(world, &child, size);
                let block = update_subtree(
                    world,
                    &entity,
                    // Supply our whole inner content area
                    content_area,
                    child_constraints,
                );

                cursor.put(&block);

                (entity, block)
            })
            .collect_vec();

        let line = cursor.finish();

        let line_size = line.size();

        let start = match self.direction {
            Direction::Horizontal => inner_rect.min,
            Direction::Vertical => inner_rect.min,
            Direction::HorizontalReverse => vec2(inner_rect.max.x, inner_rect.min.y),
            Direction::VerticalReverse => vec2(inner_rect.min.x, inner_rect.max.y),
        };

        let mut cursor = MarginCursor::new(start, axis, cross_axis, self.contain_margins);

        for (entity, block) in blocks {
            // And move it all by the cursor position
            let height = (block.rect.size() + block.margin.size()).dot(cross_axis);

            let pos = cursor.put(&block)
                + self
                    .cross_align
                    .align_offset(line_size.dot(cross_axis), height)
                    * cross_axis;

            entity.update_dedup(components::rect(), block.rect);
            entity.update_dedup(components::local_position(), pos);
        }

        let rect = cursor.finish();

        let margin = self
            .direction
            .to_edges(cursor.main_margin, cursor.cross_margin);

        Block::new(rect, margin)
    }

    pub(crate) fn query_size<'a>(
        &self,
        world: &'a World,
        entity: &EntityRef,
        inner_rect: Rect,
    ) -> (Rect, Rect, Edges, Vec<(EntityRef<'a>, SizeQuery)>) {
        let children = entity.get(children()).ok();
        let children = children.as_ref().map(|v| v.as_slice()).unwrap_or_default();

        // let available_size = inner_rect.size();

        // Start at the corner of the inner rect
        //
        // The inner rect is position relative to the layouts parent

        let (axis, cross_axis) = self.direction.axis();

        let mut min_cursor = MarginCursor::new(Vec2::ZERO, axis, cross_axis, self.contain_margins);
        let mut preferred_cursor =
            MarginCursor::new(Vec2::ZERO, axis, cross_axis, self.contain_margins);

        // Reset to local
        let content_area = Rect {
            min: Vec2::ZERO,
            max: inner_rect.size(),
        };

        let blocks = children
            .iter()
            .map(|&child| {
                let entity = world.entity(child).expect("Invalid child");

                // let local_rect = widget_outer_bounds(world, &child, size);
                let query = query_size(world, &entity, content_area);

                min_cursor.put(&Block::new(query.min, query.margin));
                preferred_cursor.put(&Block::new(query.preferred, query.margin));
                (entity, query)
            })
            .collect_vec();

        let min_margin = self
            .direction
            .to_edges(min_cursor.main_margin, min_cursor.cross_margin);

        let preferred_margin = self
            .direction
            .to_edges(preferred_cursor.main_margin, preferred_cursor.cross_margin);

        assert_eq!(min_margin, preferred_margin);

        (
            min_cursor.finish(),
            preferred_cursor.finish(),
            min_margin,
            blocks,
        )
    }
}

pub struct SizeQuery {
    min: Rect,
    preferred: Rect,
    margin: Edges,
}

pub fn query_size(world: &World, entity: &EntityRef, content_area: Rect) -> SizeQuery {
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

        let (min, preferred, inner_margin, _) =
            layout.query_size(world, entity, content_area.inset(&padding));

        SizeQuery {
            min: min.pad(&padding),
            preferred: preferred.pad(&padding),
            margin: margin.merge(inner_margin),
        }
    }
    // Stack
    else if let Ok(children) = entity.get(children()) {
        let mut inner_min = Rect {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        };
        let mut inner_preferred = Rect {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        };

        for &child in &*children {
            let entity = world.entity(child).expect("Invalid child");

            // let local_rect = widget_outer_bounds(world, &child, size);
            let query = query_size(world, &entity, content_area);

            inner_min = inner_min.merge(query.min);
            inner_preferred = inner_preferred.merge(query.preferred);
        }

        SizeQuery {
            min: inner_min.pad(&padding),
            preferred: inner_preferred.pad(&padding),
            margin,
        }
    } else {
        let (min_size, preferred_size) = resolve_size(entity, content_area);

        let min_offset = resolve_pos(entity, content_area, min_size);
        let preferred_offset = resolve_pos(entity, content_area, preferred_size);

        // Leaf

        SizeQuery {
            min: Rect::from_size_pos(min_size, min_offset),
            preferred: Rect::from_size_pos(preferred_size, preferred_offset),
            margin,
        }
    }
}

/// Constraints for a child widget passed down from the parent.
///
/// Allows for the parent to control the size of the children, such as stretching
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutLimits {
    pub min: Vec2,
    pub max: Vec2,
}

/// A block is a rectangle and surrounding support such as margin
#[derive(Debug, Clone, Copy)]
pub(crate) struct Block {
    pub(crate) rect: Rect,
    pub(crate) margin: Edges,
}

impl Block {
    pub(crate) fn new(rect: Rect, margin: Edges) -> Self {
        Self { rect, margin }
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
    if let Ok(layout) = entity.get(layout()) {
        // For a given layout use the largest size that fits within the constraints and then
        // potentially shrink it down.

        let mut block = layout.apply(
            world,
            entity,
            content_area.inset(&padding),
            LayoutLimits {
                min: limits.min,
                max: limits.max - padding.size(),
            },
        );

        block.rect = block.rect.pad(&padding).max_size(limits.min);

        // TODO: reduce margin?
        block.margin = (block.margin - padding).max(0.0) + margin;

        block
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

            assert_eq!(content_area.size(), limits.max);
            let constraints = LayoutLimits {
                min: Vec2::ZERO,
                max: limits.max - padding.size(),
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

            entity.update_dedup(components::rect(), res.rect);
        }
        Block {
            rect: total_bounds,
            margin,
        }
    } else {
        let size = resolve_size(entity, content_area)
            .1
            .clamp(limits.min, limits.max);

        let pos = resolve_pos(entity, content_area, size);

        Block {
            rect: Rect::from_size_pos(size, pos),
            margin,
        }
    }
}

fn resolve_size(entity: &EntityRef, content_area: Rect) -> (Vec2, Vec2) {
    let parent_size = content_area.size();
    let min_size = entity
        .get(components::min_size())
        .as_deref()
        .unwrap_or(&Unit::ZERO)
        .resolve(parent_size);

    let size = if let Ok(size) = entity.get(components::size()) {
        size.resolve(parent_size)
    } else {
        entity
            .get_copy(intrinsic_size())
            .expect("intrinsic size required")
    };

    // let size = entity
    //     .get(components::size())
    //     .as_deref()
    //     .unwrap_or(&Unit::ZERO)
    //     .resolve(parent_size)
    //     .max(min_size);

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
