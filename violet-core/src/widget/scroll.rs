use futures_signals::signal::Mutable;
use glam::{vec2, Mat4, Vec2};

use crate::{
    components::{rect, transform},
    input::on_scroll,
    state::StateStream,
    to_owned, Scope, Widget,
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
        scope.on_event(on_scroll(), {
            to_owned![scroll_pos];
            move |_, scroll| {
                scroll_pos.set(vec2(scroll.scroll_x, scroll.scroll_y));
            }
        });

        Stack::new(ScrolledContent {
            items: self.items,
            scroll_pos,
            size,
        })
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
        scope.monitor(rect(), move |v| {
            if let Some(v) = v {
                self.size.set(v.size());
            }
        });
        scope.spawn_stream(self.scroll_pos.stream(), |scope, v| {
            scope.set(transform(), Mat4::from_translation(-v.extend(0.0)));
        });

        Stack::new(self.items).mount(scope)
    }
}
