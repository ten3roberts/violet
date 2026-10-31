use std::{collections::BTreeMap, ops::Range};

use flax::Entity;
use futures::StreamExt;
use glam::Vec2;

use super::{ScrollArea, ScrollState, Stack};
use crate::{
    components::offset,
    state::StateStream,
    style::{SizeExt, WidgetSizeProps},
    unit::Unit,
    utils::zip_latest,
    Scope, Widget,
};

/// Rows that may be visible at `scroll` with `viewport` height, plus one row beyond each edge
pub fn visible_range(scroll: f32, viewport: f32, row_height: f32, count: usize) -> Range<usize> {
    let end = ((((scroll + viewport) / row_height).ceil().max(0.0) as usize) + 1).min(count);
    let start = ((scroll / row_height).floor().max(0.0) as usize)
        .saturating_sub(1)
        .min(end);
    start..end
}

/// A vertically scrolled list of rows of one height that mounts only the rows in view.
///
/// Rows are built by `row` from their index as they scroll into view, and detached as they
/// leave it.
pub struct VirtualList<F> {
    count: usize,
    row_height: f32,
    row: F,
    size: WidgetSizeProps,
}

impl<F, W> VirtualList<F>
where
    F: 'static + Send + Sync + Fn(usize) -> W,
    W: 'static + Widget,
{
    /// A list of `count` rows, each `row_height` pixels high.
    ///
    /// Panics if `row_height` is not positive.
    pub fn new(count: usize, row_height: f32, row: F) -> Self {
        assert!(row_height > 0.0, "row height must be positive");
        Self {
            count,
            row_height,
            row,
            size: Default::default(),
        }
    }
}

impl<F, W> Widget for VirtualList<F>
where
    F: 'static + Send + Sync + Fn(usize) -> W,
    W: 'static + Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        let state = ScrollState::default();
        ScrollArea::vertical(VirtualRows {
            count: self.count,
            row_height: self.row_height,
            row: self.row,
            state: state.clone(),
        })
        .with_state(state)
        .with_size_props(self.size)
        .mount(scope)
    }
}

impl<F> SizeExt for VirtualList<F> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}

/// The full height of a [`VirtualList`], with the rows in view mounted at their places
struct VirtualRows<F> {
    count: usize,
    row_height: f32,
    row: F,
    state: ScrollState,
}

impl<F, W> Widget for VirtualRows<F>
where
    F: 'static + Fn(usize) -> W,
    W: 'static + Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        let Self {
            count,
            row_height,
            row,
            state,
        } = self;

        let mut mounted: BTreeMap<usize, Entity> = BTreeMap::new();

        let ranges = zip_latest(state.scroll_pos.stream(), state.viewport.stream())
            .map(move |(scroll, viewport)| visible_range(scroll.y, viewport.y, row_height, count));

        scope.spawn_stream(ranges, move |scope, range| {
            let outside = mounted
                .keys()
                .copied()
                .filter(|index| !range.contains(index))
                .collect::<Vec<_>>();

            for index in outside {
                if let Some(id) = mounted.remove(&index) {
                    scope.detach(id);
                }
            }

            for index in range {
                mounted.entry(index).or_insert_with(|| {
                    scope.attach(VirtualRow {
                        index,
                        height: row_height,
                        inner: row(index),
                    })
                });
            }
        });

        Stack::new(())
            .with_min_size(Unit::px2(0.0, count as f32 * row_height))
            .mount(scope);
    }
}

/// One row of a [`VirtualList`], placed at its index
struct VirtualRow<W> {
    index: usize,
    height: f32,
    inner: W,
}

impl<W: Widget> Widget for VirtualRow<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        scope.set(offset(), Unit::px2(0.0, self.index as f32 * self.height));

        Stack::new(self.inner)
            .with_min_size(Unit::px2(0.0, self.height))
            .with_max_size(Unit::px2(f32::MAX, self.height))
            .with_maximize(Vec2::X)
            .mount(scope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_range_top() {
        assert_eq!(visible_range(0.0, 100.0, 20.0, 1000), 0..6);
    }

    #[test]
    fn visible_range_scrolled() {
        assert_eq!(visible_range(210.0, 100.0, 20.0, 1000), 9..17);
    }

    #[test]
    fn visible_range_short() {
        assert_eq!(visible_range(0.0, 100.0, 20.0, 3), 0..3);
    }

    #[test]
    fn visible_range_empty() {
        assert_eq!(visible_range(0.0, 100.0, 20.0, 0), 0..0);
    }
}
