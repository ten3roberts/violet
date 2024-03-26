use std::sync::Arc;

use flax::{Entity, EntityRef, World};
use glam::{vec2, BVec2, Vec2};
use itertools::Itertools;

use crate::{
    components,
    layout::{
        cache::{validate_cached_row, CachedValue},
        query_size, SizingHints,
    },
    Edges, Rect,
};

use super::{
    apply_layout, cache::LayoutCache, resolve_pos, Block, Direction, LayoutLimits, QueryArgs,
    Sizing,
};

#[derive(Debug, Clone)]
struct QueryCursor {
    /// Margin of the last widget to be merged before the next
    pending_margin: f32,
    start: Vec2,
    main_cursor: f32,
    cross_cursor: f32,

    // line_height: f32,
    axis: Vec2,
    cross_axis: Vec2,
    main_margin: (f32, f32),
    cross_size: f32,
    contain_margins: bool,
}

impl QueryCursor {
    fn new(start: Vec2, axis: Vec2, cross_axis: Vec2, contain_margins: bool) -> Self {
        Self {
            // Setting this to -inf will cause the margin to leak out of the container. This would
            // be akin to having no back support for the widget to be placed against.
            pending_margin: if contain_margins { 0.0 } else { f32::MIN },
            start,
            main_cursor: start.dot(axis),
            cross_cursor: start.dot(cross_axis),
            // line_height: 0.0,
            axis,
            cross_axis,
            main_margin: (0.0, 0.0),
            contain_margins,
            cross_size: 0.0,
        }
    }

    fn put(&mut self, block: &Block) -> (Vec2, f32) {
        let (back_margin, front_margin) = block.margin.in_axis(self.axis);

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

            // *self.axis + (self.cross_cursor + start_margin) * self.cross_axis;

            cross_size = block.rect.size().dot(self.cross_axis) + start_margin + end_margin;
            self.cross_size = self.cross_size.max(cross_size);
        } else {
            placement_pos = self.main_cursor * self.axis + self.cross_cursor * self.cross_axis;
            cross_size = block.rect.size().dot(self.cross_axis);

            self.cross_size = self.cross_size.max(cross_size);
        }

        let extent = block.rect.support(self.axis);

        self.main_cursor += extent;

