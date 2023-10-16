use flax::{EntityRef, World};
use glam::{vec2, Vec2};
use itertools::Itertools;

use crate::{
    components::{self, children, margin, Edges, Rect},
    layout::query_size,
};

use super::{update_subtree, Block, LayoutLimits, SizeQuery};

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
pub struct Flow {
    pub cross_align: CrossAlign,
    pub direction: Direction,
    pub contain_margins: bool,
}

impl Flow {
    /// Position and size the children of the given entity using all the provided available space
    ///
    /// Returns the inner rect
    pub(crate) fn apply(
        &self,
        world: &World,
        entity: &EntityRef,
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        let _span = tracing::info_span!("Flow::apply", ?limits, flow=?self).entered();
        let (axis, cross_axis) = self.direction.axis();

        let (_, total_preferred_size, _, blocks) = self.query_size(world, entity, content_area);

        // Size remaining if everything got at least its preferred size
        let total_preferred_size = total_preferred_size.size().dot(axis);

        let available_size = limits.max_size;

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

        let mut sum = 0.0;

        let blocks = blocks
            .into_iter()
            .map(|(entity, block)| {
                // The size required to go from min to preferred size
                let min_size = block.min.size().dot(axis);
                let preferred_size = block.preferred.size().dot(axis);

                let to_preferred = preferred_size - min_size;
                let ratio = to_preferred / total_preferred_size;
                tracing::info!("sizing: {}", ratio);
                sum += ratio;
                let axis_sizing = (min_size
                    + (limits.max_size.dot(axis) * (to_preferred / total_preferred_size)))
                    * axis;

                let child_constraints = if let CrossAlign::Stretch = self.cross_align {
                    let margin = entity.get_copy(margin()).unwrap_or_default();

                    let size = inner_rect.size().min(limits.max_size) - margin.size();
                    LayoutLimits {
                        min_size: size * cross_axis,
                        max_size: size * cross_axis + axis_sizing,
                    }
                } else {
                    LayoutLimits {
                        min_size: Vec2::ZERO,
                        max_size: available_size * cross_axis + axis_sizing,
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

        tracing::info!(sum, "sum");

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
