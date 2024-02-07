use flax::{Entity, EntityRef, World};
use glam::{vec2, Vec2};
use itertools::Itertools;

use crate::{
    components::{self, Edges, Rect},
    layout::query_size,
};

use super::{resolve_pos, update_subtree, Block, Direction, LayoutLimits, Sizing};

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

    fn rect(&self) -> Rect {
        let cross_cursor = self.cross_cursor + self.line_height;

        // tracing::debug!(?self.main_margin);

        let main_cursor = if self.contain_margins {
            self.main_cursor + self.pending_margin
        } else {
            self.main_cursor
        };

        Rect::from_two_points(
            self.start,
            main_cursor * self.axis + cross_cursor * self.cross_axis,
        )
    }

    /// Finishes the current line and moves the cursor to the next
    fn finish(&mut self) -> Rect {
        self.cross_cursor += self.line_height;

        // tracing::debug!(?self.main_margin);

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

impl<'a> Row<'a> {
    pub(crate) fn sizing(&self) -> Sizing {
        Sizing {
            min: self.min,
            preferred: self.preferred,
            margin: self.margin,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct FlowLayout {
    pub cross_align: CrossAlign,
    pub stretch: bool,
    pub direction: Direction,
    pub reverse: bool,
    pub contain_margins: bool,
    pub proportional_growth: bool,
}

impl FlowLayout {
    /// Position and size the children of the given entity using all the provided available space
    ///
    /// Returns the inner rect
    pub(crate) fn apply(
        &self,
        world: &World,
        entity: &EntityRef,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Block {
        let _span = tracing::debug_span!("Flow::apply", ?limits, flow=?self).entered();

        // Query the minimum and preferred size of this flow layout, optimizing for minimum size in
        // the direction of this axis.
        let row = self.query_row(world, children, content_area, limits, self.direction);

        // tracing::info!(?row.margin, "row margins to be contained");
        self.distribute_children(world, entity, &row, content_area, limits, false)
    }

    fn distribute_children(
        &self,
        world: &World,
        entity: &EntityRef,
        row: &Row<'_>,
        content_area: Rect,
        limits: LayoutLimits,
        min: bool,
    ) -> Block {
        let (axis, cross_axis) = self.direction.axis(self.reverse);

        // If everything was squished as much as possible
        let minimum_inner_size = row.min.size().dot(axis);

        // if minimum_inner_size > limits.max_size.dot(axis) {
        //     tracing::error!(
        //         ?minimum_inner_size,
        //         ?limits.max_size,
        //         "minimum inner size exceeded max size",
        //     );
        // }

        // If everything could take as much space as it wants
        let preferred_inner_size = row.preferred.size().dot(axis);

        // if minimum_inner_size > preferred_inner_size {
        //     tracing::error!(
        //         ?minimum_inner_size,
        //         ?preferred_inner_size,
        //         "minimum inner size exceeded preferred size",
        //     );
        // }

        // How much space there is left to distribute out
        let distribute_size = (preferred_inner_size - minimum_inner_size).max(0.0);
        // tracing::info!(?distribute_size);

        // Clipped maximum that we remap to
        let target_inner_size = distribute_size
            .min(limits.max_size.dot(axis) - minimum_inner_size)
            .max(0.0);

        // tracing::info!(
        //     ?row.preferred,
        //     distribute_size,
        //     target_inner_size,
        //     blocks = row.blocks.len(),
        //     "query size"
        // );

        let available_size = limits.max_size;

        let mut cursor =
            MarginCursor::new(content_area.min, axis, cross_axis, self.contain_margins);

        // Reset to local
        let mut sum = 0.0;

        // Distribute the size to the widgets and apply their layout
        let blocks = row
            .blocks
            .iter()
            .map(|(entity, sizing)| {
                let _span = tracing::debug_span!("block", %entity).entered();
                // The size required to go from min to preferred size
                let block_min_size = sizing.min.size().dot(axis);
                let block_preferred_size = sizing.preferred.size().dot(axis);

                if block_min_size > block_preferred_size {
                    tracing::error!(
                        ?block_min_size,
                        block_preferred_size,
                        "min is larger than preferred",
                    );
                }

                assert!(block_min_size.is_finite());
                assert!(block_preferred_size.is_finite());
                let remaining = block_preferred_size - block_min_size;
                let ratio = if distribute_size == 0.0 {
                    0.0
                } else {
                    remaining / distribute_size
                };

                let given_size = block_min_size + target_inner_size * ratio;
                tracing::debug!(
                    remaining,
                    distribute_size,
                    ratio,
                    given_size,
                    target_inner_size,
                    block_min_size,
                    "block"
                );

                sum += ratio;

                let axis_sizing = given_size * axis;
                // tracing::info!(ratio, %axis_sizing, block_min_size, target_inner_size);

                assert!(
                    axis_sizing.dot(axis) >= block_min_size,
                    "{axis_sizing} {block_min_size}"
                );
                // tracing::info!(%axis_sizing, block_min_size, remaining, "sizing: {}", ratio);

                let child_margin = if self.contain_margins {
                    sizing.margin
                } else {
                    Edges::ZERO
                };

                // Calculate hard sizing constraints ensure the children are laid out
                // accordingly.
                //
                // The child may return a size *less* than the specified limit
                let child_limits = if self.stretch {
                    let cross_size = content_area.size().min(limits.max_size) - child_margin.size();
                    LayoutLimits {
                        min_size: cross_size * cross_axis,
                        max_size: axis_sizing + cross_size * cross_axis,
                    }
                } else {
                    let cross_size = available_size - child_margin.size();

                    LayoutLimits {
                        min_size: Vec2::ZERO,
                        max_size: axis_sizing + cross_size * cross_axis,
                    }
                };

                // let local_rect = widget_outer_bounds(world, &child, size);
                let block = update_subtree(world, entity, content_area.size(), child_limits);

                tracing::debug!(?block, "updated subtree");

                // block.rect = block
                //     .rect
                //     .clamp_size(child_limits.min_size, child_limits.max_size);

                if block.rect.size().x > child_limits.max_size.x
                    || block.rect.size().y > child_limits.max_size.y
                {
                    tracing::error!(
                        block_min_size,
                        block_preferred_size,
                        "child {} exceeded max size: {:?} > {:?}",
                        entity,
                        block.rect.size(),
                        child_limits.max_size,
                    );
                }

                cursor.put(&block);

                (entity, block)
            })
            .collect_vec();

        let line = cursor.finish();

        let line_size = line.size().max(limits.min_size);

        // Apply alignment offsets
        let start = match (self.direction, self.reverse) {
            (Direction::Horizontal, false) => content_area.min,
            (Direction::Vertical, false) => content_area.min,
            (Direction::Horizontal, true) => vec2(content_area.max.x, content_area.min.y),
            (Direction::Vertical, true) => vec2(content_area.min.x, content_area.max.y),
        };

        let offset = resolve_pos(entity, content_area.size(), line_size);
        let start = start + offset;

        // Do layout one last time for alignment
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

            entity.update_dedup(components::rect(), block.rect);
            entity.update_dedup(components::local_position(), pos);
        }

        let rect = cursor.finish().clamp_size(limits.min_size, limits.max_size);

        let margin = self
            .direction
            .to_edges(cursor.main_margin, cursor.cross_margin, self.reverse);

        Block::new(rect, margin)
    }

    fn distribute_query(
        &self,
        world: &World,
        row: &Row<'_>,
        content_area: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> Sizing {
        let (axis, cross_axis) = self.direction.axis(self.reverse);

        // If everything was squished as much as possible
        let minimum_inner_size = row.min.size().dot(axis);

        // if minimum_inner_size > limits.max_size.dot(axis) {
        //     tracing::error!(
        //         ?minimum_inner_size,
        //         ?limits.max_size,
        //         "minimum inner size exceeded max size",
        //     );
        // }

        // If everything could take as much space as it wants
        let preferred_inner_size = row.preferred.size().dot(axis);

        // if minimum_inner_size > preferred_inner_size {
        //     tracing::error!(
        //         ?minimum_inner_size,
        //         ?preferred_inner_size,
        //         "minimum inner size exceeded preferred size",
        //     );
        // }

        // How much space there is left to distribute out
        let distribute_size = (preferred_inner_size - minimum_inner_size).max(0.0);
        // tracing::info!(?distribute_size);

        // Clipped maximum that we remap to
        let target_inner_size = distribute_size
            .min(limits.max_size.dot(axis) - minimum_inner_size)
            .max(0.0);

        tracing::debug!(
            min=?row.min.size(),
            preferre2=?row.preferred.size(),
            distribute_size,
            target_inner_size,
            "distribute"
        );

        let available_size = limits.max_size;

        let mut min_cursor =
            MarginCursor::new(content_area.min, axis, cross_axis, self.contain_margins);
        let mut cursor =
            MarginCursor::new(content_area.min, axis, cross_axis, self.contain_margins);

        let mut sum = 0.0;

        // Distribute the size to the widgets and apply their layout
        let blocks = row
            .blocks
            .iter()
            .map(|(entity, sizing)| {
                let _span = tracing::debug_span!("block", %entity).entered();
                // The size required to go from min to preferred size
                let block_min_size = sizing.min.size().dot(axis);
                let block_preferred_size = sizing.preferred.size().dot(axis);

                if block_min_size > block_preferred_size {
                    tracing::error!(
                        ?block_min_size,
                        block_preferred_size,
                        "min is larger than preferred",
                    );
                }

                assert!(block_min_size.is_finite());
                assert!(block_preferred_size.is_finite());

                let remaining = block_preferred_size - block_min_size;
                let ratio = if distribute_size == 0.0 {
                    0.0
                } else {
                    remaining / distribute_size
                };

                let given_size = block_min_size + target_inner_size * ratio;
                // tracing::debug!(
                //     remaining,
                //     distribute_size,
                //     ratio,
                //     given_size,
                //     target_inner_size,
                //     block_min_size,
                //     "block"
                // );

                sum += ratio;

                let axis_sizing = given_size * axis;
                // tracing::info!(ratio, %axis_sizing, block_min_size, target_inner_size);

                assert!(
                    axis_sizing.dot(axis) >= block_min_size,
                    "{axis_sizing} {block_min_size}"
                );
                // // tracing::info!(%axis_sizing, block_min_size, remaining, "sizing: {}", ratio);

                let child_margin = if self.contain_margins {
                    sizing.margin
                } else {
                    Edges::ZERO
                };

                // Calculate hard sizing constraints ensure the children are laid out
                // accordingly.
                //
                // The child may return a size *less* than the specified limit
                let child_limits = if self.stretch {
                    let cross_size = content_area.size().min(limits.max_size) - child_margin.size();
                    LayoutLimits {
                        min_size: cross_size * cross_axis,
                        max_size: axis_sizing + cross_size * cross_axis,
                    }
                } else {
                    let cross_size = available_size - child_margin.size();

                    LayoutLimits {
                        min_size: Vec2::ZERO,
                        max_size: axis_sizing + cross_size * cross_axis,
                    }
                };

                // let local_rect = widget_outer_bounds(world, &child, size);
                let block = query_size(world, entity, content_area.size(), child_limits, squeeze);

                tracing::debug!(min=%block.min.size(), preferred=%block.preferred.size(), ?child_limits, "query");

                if block.preferred.size().x > child_limits.max_size.x
                    || block.preferred.size().y > child_limits.max_size.y
                {
                    tracing::error!(
                        %entity,
                        block_min_size,
                        block_preferred_size,
                        "Widget exceeded max size: {:?} > {:?}",
                        block.preferred.size(),
                        child_limits.max_size,
                    );
                }

                min_cursor.put(&Block::new(block.min, block.margin));
                cursor.put(&Block::new(block.preferred, block.margin));

                tracing::debug!(min_cursor=%min_cursor.rect().size(), cursor=%cursor.rect().size(), "cursor");

                (entity, block)
            })
            .collect_vec();

        let min_rect = min_cursor.finish();
        let rect = cursor.finish();

        let margin = self
            .direction
            .to_edges(cursor.main_margin, cursor.cross_margin, self.reverse);

        if (rect.size().x > limits.max_size.x || rect.size().y > limits.max_size.y) && !self.stretch
        {
            tracing::error!(
                %axis,
                "Preferred size exceeded max size, preferred: {:?} max: {:?}",
                rect.size(),
                limits.max_size
            );
        }
        Sizing {
            min: min_rect,
            preferred: rect,
            margin,
        }
    }

    pub(crate) fn query_row<'a>(
        &self,
        world: &'a World,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> Row<'a> {
        // let available_size = inner_rect.size();

        // Start at the corner of the inner rect
        //
        // The inner rect is position relative to the layouts parent

        let (axis, cross_axis) = self.direction.axis(self.reverse);

        let mut min_cursor = MarginCursor::new(Vec2::ZERO, axis, cross_axis, self.contain_margins);
        let mut preferred_cursor =
            MarginCursor::new(Vec2::ZERO, axis, cross_axis, self.contain_margins);

        let blocks = children
            .iter()
            .map(|&child| {
                let entity = world.entity(child).expect("Invalid child");

                let child_margin = if self.contain_margins {
                    query_size(world, &entity, content_area.size(), limits, squeeze).margin
                } else {
                    Edges::ZERO
                };

                let sizing = query_size(world, &entity, content_area.size(), limits, squeeze);

                min_cursor.put(&Block::new(sizing.min, sizing.margin));
                preferred_cursor.put(&Block::new(sizing.preferred, sizing.margin));
                (entity, sizing)
            })
            .collect_vec();

        // let min_margin = self
        //     .direction
        //     .to_edges(min_cursor.main_margin, min_cursor.cross_margin);

        let preferred = preferred_cursor.finish();
        let min = min_cursor.finish();
        // assert!(
        //     preferred.size().x <= content_area.size().x
        //         && preferred.size().x <= content_area.size().y,
        //     "preferred size exceeded content area, preferred: {:?} content: {:?}",
        //     preferred.size(),
        //     content_area.size()
        // );

        let preferred_margin = self.direction.to_edges(
            preferred_cursor.main_margin,
            preferred_cursor.cross_margin,
            self.reverse,
        );

        // assert_le!(preferred.size().x, limits.max_size.x);
        // assert_le!(preferred.size().y, limits.max_size.y);
        // assert_le!(min.size().x, limits.max_size.x);
        // assert_le!(min.size().y, limits.max_size.y);

        // assert_eq!(min_margin, preferred_margin);

        Row {
            min,
            preferred,
            margin: preferred_margin,
            blocks,
        }
    }

    pub(crate) fn query_size<'a>(
        &self,
        world: &'a World,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> Sizing {
        let _span =
            tracing::debug_span!("Flow::query_size", ?limits, flow=?self, ?squeeze).entered();
        let row = self.query_row(world, children, content_area, limits, self.direction);

        let block = self.distribute_query(world, &row, content_area, limits, squeeze);
        tracing::debug!(?self.direction, ?block, "query");

        block
    }
}
