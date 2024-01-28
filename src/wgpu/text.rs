use std::sync::Arc;

use cosmic_text::{Buffer, LayoutGlyph, Metrics};
use glam::{vec2, Vec2};
use parking_lot::Mutex;

use crate::{
    components::{font_size, Rect},
    layout::{Direction, LayoutLimits, SizeResolver},
};

use super::{
    components::{text_buffer_state, TextBufferState},
    text_renderer::TextSystem,
};

pub struct TextSizeResolver {
    text_system: Arc<Mutex<TextSystem>>,
}

impl SizeResolver for TextSizeResolver {
    fn query(
        &mut self,
        entity: &flax::EntityRef,
        content_area: Rect,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> (glam::Vec2, glam::Vec2) {
        let _span =
            tracing::info_span!("TextSizeResolver::query", ?squeeze, ?content_area).entered();

        let query = (text_buffer_state().as_mut(), font_size());

        let mut query = entity.query(&query);
        let (state, &font_size) = query.get().unwrap();

        let text_system = &mut *self.text_system.lock();

        let min = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            match squeeze {
                Direction::Horizontal => vec2(1.0, limits.max_size.y),
                Direction::Vertical => vec2(limits.max_size.x, f32::MAX),
            },
        );

        let preferred = Self::resolve_text_size(state, text_system, font_size, limits.max_size)
            + vec2(5.0, 5.0);

        (min, preferred)
    }

    fn apply(
        &mut self,
        entity: &flax::EntityRef,
        content_area: Rect,
        limits: LayoutLimits,
    ) -> Vec2 {
        let _span = tracing::info_span!("TextSizeResolver::apply", ?content_area).entered();

        let query = (text_buffer_state().as_mut(), font_size());

        let mut query = entity.query(&query);
        let (state, &font_size) = query.get().unwrap();

        let text_system = &mut *self.text_system.lock();

        Self::resolve_text_size(state, text_system, font_size, limits.max_size)
    }
}

impl TextSizeResolver {
    pub fn new(text_system: Arc<Mutex<TextSystem>>) -> Self {
        Self { text_system }
    }

    fn resolve_text_size(
        state: &mut TextBufferState,
        text_system: &mut TextSystem,
        font_size: f32,
        size: Vec2,
    ) -> Vec2 {
        // let _span = tracing::debug_span!("resolve_text_size", font_size, ?text, ?limits).entered();

        let mut buffer = state.buffer.borrow_with(&mut text_system.font_system);

        let metrics = Metrics::new(font_size, font_size);
        buffer.set_metrics(metrics);
        buffer.set_size(size.x, size.y);

        buffer.shape_until_scroll();

        measure(&state.buffer)
    }
}

fn glyph_bounds(glyph: &LayoutGlyph) -> (f32, f32) {
    (glyph.x, glyph.x + glyph.w)
}

fn measure(buffer: &Buffer) -> Vec2 {
    let (width, total_lines) =
        buffer
            .layout_runs()
            .fold((0.0f32, 0), |(width, total_lines), run| {
                if let (Some(first), Some(last)) = (run.glyphs.first(), run.glyphs.last()) {
                    let (l1, r1) = glyph_bounds(first);
                    let (l2, r2) = glyph_bounds(last);

                    let l = l1.min(l2);
                    let r = r1.max(r2);

                    assert!(l <= r);
                    // tracing::debug!(l1, r1, l2, r2, "run");

                    (width.max(r - l), total_lines + 1)
                } else {
                    (width, total_lines + 1)
                }
            });

    vec2(width, total_lines as f32 * buffer.metrics().line_height)
}
