use std::{ffi::FromBytesWithNulError, sync::Arc};

use cosmic_text::{Attrs, Buffer, FontSystem, LayoutGlyph, Metrics, Shaping};
use flax::EntityRef;
use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::Srgba;
use parking_lot::Mutex;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    components::{font_size, Rect},
    layout::{Direction, LayoutLimits, SizeResolver},
    text::{LayoutGlyphs, LayoutLineGlyphs, TextSegment},
};

use super::{components::text_buffer_state, text_renderer::TextSystem};

pub struct TextSizeResolver {
    text_system: Arc<Mutex<TextSystem>>,
}

impl SizeResolver for TextSizeResolver {
    fn query(
        &mut self,
        entity: &flax::EntityRef,
        content_area: Vec2,
        limits: LayoutLimits,
        squeeze: Direction,
    ) -> (glam::Vec2, glam::Vec2) {
        let _span =
            tracing::debug_span!("TextSizeResolver::query", ?squeeze, ?content_area).entered();

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

        let preferred =
            Self::resolve_text_size(state, text_system, font_size, limits.max_size - 5.0)
                + vec2(5.0, 5.0);

        (min, preferred)
    }

    fn apply(
        &mut self,
        entity: &flax::EntityRef,
        content_area: Vec2,
        limits: LayoutLimits,
    ) -> Vec2 {
        let _span = tracing::debug_span!("TextSizeResolver::apply", ?content_area).entered();

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

        buffer.shape_until_scroll(true);

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

pub(crate) struct TextBufferState {
    pub(crate) buffer: Buffer,
}

impl TextBufferState {
    pub(crate) fn new(font_system: &mut FontSystem) -> Self {
        Self {
            buffer: Buffer::new(font_system, Metrics::new(14.0, 14.0)),
        }
    }

    pub(crate) fn update_text(&mut self, font_system: &mut FontSystem, text: &[TextSegment]) {
        self.buffer.set_rich_text(
            font_system,
            text.iter().map(|v| {
                let color: Srgba<u8> = v.color.into_format();

                (
                    &*v.text,
                    Attrs::new()
                        .family((&v.family).into())
                        .style(v.style)
                        .weight(v.weight)
                        .color(cosmic_text::Color::rgba(
                            color.red,
                            color.green,
                            color.blue,
                            color.alpha,
                        )),
                )
            }),
            Attrs::new(),
            Shaping::Advanced,
        );
        // self.buffer.set_text(
        //     font_system,
        //     text,
        //     Attrs::new()
        //         .family(cosmic_text::Family::Name("Inter"))
        //         .style(Style::Normal)
        //         .weight(400.0)
        //     Shaping::Advanced,
        // );
    }

    pub(crate) fn to_layout_lines(
        &self,
        font_system: &mut FontSystem,
    ) -> impl Iterator<Item = LayoutLineGlyphs> + '_ {
        let lh = self.buffer.metrics().line_height;

        let mut result = Vec::new();

        let mut current_offset = 0;

        for (row, line) in self.buffer.lines.iter().enumerate() {
            let mut glyph_index = 0;
            let layout = line.layout_opt().as_ref().unwrap();

            result.extend(layout.iter().enumerate().map(|(i, run)| {
                let top = i as f32 * lh;
                let bottom = top + lh;

                let start = current_offset;
                let glyphs = run
                    .glyphs
                    .iter()
                    .map(|glyph| {
                        let index = glyph_index;
                        glyph_index += 1;

                        current_offset = glyph.end;

                        crate::text::LayoutGlyph {
                            index,
                            start: glyph.start,
                            end: glyph.end,
                            bounds: Rect {
                                min: vec2(glyph.x, top),
                                max: vec2(glyph.x + glyph.w, bottom),
                            },
                        }
                    })
                    .collect_vec();

                let bounds = if let (Some(l), Some(r)) = (glyphs.first(), glyphs.last()) {
                    l.bounds.merge(r.bounds)
                } else {
                    Rect::ZERO
                };

                LayoutLineGlyphs {
                    row,
                    bounds,
                    glyphs,
                    start,
                    end: current_offset,
                }
            }));
        }

        result.into_iter()
    }

    pub(crate) fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub(crate) fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    pub(crate) fn layout_glyphs(&mut self, font_system: &mut FontSystem) -> LayoutGlyphs {
        let lines = self.to_layout_lines(font_system).collect_vec();
        LayoutGlyphs::new(lines, self.buffer.metrics().line_height)
    }
}

// pub struct TextBufferArea {}

// impl TextArea for TextBufferArea {
//     fn hit(&self, entity: &EntityRef, x: f32, y: f32) -> Option<(usize, usize)> {
//         let state = entity.get(text_buffer_state()).ok()?;
//         let cursor = state.buffer.hit(x, y)?;
//         Some((cursor.line, cursor.index))
//     }

//     fn find_glyph(&self, entity: &EntityRef, row: usize, col: usize) -> Option<Rect> {
//         let state = entity.get(text_buffer_state()).ok()?;

//         let (visual_line, glyph) = state
//             .buffer
//             .layout_runs()
//             .enumerate()
//             .filter(|(_, v)| v.line_i == row)
//             .flat_map(|(i, v)| v.glyphs.iter().map(move |v| (i, v)))
//             .nth(col)?;

//         let (l, r) = glyph_bounds(glyph);
//         let line_start = visual_line as f32 * state.buffer.metrics().line_height;

//         Some(Rect {
//             min: vec2(l, line_start),
//             max: vec2(r, line_start + state.buffer.metrics().line_height),
//         })
//     }
// }
