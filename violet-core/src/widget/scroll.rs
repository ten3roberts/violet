use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{vec2, BVec2, Mat4, Vec2, Vec2Swizzles};

use crate::{
    components::{min_size, offset, padding, rect, transform},
    input::{focusable, on_scroll},
    state::{StateMut, StateStream},
    style::{interactive_active, SizeExt},
    to_owned,
    unit::Unit,
    utils::zip_latest,
    Edges, Scope, Widget,
};

use super::{Float, Movable, Rectangle, Stack};

/// Wraps a widget in a scroll area.
///
/// Scroll bars will be shown if the content is larger than the available space.
pub struct ScrollArea<W> {
    items: W,
    directions: BVec2,
}

impl<W> ScrollArea<W> {
    pub fn new(directions: impl Into<BVec2>, items: W) -> Self
    where
        W: Widget,
    {
        Self {
            items,
            directions: directions.into(),
        }
    }

    pub fn vertical(items: W) -> Self
    where
        W: Widget,
    {
        Self {
            items,
            directions: BVec2::new(false, true),
        }
    }

    pub fn horizontal(items: W) -> Self
    where
        W: Widget,
    {
        Self {
            items,
            directions: BVec2::new(true, false),
        }
    }
}

const SCROLLBAR_SIZE: f32 = 8.0;

impl<W: Widget> Widget for ScrollArea<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let size = Mutable::new(Vec2::ZERO);
        let outer_size = Mutable::new(Vec2::ZERO);

        let scroll_pos = Mutable::new(Vec2::ZERO);
        let sensitivity = vec2(1.0, -1.0);
        scope.on_event(on_scroll(), {
            to_owned![size, outer_size, scroll_pos];
            move |_, scroll| {
                scroll_pos.write_mut(|v| {
                    let delta = if scroll.modifiers.shift_key() {
                        scroll.delta.perp()
                    } else {
                        scroll.delta
                    };

                    let max_scroll = (size.get() - outer_size.get()).max(Vec2::ZERO);
                    *v = (*v + delta * sensitivity).clamp(Vec2::ZERO, max_scroll)
                });
            }
        });

        scope.set(focusable(), ());

        let scroll = zip_latest(size.stream(), outer_size.stream()).map(|(size, outer_size)| {
            // tracing::info!(%size, %outer_size);
            size.cmpgt(outer_size)
        });

        scope.spawn_stream(scroll, |scope, needs_scroll| {
            // tracing::info!(%needs_scroll);
            scope
                .update_dedup(
                    padding(),
                    Edges::new(
                        0.0,
                        needs_scroll.y as u32 as f32 * SCROLLBAR_SIZE,
                        0.0,
                        needs_scroll.x as u32 as f32 * SCROLLBAR_SIZE,
                    ),
                )
                .unwrap();
        });

        let padded_scroll_area = {
            to_owned![scroll_pos, size, outer_size];
            move |scope: &mut Scope| {
                scope.monitor(rect(), {
                    to_owned![outer_size];
                    move |v| {
                        if let Some(v) = v {
                            outer_size.set(v.size());
                        }
                    }
                });

                Stack::new(ScrolledContent {
                    items: self.items,
                    scroll_pos,
                    outer_size,
                    size,
                })
                .with_clip(self.directions)
                .mount(scope)
            }
        };

        Stack::new((
            padded_scroll_area,
            Scrollbar {
                size: size.clone(),
                outer_size: outer_size.clone(),
                scroll_pos: scroll_pos.clone(),
                axis: Vec2::X,
            },
            Scrollbar {
                size,
                outer_size,
                scroll_pos,
                axis: Vec2::Y,
            },
        ))
        .with_padding(Edges {
            left: 0.0,
            right: SCROLLBAR_SIZE,
            top: 0.0,
            bottom: SCROLLBAR_SIZE,
        })
        .mount(scope)
    }
}

struct Scrollbar {
    axis: Vec2,
    size: Mutable<Vec2>,
    outer_size: Mutable<Vec2>,
    scroll_pos: Mutable<Vec2>,
}

impl Widget for Scrollbar {
    fn mount(self, scope: &mut Scope<'_>) {
        let scroll_pos = self.scroll_pos.clone();
        let outer_size = self.outer_size.clone();
        let size = self.size.clone();

        let stream = zip_latest(
            zip_latest(size.stream(), outer_size.stream()),
            scroll_pos.stream(),
        )
        .map(move |((size, outer_size), pos)| {
            if size.dot(self.axis) <= outer_size.dot(self.axis) {
                return (Vec2::ZERO, Vec2::ZERO);
            }

            let h = (outer_size / size * outer_size)
                .min(outer_size)
                .max(Vec2::splat(4.0))
                * self.axis;

            let progress = ((pos / size) * outer_size).min(outer_size - h) * self.axis;

            let perp = vec2(self.axis.y, self.axis.x);

            (h + perp * SCROLLBAR_SIZE, progress + outer_size * perp)
        });

        let handle = |scope: &mut Scope<'_>| {
            scope.spawn_stream(stream, |scope, (size, pos)| {
                // tracing::info!(%size, %pos, "scrollbar");
                scope.update_dedup(min_size(), Unit::px(size)).unwrap();
                scope.update_dedup(offset(), Unit::px(pos)).unwrap();
            });

            Movable::new(Rectangle::new(interactive_active()).with_min_size(Unit::px2(40.0, 40.0)))
                .on_move(move |_, v| {
                    let size = size.get();
                    let outer_size = outer_size.get();
                    let max_scroll = (size - outer_size).max(Vec2::ZERO);
                    let new_scroll_pos = (v / outer_size * size).clamp(Vec2::ZERO, max_scroll);
                    // tracing::info!(%new_scroll_pos, %outer_size, %size, "moved");
                    scroll_pos.write_mut(|v| {
                        let perp = self.axis.yx();
                        *v = *v * perp + new_scroll_pos * self.axis;
                    });
                    v
                })
                .mount(scope)
        };

        Float::new(handle)
            // .with_background(Background::new(interactive_passive()))
            // .with_maximize(self.axis)
            .mount(scope)
    }
}

struct ScrolledContent<W> {
    items: W,
    scroll_pos: Mutable<Vec2>,
    size: Mutable<Vec2>,
    outer_size: Mutable<Vec2>,
}

impl<W: Widget> Widget for ScrolledContent<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        scope.spawn_stream(
            zip_latest(
                zip_latest(self.size.stream(), self.outer_size.stream()),
                self.scroll_pos.stream(),
            ),
            |scope, ((size, outer_size), scroll)| {
                // tracing::info!(%scroll, %size);
                let max_scroll = (size - outer_size).max(Vec2::ZERO);
                let scroll = scroll.clamp(Vec2::ZERO, max_scroll);
                scope.set(transform(), Mat4::from_translation(-scroll.extend(0.0)));
            },
        );

        scope.monitor(rect(), move |v| {
            if let Some(v) = v {
                self.size.set(v.size());
            }
        });

        Stack::new(self.items).mount(scope)
    }
}