        (placement_pos, cross_size)
    }

    fn rect(&self) -> Rect {
        let cross_cursor = self.cross_cursor + self.cross_size;

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
        self.cross_cursor += self.cross_size;

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

#[derive(Debug, Clone)]
struct AlignCursor {
    /// Margin of the last widget to be merged before the next
    pending_margin: f32,
    start: Vec2,
    main_cursor: f32,
    cross_cursor: f32,

    // line_height: f32,
    axis: Vec2,
    cross_axis: Vec2,
    main_margin: (f32, f32),
    cross_inner: (f32, f32),
    cross_outer: (f32, f32),
    cross_size: f32,
    align: Alignment,
    contain_margins: bool,
}

impl AlignCursor {
    fn new(
        start: Vec2,
        axis: Vec2,
        cross_axis: Vec2,
        contain_margins: bool,
        cross_size: f32,
        align: Alignment,
    ) -> Self {
        Self {
            // Setting this to -inf will cause the margin to leak out of the container. This would
            // be akin to having no back support for the widget to be placed against.
            pending_margin: if contain_margins { 0.0 } else { f32::MIN },
            start,
            main_cursor: start.dot(axis),
            cross_cursor: start.dot(cross_axis),
            // line_height: 0.0,
            axis,
            cross_axis,
            main_margin: (0.0, 0.0),
            contain_margins,
            cross_inner: (0.0, 0.0),
            cross_outer: (0.0, 0.0),
            cross_size,
            align,
        }
    }

    fn put(&mut self, block: &Block) -> Vec2 {
        let (back_margin, front_margin) = block.margin.in_axis(self.axis);

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

        if self.contain_margins {
            let main_pos = self.main_cursor;

            let cross_pos = self.align.align_offset(
                self.cross_size,
                block.rect.pad(&block.margin).size().dot(self.cross_axis),
            ) + start_margin;

            // tracing::info!( main_pos, %cross_pos, ?block, "aligning");
            placement_pos =
                main_pos * self.axis + (self.cross_cursor + cross_pos) * self.cross_axis;
            // *self.axis + (self.cross_cursor + start_margin) * self.cross_axis;

            let outer = block
                .rect
                .pad(&block.margin)
                .translate(self.cross_axis + cross_pos);
            self.cross_inner.0 = self.cross_inner.0.min(outer.min.dot(self.cross_axis));
            self.cross_inner.1 = self.cross_inner.1.max(outer.max.dot(self.cross_axis));

            self.cross_outer = self.cross_inner;
            // self.line_height = self.line_height.max(cross_size);
        } else {
            let main_pos = self.main_cursor;
            let cross_pos = self
                .align
                .align_offset(self.cross_size, block.rect.size().dot(self.cross_axis))
                * self.cross_axis;

            // tracing::info!( main_pos, %cross_pos, ?block, "aligning");
            placement_pos =
                main_pos * self.axis + (self.cross_cursor + cross_pos) * self.cross_axis;

            let outer = block.rect.pad(&block.margin).translate(cross_pos);
            let inner = block.rect.translate(cross_pos);

            self.cross_inner.0 = self.cross_inner.0.min(inner.min.dot(self.cross_axis));
            self.cross_inner.1 = self.cross_inner.1.max(inner.max.dot(self.cross_axis));

            self.cross_outer.0 = self.cross_outer.0.min(outer.min.dot(self.cross_axis));
            self.cross_outer.1 = self.cross_outer.1.max(outer.max.dot(self.cross_axis));

            // self.line_height = self.line_height.max(block.rect.size().dot(self.cross_axis));
        }

        let extent = block.rect.support(self.axis);

        self.main_cursor += extent;

        placement_pos
    }

    fn rect(&self) -> Rect {
        let cross_cursor = self.cross_cursor + self.cross_size;

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
        self.cross_cursor += self.cross_size;

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

    fn cross_margin(&self) -> (f32, f32) {
        (
            self.cross_inner.0 - self.cross_outer.0,
            self.cross_outer.1 - self.cross_inner.1,
        )
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Alignment {
    #[default]
    /// Align items to the start of the cross axis
    Start,
    /// Align items to the center of the cross axis
    Center,
    /// Align items to the end of the cross axis
    End,
}

impl Alignment {
    pub fn align_offset(&self, total_size: f32, size: f32) -> f32 {
        match self {
            Alignment::Start => 0.0,
            Alignment::Center => (total_size - size) / 2.0,
            Alignment::End => total_size - size,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub(crate) min: Rect,
    pub(crate) preferred: Rect,
    pub(crate) blocks: Arc<Vec<(Entity, Sizing)>>,
    pub(crate) hints: SizingHints,
    maximize_sum: Vec2,
}

#[derive(Default, Debug, Clone)]
pub struct FlowLayout {
    pub cross_align: Alignment,
    pub stretch: bool,
    pub direction: Direction,
    pub reverse: bool,
    pub contain_margins: bool,
}

impl FlowLayout {
    /// Position and size the children of the given entity using all the provided available space
    ///
    /// Returns the inner rect
    pub(crate) fn apply(
        &self,
        world: &World,
        entity: &EntityRef,
        cache: &mut LayoutCache,
        children: &[Entity],
        content_area: Rect,
        limits: LayoutLimits,
        preferred_size: Vec2,
    ) -> Block {
        puffin::profile_function!();
        let _span =
            tracing::info_span!("Flow::apply", %content_area, ?limits, flow=?self).entered();

        // Query the minimum and preferred size of this flow layout, optimizing for minimum size in
        // the direction of this axis.
        let row = self.query_row(
            world,
            cache,
            children,
            QueryArgs {
                limits,
                content_area: content_area.size(),
                direction: self.direction,
            },
        );

        // tracing::info!(?row.margin, "row margins to be contained");
        self.distribute_children(world, entity, &row, content_area, limits, preferred_size)
    }

    fn distribute_children(
        &self,
        world: &World,
        entity: &EntityRef,
        row: &Row,
        content_area: Rect,
        limits: LayoutLimits,
        preferred_size: Vec2,
    ) -> Block {
        puffin::profile_function!();
        let (axis, cross_axis) = self.direction.as_main_and_cross(self.reverse);

        // If everything was squished as much as possible
        let minimum_inner_size = row.min.size().dot(axis);

        // If everything could take as much space as it wants
        let preferred_inner_size = row.preferred.size().dot(axis);

        // How much space there is left to distribute to the children
        let distribute_size = (preferred_inner_size - minimum_inner_size).max(0.0);
        // tracing::info!(?distribute_size);

        // Clipped maximum that we remap to
        let target_inner_size = distribute_size
            .min(limits.max_size.dot(axis) - minimum_inner_size)
            .max(0.0);

        let remaining_size = (limits.max_size.dot(axis) - preferred_inner_size).max(0.0);

        // for cross
        let available_size = limits.max_size;

        let mut cursor = QueryCursor::new(content_area.min, axis, cross_axis, self.contain_margins);

        // Reset to local
        let mut sum = 0.0;

        let cross_size = row.preferred.size().max(preferred_size).dot(cross_axis);

        let mut can_grow = BVec2::FALSE;
        // Distribute the size to the widgets and apply their layout
        let blocks = row
            .blocks
            .iter()
            .map(|(id, sizing)| {
                let entity = world.entity(*id).expect("Invalid child");
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

                let mut given_size = block_min_size + target_inner_size * ratio;
                sum += ratio;

                let maximize = sizing.maximize.dot(axis);

                if maximize > 0.0 {
                    given_size += remaining_size * (maximize / row.maximize_sum.dot(axis));
                }

                let axis_sizing = given_size * axis;

                // tracing::info!(ratio, %axis_sizing, block_min_size, target_inner_size);

                assert!(
                    axis_sizing.dot(axis) >= block_min_size,
                    "{axis_sizing} {block_min_size}"
                );

                let child_margin = if self.contain_margins {
                    sizing.margin
                } else {
                    Edges::ZERO
                };

                // Calculate hard sizing constraints and ensure the children are laid out
                // accordingly.
                //
                // The child may return a size *less* than the specified limit
                let child_limits = if self.stretch {
                    let cross_size = cross_size - child_margin.size().dot(cross_axis);
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
                let block = apply_layout(world, &entity, content_area.size(), child_limits);

                can_grow |= block.can_grow;
                tracing::debug!(?block, "updated subtree");

                cursor.put(&block);

                (entity, block)
            })
            .collect_vec();

        let line = cursor.finish();

        let line_size = line.size().max(preferred_size);

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
        let mut cursor = AlignCursor::new(
            start,
            axis,
            cross_axis,
            self.contain_margins,
            line_size.dot(cross_axis),
            self.cross_align,
        );

        // tracing::info!(?blocks, "aligning blocks");
        for (entity, block) in blocks {
            let pos = cursor.put(&block);

            tracing::info!(%pos);

            entity.update_dedup(components::rect(), block.rect);
            entity.update_dedup(components::local_position(), pos);
        }

        let rect = cursor
            .finish()
            .max_size(preferred_size)
            .max_size(limits.min_size);

        let margin =
            self.direction
                .to_edges(cursor.main_margin, cursor.cross_margin(), self.reverse);

        tracing::info!(%rect, %entity, %margin);

        Block::new(rect, margin, can_grow)
    }

    fn distribute_query(
        &self,
        world: &World,
        row: &Row,
        args: QueryArgs,
        preferred_size: Vec2,
    ) -> Sizing {
        puffin::profile_function!();
        let (axis, cross_axis) = self.direction.as_main_and_cross(self.reverse);

        // If everything was squished as much as possible
        let minimum_inner_size = row.min.size().dot(axis);

        // If everything could take as much space as it wants
        let preferred_inner_size = row.preferred.size().dot(axis);

        // How much space there is left to distribute out
        let distribute_size = (preferred_inner_size - minimum_inner_size).max(0.0);
        // tracing::info!(?distribute_size);

        // Clipped maximum that we remap to
        let target_inner_size = distribute_size
            .min(args.limits.max_size.dot(axis) - minimum_inner_size)
            .max(0.0);

        let remaining_size = args.limits.max_size.dot(axis) - preferred_inner_size;

        tracing::debug!(
            min=?row.min.size(),
            preferre2=?row.preferred.size(),
            distribute_size,
            target_inner_size,
            "distribute"
        );

        let available_size = args.limits.max_size;

        let mut min_cursor = QueryCursor::new(Vec2::ZERO, axis, cross_axis, self.contain_margins);
        let mut cursor = QueryCursor::new(Vec2::ZERO, axis, cross_axis, self.contain_margins);

        let mut sum = 0.0;

        let cross_size = row.preferred.size().max(preferred_size).dot(cross_axis);
        let mut hints = SizingHints::default();

        // Distribute the size to the widgets and apply their layout
        let blocks = row
            .blocks
            .iter()
            .map(|(id, sizing)| {
                let entity = world.entity(*id).expect("Invalid child");
                let _span = tracing::debug_span!("block", %entity).entered();
                // The size required to go from min to preferred size
                let block_min_size = sizing.min.size().dot(axis);
                let block_preferred_size = sizing.preferred.size().dot(axis);

                if block_min_size > block_preferred_size {
                    tracing::error!(
                        %entity,
                        ?block_min_size,
                        block_preferred_size,
                        "min is larger than preferred",
                    );

                    // return;
                }

                assert!(block_min_size.is_finite());
                assert!(block_preferred_size.is_finite());

                assert!(block_min_size <= block_preferred_size, "min is larger than preferred");

                let remaining = block_preferred_size - block_min_size;
                let ratio = if distribute_size == 0.0 {
                    0.0
                } else {
                    remaining / distribute_size
                };

                let mut given_size = block_min_size + target_inner_size * ratio;
                sum += ratio;

                let maximize = sizing.maximize.dot(axis);

                if maximize > 0.0 {
                    given_size =
                        given_size.max(remaining_size * (maximize / row.maximize_sum).dot(axis));
                }

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
                    let cross_size = cross_size - child_margin.size().dot(cross_axis);
                    LayoutLimits {
                        min_size: cross_size*cross_axis,
                        max_size: axis_sizing + cross_size * cross_axis,
                    }
                } else {
                    let cross_size = available_size - child_margin.size();

                    LayoutLimits {
                        min_size: Vec2::ZERO,
                        max_size: axis_sizing + cross_size * cross_axis,
                    }
                };

                // NOTE: optimize for the minimum size in the query direction, not the
                // direction of the flow
                let sizing = query_size(world, &entity, QueryArgs {
                    limits: child_limits,
                    content_area: args.content_area,
                    // Use the query direction, not the flow direction
                    direction: args.direction,
                });


                hints = hints.combine(sizing.hints);

                tracing::debug!(min=%sizing.min.size(), preferred=%sizing.preferred.size(), ?child_limits, "query");

                min_cursor.put(&Block::new(sizing.min, sizing.margin, sizing.hints.can_grow));
                cursor.put(&Block::new(sizing.preferred, sizing.margin, sizing.hints.can_grow));

                tracing::debug!(min_cursor=%min_cursor.rect().size(), cursor=%cursor.rect().size(), "cursor");
                sizing
            }).collect_vec();

        let min_rect = min_cursor.finish();
        let rect = cursor.finish().max_size(preferred_size);

        let line_size = rect.size().max(preferred_size);

        // Do layout one last time for alignment
        let mut cursor = AlignCursor::new(
            Vec2::ZERO,
            axis,
            cross_axis,
            self.contain_margins,
            line_size.dot(cross_axis),
            self.cross_align,
        );

        for block in blocks {
            cursor.put(&Block::new(
                block.preferred,
                block.margin,
                block.hints.can_grow,
            ));
        }

        let rect = cursor.finish();
        let margin =
            self.direction
                .to_edges(cursor.main_margin, cursor.cross_margin(), self.reverse);

        Sizing {
            min: min_rect.max_size(args.limits.min_size),
            preferred: rect.max_size(args.limits.min_size),
            margin,
            hints,
            maximize: row.maximize_sum,
        }
    }

    pub(crate) fn query_row(
        &self,
        world: &World,
        cache: &mut LayoutCache,
        children: &[Entity],
        args: QueryArgs,
    ) -> Row {
        puffin::profile_function!();
        if let Some(value) = cache.query_row.as_ref() {
            if validate_cached_row(value, args.limits, args.content_area) {
                // return value.value.clone();
            }
        }

        // let available_size = inner_rect.size();

        // Start at the corner of the inner rect
        //
        // The inner rect is position relative to the layouts parent

        let (axis, cross_axis) = self.direction.as_main_and_cross(self.reverse);

        let mut min_cursor = QueryCursor::new(Vec2::ZERO, axis, cross_axis, self.contain_margins);
        let mut preferred_cursor =
            QueryCursor::new(Vec2::ZERO, axis, cross_axis, self.contain_margins);

        let mut max_cross_size = 0.0f32;

        let mut hints = SizingHints::default();

        let mut maximize = Vec2::ZERO;

        let blocks = children
            .iter()
            .map(|&child| {
                let entity = world.entity(child).expect("Invalid child");

                let child_margin = if self.contain_margins {
                    query_size(
                        world,
                        &entity,
                        QueryArgs {
                            limits: LayoutLimits {
                                min_size: Vec2::ZERO,
                                max_size: args.limits.max_size,
                            },
                            content_area: args.content_area,
                            direction: self.direction,
                        },
                    )
                    .margin;
                    Edges::ZERO
                } else {
                    Edges::ZERO
                };

                let sizing = query_size(
                    world,
                    &entity,
                    QueryArgs {
                        limits: LayoutLimits {
                            min_size: Vec2::ZERO,
                            max_size: args.limits.max_size - child_margin.size(),
                        },
                        content_area: args.content_area,
                        direction: self.direction,
                    },
                );

                maximize += sizing.maximize;
                hints = hints.combine(sizing.hints);

                min_cursor.put(&Block::new(
                    sizing.min,
                    sizing.margin,
                    sizing.hints.can_grow,
                ));

                preferred_cursor.put(&Block::new(
                    sizing.preferred,
                    sizing.margin,
                    sizing.hints.can_grow,
                ));

                // NOTE: cross size is guaranteed to be fulfilled by the parent
                max_cross_size = max_cross_size.max(sizing.preferred.size().dot(cross_axis));

                (entity.id(), sizing)
            })
            .collect_vec();

        let preferred = preferred_cursor.finish();
        let min = min_cursor.finish();

        let row = Row {
            min,
            preferred,
            blocks: Arc::new(blocks),
            hints,
            maximize_sum: maximize,
        };

        cache.insert_query_row(CachedValue::new(
            args.limits,
            args.content_area,
            row.clone(),
        ));
        row
    }

    pub(crate) fn query_size(
        &self,
        world: &World,
        cache: &mut LayoutCache,
        children: &[Entity],
        args: QueryArgs,
        preferred_size: Vec2,
    ) -> Sizing {
        puffin::profile_function!(format!("{args:?}"));
        let _span = tracing::info_span!("query_size", %args.limits).entered();

        // We want to query the min/preferred size in the direction orthogonal to the flows
        // layout
        //
        // For example, returning the min/preferred height of a horizontal flow layout
        //
        // This means that we want to return the layout with the least height, as well as
        // height if we could take up as much space that we need within the given limits.
        //
        // Each child's size is of three variants:
        //     - Uncoupled: width is independent of the height
        //     - Coupled width: is dependent on the height (such as a fixed aspect ratio)
        //     - Inversely coupled: width is inversely dependent on the height, such as wrapped text. Increasing the width decreases the height
        //
        // This means that making the flow as big as possible horizontally may either lead to a
        // larger height (such as fixed aspect images), a smaller height (text does not need
        // to wrap and fit on one line), no change at all (uncoupled), or a mix thereof if the
        // children are of different types.
        //
        // The way to solve this is to query the min/preferred size of the children in *our*
        // direction, and then during distribution querying we limit the allowed width to the
        // ratio, and then query the children in the orthogonal direction to get the height to
        // be optimized. This may lead to a situation where the children are not fully
        // utilizing the space, but that is the tradeoff we make for a more predictable layout.
        // This is a small compromise, but is allowed by the layout system, as layouts today
        // can use less space in a flow than requested, such as word-wrapped text.
        //
        // The comprimise will lead to a solution that is not perfectly optimal, as there may
        // exist a better solution where some widgets may get slightly more space and still
        // fall within the max height. If anybody comes across a non-iterative solution for
        // this, be sure to let me know :)
        let row = self.query_row(
            world,
            cache,
            children,
            QueryArgs {
                direction: self.direction,
                ..args
            },
        );

        if true || row.hints.coupled_size {
            let sizing = self.distribute_query(world, &row, args, preferred_size);
            tracing::info!(?self.direction, %sizing.min, %sizing.preferred, %sizing.margin, "query");
            sizing
        } else {
            let (axis, cross) = self.direction.as_main_and_cross(self.reverse);
            let minimum_inner_size = row.min.size().dot(axis);

            let preferred_size = row.preferred.size().dot(axis);
            let to_distribute = (preferred_size - minimum_inner_size).max(0.0);

            let can_grow = to_distribute > (args.limits.max_size.dot(axis) - minimum_inner_size);

            let can_grow = if self.direction.is_horizontal() {
                BVec2::new(can_grow, false)
            } else {
                BVec2::new(false, can_grow)
            };

            let to_distribute = to_distribute
                .min(args.limits.max_size.dot(axis) - minimum_inner_size)
                .max(0.0);

            let preferred =
                (minimum_inner_size + to_distribute) * axis + row.preferred.size() * cross;

            let min = row.min.max_size(args.limits.min_size);
            let preferred = preferred.max(preferred).max(args.limits.min_size);

            let (axis, cross_axis) = self.direction.as_main_and_cross(self.reverse);
            // Do layout one last time for alignment
            let mut cursor = AlignCursor::new(
                Vec2::ZERO,
                axis,
                cross_axis,
                self.contain_margins,
                row.preferred.size().dot(cross_axis),
                self.cross_align,
            );

            for (_, block) in row.blocks.iter() {
                cursor.put(&Block::new(
                    block.preferred,
                    block.margin,
                    block.hints.can_grow,
                ));
            }

            let margin =
                self.direction
                    .to_edges(cursor.main_margin, cursor.cross_margin(), self.reverse);

            Sizing {
                min,
                preferred: Rect::from_size(preferred),
                margin,
                hints: SizingHints {
                    can_grow: can_grow | row.hints.can_grow,
                    ..row.hints
                },
                maximize: row.maximize_sum,
            }
        }
    }
}

// fn row_cross_margin(
//     cross_size: f32,
//     cross_axis: Vec2,
//     alignment: Alignment,
//     blocks: impl Iterator<Item = Rect>,
// ) -> (f32, f32) {
//     let mut cross_outer = (0f32, 0f32);
//     let mut cross_inner = (0f32, 0f32);
//     for (entity, block) in blocks {
//         // let (pos, cross_size) = cursor.put(&block);

//         let pos = self
//                 .cross_align
//                 .align_offset(line_size.dot(cross_axis), cross_size)
//                 * cross_axis;

//         let outer = block.rect.translate(pos).pad(&block.margin);
//         cross_outer.0 = cross_outer.0.min(outer.min.dot(cross_axis));
//         cross_outer.1 = cross_outer.0.max(outer.max.dot(cross_axis));

//         let inner = block.rect.translate(pos);
//         cross_inner.0 = cross_inner.0.min(inner.min.dot(cross_axis));
//         cross_inner.1 = cross_inner.1.max(inner.max.dot(cross_axis));

//         entity.update_dedup(components::rect(), block.rect);
//         entity.update_dedup(components::local_position(), pos);
//     }
// }
