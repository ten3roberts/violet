use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{vec2, BVec2, Mat4, Vec2, Vec2Swizzles};

use super::{Float, Movable, Rectangle, Stack};
use crate::{
    components::{item_align, max_size, min_size, offset, rect, transform, LayoutAlignment},
    input::{interactive, on_scroll},
    state::{StateStream, StateWrite},
    style::{
        default_corner_radius, scrollbar_size, surface_interactive_accent, Background, SizeExt,
        WidgetSizeProps,
    },
    to_owned,
    unit::Unit,
    utils::zip_latest,
    Scope, Widget,
};

/// Wraps a widget in a scroll area.
///
/// Scroll bars will be shown if the content is larger than the available space.
pub struct ScrollArea<W> {
    items: W,
    directions: BVec2,
    size: WidgetSizeProps,
    background: Option<Background>,
}

impl<W> ScrollArea<W> {
    pub fn new(directions: impl Into<BVec2>, items: W) -> Self
    where
        W: Widget,
    {
        Self {
            items,
            directions: directions.into(),
            size: WidgetSizeProps::default(),
            background: None,
        }
    }

    pub fn vertical(items: W) -> Self
    where
        W: Widget,
    {
        Self {
            items,
            directions: BVec2::new(false, true),
            size: WidgetSizeProps::default(),
            background: None,
        }
    }

    pub fn horizontal(items: W) -> Self
    where
        W: Widget,
    {
        Self {
            items,
            directions: BVec2::new(true, false),
            size: WidgetSizeProps::default(),
            background: None,
        }
    }

    pub fn with_background(mut self, background: impl Into<Background>) -> Self {
        self.background = Some(background.into());
        self
    }
}

impl<W: Widget> Widget for ScrollArea<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let content_size = Mutable::new(Vec2::ZERO);
        let outer_size = Mutable::new(Vec2::ZERO);

        let scroll_pos = Mutable::new(Vec2::ZERO);
        let sensitivity = vec2(2.0, -2.0);
        scope.on_event(on_scroll(), {
            to_owned![content_size, outer_size, scroll_pos];
            move |_, scroll| {
                scroll_pos.write_mut(|v| {
                    let delta = if scroll.modifiers.shift_key() {
                        scroll.delta.perp()
                    } else {
                        scroll.delta
                    };

                    let max_scroll = (content_size.get() - outer_size.get()).max(Vec2::ZERO);
                    *v = (*v + delta * sensitivity).clamp(Vec2::ZERO, max_scroll)
                });

                None
            }
        });

        scope.set(interactive(), ());

        let stylesheet = scope.stylesheet();

        let scrollbar_size = stylesheet.get_copy(scrollbar_size()).unwrap_or_default();

        let scroll_area = {
            to_owned![scroll_pos, content_size, outer_size];
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
                    content_size,
                })
                .with_clip(self.directions)
                .with_preserve_size(BVec2::TRUE)
                .mount(scope)
            }
        };

        self.size.mount(scope);

        Stack::new((
            scroll_area,
            Scrollbar {
                size: content_size.clone(),
                outer_size: outer_size.clone(),
                scroll_pos: scroll_pos.clone(),
                axis: Vec2::X,
                scrollbar_size,
            },
            Scrollbar {
                size: content_size,
                outer_size,
                scroll_pos,
                axis: Vec2::Y,
                scrollbar_size,
            },
        ))
        .with_background_opt(self.background)
        .mount(scope)
    }
}

impl<W> SizeExt for ScrollArea<W> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}

struct Scrollbar {
    axis: Vec2,
    size: Mutable<Vec2>,
    outer_size: Mutable<Vec2>,
    scroll_pos: Mutable<Vec2>,
    scrollbar_size: f32,
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

            let perp = -self.axis.perp();

            (
                h + perp * self.scrollbar_size,
                progress - perp * self.scrollbar_size * Vec2::X,
            )
        });

        let handle = |scope: &mut Scope<'_>| {
            scope.spawn_stream(stream, move |scope, (size, pos)| {
                scope.update_dedup(min_size(), Unit::px(size)).unwrap();
                scope.update_dedup(max_size(), Unit::px(size)).unwrap();
                scope.update_dedup(offset(), Unit::px(pos)).unwrap();
            });

            Movable::new(
                Rectangle::new(surface_interactive_accent())
                    .with_min_size(Unit::px2(40.0, 40.0))
                    .with_max_size(Unit::px2(40.0, 40.0))
                    .with_corner_radius(default_corner_radius()),
            )
            .on_move(move |_, v| {
                let size = size.get();
                let outer_size = outer_size.get();
                let max_scroll = (size - outer_size).max(Vec2::ZERO);
                let new_scroll_pos = (v / outer_size * size).clamp(Vec2::ZERO, max_scroll);

                scroll_pos.write_mut(|v| {
                    let perp = self.axis.yx();
                    *v = *v * perp + new_scroll_pos * self.axis;
                });
                v
            })
            .mount(scope)
        };

        scope.set(
            item_align(),
            if self.axis == Vec2::Y {
                LayoutAlignment::top_right()
            } else {
                LayoutAlignment::bottom_left()
            },
        );

        Float::new(handle)
            // .with_background(
            //     scope
            //         .stylesheet()
            //         .get_copy(surface_accent())
            //         .unwrap_or_default()
            //         .with_alpha(0.5),
            // )
            // .with_maximize(self.axis)
            // .with_maximize(self.axis)
            .mount(scope)
    }
}

struct ScrolledContent<W> {
    items: W,
    scroll_pos: Mutable<Vec2>,
    content_size: Mutable<Vec2>,
    outer_size: Mutable<Vec2>,
}

impl<W: Widget> Widget for ScrolledContent<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        scope.spawn_stream(
            zip_latest(
                zip_latest(self.content_size.stream(), self.outer_size.stream()),
                self.scroll_pos.stream(),
            ),
            |scope, ((size, outer_size), scroll)| {
                let max_scroll = (size - outer_size).max(Vec2::ZERO);
                let scroll = scroll.clamp(Vec2::ZERO, max_scroll);
                scope.set(transform(), Mat4::from_translation(-scroll.extend(0.0)));
            },
        );

        scope.monitor(rect(), move |v| {
            if let Some(v) = v {
                self.content_size.set(v.size());
            }
        });

        Stack::new(self.items).mount(scope)
    }
}
