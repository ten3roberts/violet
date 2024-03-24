use futures_signals::signal::Mutable;
use glam::{vec2, Mat4, Vec2};

use crate::{
    components::{rect, transform},
    input::{focusable, on_scroll},
    state::{StateMut, StateStream},
    style::{interactive_active, Background},
    to_owned,
    utils::zip_latest,
    Scope, Widget,
};

use super::Stack;

pub struct Scroll<W> {
    items: W,
}

impl<W> Scroll<W> {
    pub fn new(items: W) -> Self {
        Self { items }
    }
}

impl<W: Widget> Widget for Scroll<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let size = Mutable::new(Vec2::ZERO);

        let scroll_pos = Mutable::new(Vec2::ZERO);
        let sensitivity = vec2(32.0, -32.0);
        scope.on_event(on_scroll(), {
            to_owned![size, scroll_pos];
            move |_, scroll| {
                scroll_pos.write_mut(|v| {
                    *v = (*v + scroll.delta * sensitivity).clamp(Vec2::ZERO, size.get())
                });
            }
        });

        scope.set(focusable(), ());

        Stack::new(ScrolledContent {
            items: self.items,
            scroll_pos,
            size,
        })
        .with_clip(true)
        .mount(scope)
    }
}

struct ScrolledContent<W> {
    items: W,
    scroll_pos: Mutable<Vec2>,
    size: Mutable<Vec2>,
}

impl<W: Widget> Widget for ScrolledContent<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        scope.spawn_stream(
            zip_latest(self.size.stream(), self.scroll_pos.stream()),
            |scope, (size, scroll)| {
                tracing::info!(%scroll, %size);
                let scroll = scroll.clamp(Vec2::ZERO, size);
                scope.set(transform(), Mat4::from_translation(-scroll.extend(0.0)));
            },
        );

        scope.monitor(rect(), move |v| {
            if let Some(v) = v {
                self.size.set(v.size());
            }
        });

        Stack::new(self.items)
            .with_background(Background::new(interactive_active()))
            .mount(scope)
    }
}
