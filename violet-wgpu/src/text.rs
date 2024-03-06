use std::sync::Arc;

use cosmic_text::{
    fontdb::Source, Attrs, Buffer, FontSystem, LayoutGlyph, Metrics, Shaping, SwashCache,
};
use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::Srgba;
use parking_lot::Mutex;

use violet_core::{
    components::font_size,
    layout::{Direction, LayoutLimits, SizeResolver, SizingHints},
    text::{LayoutGlyphs, LayoutLineGlyphs, TextSegment},
    Rect,
};

use super::components::text_buffer_state;

static INTER_FONT: &[u8] =
    include_bytes!("../../assets/fonts/Inter/Inter-VariableFont_slnt,wght.ttf");
pub struct TextSystem {
    pub(crate) font_system: FontSystem,
    pub(crate) swash_cache: SwashCache,
}

impl TextSystem {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    pub fn new_with_defaults() -> Self {
        let sources = [Source::Binary(Arc::new(INTER_FONT.to_vec()))];
        let font_system = FontSystem::new_with_fonts(sources);

        Self {
            font_system,
            swash_cache: SwashCache::new(),
        }
    }
}

impl Default for TextSystem {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TextSizeResolver {
    text_system: Arc<Mutex<TextSystem>>,
}

impl SizeResolver for TextSizeResolver {
    fn query(
        &mut self,
        entity: &flax::EntityRef,
        _content_area: Vec2,
        limits: LayoutLimits,
        direction: Direction,
    ) -> (Vec2, Vec2, SizingHints) {
        puffin::profile_scope!("TextSizeResolver::query");
        let _span = tracing::debug_span!("TextSizeResolver::query", ?direction).entered();

        let query = (text_buffer_state().as_mut(), font_size());

        let mut query = entity.query(&query);
        let (state, &font_size) = query.get().unwrap();

        let text_system = &mut *self.text_system.lock();

        let line_height = state.buffer.metrics().line_height;

        // If preferred is clamped, so is min
        let (min, _clamped) = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            match direction {
                Direction::Horizontal => vec2(1.0, limits.max_size.y.max(line_height)),
                Direction::Vertical => vec2(limits.max_size.x, limits.max_size.y.max(line_height)),
            },
        );

        let (preferred, clamped) = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            limits.max_size.max(vec2(1.0, line_height)),
        );
        // + vec2(5.0, 5.0);

        if min.dot(direction.to_axis()) > preferred.dot(direction.to_axis()) {
            tracing::error!(%entity, text=?state.text(), %min, %preferred, ?direction, %limits.max_size, "Text wrapping failed");
        }
        (
            min,
            preferred,
            SizingHints {
                can_grow: clamped,
                fixed_size: true,
            },
        )
    }

    fn apply(
        &mut self,
        entity: &flax::EntityRef,
        content_area: Vec2,
        limits: LayoutLimits,
    ) -> (Vec2, bool) {
        puffin::profile_scope!("TextSizeResolver::apply");
        let _span = tracing::debug_span!("TextSizeResolver::apply", ?content_area).entered();

        let query = (text_buffer_state().as_mut(), font_size());

        let mut query = entity.query(&query);
        let (state, &font_size) = query.get().unwrap();

        let text_system = &mut *self.text_system.lock();
        let line_height = state.buffer.metrics().line_height;

        let (size, clamped) = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            // Add a little leeway, because an exact fit from the query may miss the last
            // word/glyph
            limits.max_size.max(vec2(0.0, line_height)) + vec2(5.0, 5.0),
        );

        if size.x > limits.max_size.x || size.y > limits.max_size.y {
            // tracing::error!(%entity, text=?state.text(), %size, %limits.max_size, "Text overflowed");
        }

        (size, clamped)
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
    ) -> (Vec2, bool) {
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

fn measure(buffer: &Buffer) -> (Vec2, bool) {
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

    // tracing::info!(?total_lines, lines = buffer.lines.len(), "measure");

    (
        vec2(width, total_lines as f32 * buffer.metrics().line_height),
        total_lines > buffer.lines.len(),
    )
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
        puffin::profile_function!();
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

    fn text(&self) -> Vec<String> {
        self.buffer
            .lines
            .iter()
            .map(|v| v.text().to_owned())
            .collect::<Vec<_>>()
    }

    pub(crate) fn to_layout_lines(&self) -> impl Iterator<Item = LayoutLineGlyphs> + '_ {
        puffin::profile_function!();
        let lh = self.buffer.metrics().line_height;

        let mut result = Vec::new();

        for (row, line) in self.buffer.lines.iter().enumerate() {
            let mut current_offset = 0;

            let mut glyph_index = 0;
            let Some(layout) = line.layout_opt().as_ref() else {
                continue;
            };

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

                        violet_core::text::LayoutGlyph {
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

    pub(crate) fn layout_glyphs(&mut self) -> LayoutGlyphs {
        let lines = self.to_layout_lines().collect_vec();
        LayoutGlyphs::new(lines, self.buffer.metrics().line_height)
    }
}
