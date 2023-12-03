use flax::{EntityRef, World};
use glam::{vec2, Vec2};
use itertools::Itertools;

use crate::{
    components::{self, children, margin, Edges, Rect},
    layout::query_size,
};

use super::{update_subtree, Block, LayoutLimits, Sizing};

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

    fn put(&mut self, block: &Block) -> (Vec2, f32) {
        let (front_margin, back_margin) = block.margin.in_axis(self.axis);

        let advance = (self.pending_margin.max(0.0).max(back_margin.max(0.0))
            + self.pending_margin.min(0.0)
            + back_margin.min(0.0))
        .max(0.0);

        if self.main_cursor - (back_margin - advance) < 0.0 {
            self.main_margin.0 = self
                .main_margin
                .0
                .max((back_margin - advance) - self.main_cursor);
        }

        self.pending_margin = front_margin;

        self.main_cursor += advance; // + block.rect.support(-self.axis);

        // Cross axis margin calculation
        let (start_margin, end_margin) = block.margin.in_axis(self.cross_axis);

        let placement_pos;
        let cross_size;

        if self.contain_margins {
            placement_pos =
                self.main_cursor * self.axis + (self.cross_cursor + start_margin) * self.cross_axis;

            cross_size = block.rect.size().dot(self.cross_axis) + start_margin + end_margin;
            self.line_height = self.line_height.max(cross_size);
        } else {
            placement_pos = self.main_cursor * self.axis + self.cross_cursor * self.cross_axis;
            cross_size = block.rect.size().dot(self.cross_axis);

            self.cross_margin.0 = self.cross_margin.0.max(start_margin);
            self.cross_margin.1 = self.cross_margin.1.max(end_margin);
            self.line_height = self.line_height.max(block.rect.size().dot(self.cross_axis));
        }

        let extent = block.rect.support(self.axis);

        self.main_cursor += extent;

        (placement_pos, cross_size)
    }

    /// Finishes the current line and moves the cursor to the next
    fn finish(&mut self) -> Rect {
        self.cross_cursor += self.line_height;

        tracing::debug!(?self.main_margin);

        if self.contain_margins {
            self.main_cursor += self.pending_margin;
        } else {
            self.main_margin.1 = self.main_margin.1.max(self.pending_margin);
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
}

impl CrossAlign {
    pub fn align_offset(&self, total_size: f32, size: f32) -> f32 {
        match self {
            CrossAlign::Start => 0.0,
            CrossAlign::Center => (total_size - size) / 2.0,
            CrossAlign::End => total_size - size,
        }
    }
}

pub(crate) struct Row<'a> {
    pub(crate) min: Rect,
    pub(crate) preferred: Rect,
    pub(crate) margin: Edges,
    pub(crate) blocks: Vec<(EntityRef<'a>, Sizing)>,
}

#[derive(Default, Debug)]
pub struct Flow {
    pub cross_align: CrossAlign,
    pub stretch: bool,
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

        let row = self.query_size(world, entity, content_area);

        tracing::info!(?row.margin, "row margins to be contained");
        if self.contain_margins {}

        // If everything was squished as much as possible
        let minimum_inner_size = row.min.size().dot(axis);
        // If everything could take as much space as it wants
        let preferred_inner_size = row.preferred.size().dot(axis);

        // How much space there is left to distribute out
        let distribute_size = preferred_inner_size - minimum_inner_size;

        // Clipped maximum that we remap to
        let target_inner_size = distribute_size
            .min(limits.max_size.dot(axis) - minimum_inner_size)
            .max(0.0);

        tracing::info!(
            ?row.preferred,
            distribute_size,
            target_inner_size,
            blocks = row.blocks.len(),
            "query size"
        );

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

        let blocks = row
            .blocks
            .into_iter()
            .map(|(entity, sizing)| {
                // The size required to go from min to preferred size
                let block_min_size = sizing.min.size().dot(axis);
                let block_preferred_size = sizing.preferred.size().dot(axis);

                let remaining = block_preferred_size - block_min_size;
                let ratio = remaining / distribute_size;

                sum += ratio;

                let axis_sizing = (block_min_size + (target_inner_size * ratio)) * axis;
                // tracing::info!(%axis_sizing, block_min_size, remaining, "sizing: {}", ratio);

                let child_margin = if self.contain_margins {
                    /// TODO recursively save the margin for transitive uncontained margins
                    sizing.margin
                    // entity.get_copy(margin()).unwrap_or_default()
                } else {
                    Edges::ZERO
                };

                let child_limits = if self.stretch {
                    let cross_size = inner_rect.size().min(limits.max_size) - child_margin.size();
                    LayoutLimits {
                        min_size: cross_size * cross_axis,
                        max_size: axis_sizing + cross_size * cross_axis,
                    }
                } else {
                    tracing::info!(?available_size);
                    let cross_size = available_size - child_margin.size();

                    LayoutLimits {
                        min_size: Vec2::ZERO,
                        max_size: axis_sizing + cross_size * cross_axis,
                    }
                };

                // let local_rect = widget_outer_bounds(world, &child, size);
                let block = update_subtree(
                    world,
                    &entity,
                    // Supply our whole inner content area
                    content_area,
                    child_limits,
                );

                if block.rect.size().x > child_limits.max_size.x
                    || block.rect.size().y > child_limits.max_size.y
                {
                    tracing::warn!(
                        "child {} exceeded max size: {:?} > {:?}",
                        entity,
                        block.rect.size(),
                        child_limits.max_size
                    );
                }

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
            // let _span = tracing::info_span!("put", %entity).entered();
            // And move it all by the cursor position
            // let height = (block.rect.size() + block.margin.size()).dot(cross_axis);

            let (pos, cross_size) = cursor.put(&block);

            let pos = pos
                + self
                    .cross_align
                    .align_offset(line_size.dot(cross_axis), cross_size)
                    * cross_axis;

            // tracing::info!(%pos, cross_size, ?block.rect);
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
    ) -> Row<'a> {
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
                // tracing::info!("query: {:?}", query);

                min_cursor.put(&Block::new(query.min, query.margin));
                preferred_cursor.put(&Block::new(query.preferred, query.margin));
                (entity, query)
            })
            .collect_vec();

        // let min_margin = self
        //     .direction
        //     .to_edges(min_cursor.main_margin, min_cursor.cross_margin);

        let preferred = preferred_cursor.finish();
        let min = min_cursor.finish();

        let preferred_margin = self
            .direction
            .to_edges(preferred_cursor.main_margin, preferred_cursor.cross_margin);

        // assert_eq!(min_margin, preferred_margin);

        Row {
            min,
            preferred,
            margin: preferred_margin,
            blocks,
        }
    }
}
