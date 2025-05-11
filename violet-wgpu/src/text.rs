use std::sync::Arc;

use cosmic_text::{
    fontdb::Source, Attrs, Buffer, FontSystem, LayoutGlyph, Metrics, Shaping, SwashCache,
};
use flax::EntityRef;
use glam::{vec2, BVec2, Vec2};
use itertools::Itertools;
use palette::Srgba;
use parking_lot::Mutex;
use violet_core::{
    components::{font_size, layout_glyphs},
    layout::{LayoutArgs, QueryArgs, SizeResolver, SizingHints},
    style::ResolvableStyle,
    text::{LayoutGlyphs, LayoutLineGlyphs, TextSegment},
    Rect,
};

use super::components::text_buffer_state;

pub(crate) static INTER_FONT: &[u8] =
    include_bytes!("../../assets/fonts/Inter/Inter-VariableFont_opsz,wght.ttf");

pub(crate) static INTER_FONT_BOLD: &[u8] =
    include_bytes!("../../assets/fonts/Inter/static/Inter-Bold.ttf");
pub(crate) static INTER_FONT_ITALIC: &[u8] =
    include_bytes!("../../assets/fonts/Inter/Inter-Italic-VariableFont_opsz,wght.ttf");

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

    pub fn new_with_fonts(sources: impl IntoIterator<Item = Source>) -> Self {
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
    fn query(&mut self, entity: &flax::EntityRef, args: QueryArgs) -> (Vec2, Vec2, SizingHints) {
        puffin::profile_scope!("TextSizeResolver::query");
        let _span = tracing::debug_span!("TextSizeResolver::query", ?args.direction).entered();

        let query = (text_buffer_state().as_mut(), font_size());

        let mut query = entity.query(&query);
        let (state, &font_size) = query.get().unwrap();

        let text_system = &mut *self.text_system.lock();

        let line_height = state.buffer.metrics().line_height;

        // Text wraps to the size of the container
        //
        // Wrapping text will decrease width, and increase height.
        //
        //
        // To optimize for X, we wrap as much as possible, and then measure the height.
        //
        // To optimize for Y, we wrap as little as possible. This is equivalent to the preferred
        // size as the widest width (which text wants) also gives the least height.

        // If preferred is can_grow, so is min
        let (most_wrapped, _can_grow, wrapped_lines) = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            vec2(1.0, args.limits.max_size.y.max(line_height)),
        );

        let (preferred, can_grow, preferred_lines) = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            args.limits.max_size.max(vec2(1.0, line_height)),
        );
        // + vec2(5.0, 5.0);

        // if min.dot(args.direction.to_axis()) > preferred.dot(args.direction.to_axis()) {
        //     tracing::error!(%entity, text=?state.text(), %min, %preferred, ?args.direction, %args.limits.max_size, "Text wrapping failed");
        // }

        (
            if args.direction.is_horizontal() {
                most_wrapped
            } else {
                preferred
            },
            preferred,
            SizingHints {
                can_grow,
                relative_size: BVec2::TRUE,
                coupled_size: wrapped_lines != preferred_lines,
            },
        )
    }

    fn apply(&mut self, entity: &flax::EntityRef, args: LayoutArgs) -> (Vec2, BVec2) {
        puffin::profile_scope!("TextSizeResolver::apply");
        let _span = tracing::debug_span!("TextSizeResolver::apply", ?args).entered();

        let query = (text_buffer_state().as_mut(), font_size());

        let mut query = entity.query(&query);
        let (state, &font_size) = query.get().unwrap();

        let text_system = &mut *self.text_system.lock();
        let line_height = state.buffer.metrics().line_height;

        let (size, can_grow, _) = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            // Add a little leeway, because an exact fit from the query may miss the last
            // word/glyph
            args.limits.max_size.max(vec2(0.0, line_height)) + vec2(5.0, 5.0),
        );

        if size.x > args.limits.max_size.x || size.y > args.limits.max_size.y {
            // tracing::error!(%entity, text=?state.text(), %size, %limits.max_size, "Text overflowed");
        }

        let glyphs = state.layout_glyphs();

        // tracing::trace!(lines=?glyphs.rows.iter().map(|v| v.len()).collect::<Vec<_>>(), "updating layout glyphs");

        *entity.get_mut(layout_glyphs()).unwrap() = glyphs;

        (size, can_grow)
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
    ) -> (Vec2, BVec2, usize) {
        // let _span = tracing::debug_span!("resolve_text_size", font_size, ?text, ?limits).entered();

        let mut buffer = state.buffer.borrow_with(&mut text_system.font_system);

        let metrics = Metrics::new(font_size, font_size);
        buffer.set_metrics(metrics);
        buffer.set_size(Some(size.x), Some(size.y));

        buffer.shape_until_scroll(true);

        measure(&state.buffer)
    }
}

fn glyph_bounds(glyph: &LayoutGlyph) -> (f32, f32) {
    (glyph.x, glyph.x + glyph.w)
}

fn measure(buffer: &Buffer) -> (Vec2, BVec2, usize) {
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

    (
        vec2(width, total_lines as f32 * buffer.metrics().line_height),
        BVec2::new(total_lines > buffer.lines.len(), false),
        total_lines,
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

    pub(crate) fn update_text(
        &mut self,
        stylesheet: &EntityRef,
        font_system: &mut FontSystem,
        text: &[TextSegment],
    ) {
        puffin::profile_function!();
        self.buffer.set_rich_text(
            font_system,
            text.iter().map(|v| {
                let color: Srgba<u8> = v.color.resolve(*stylesheet).into_format();

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
    }

    pub(crate) fn to_layout_lines(&self) -> impl Iterator<Item = Vec<LayoutLineGlyphs>> + '_ {
        puffin::profile_function!();
        let lh = self.buffer.metrics().line_height;

        self.buffer
            .lines
            .iter()
            .enumerate()
            .map(move |(row, line)| {
                let mut current_offset = 0;

                let mut glyph_index = 0;
                let Some(layout) = line.layout_opt().as_ref() else {
                    return Vec::new();
                };

                layout
                    .iter()
                    .map(|run| {
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
                                        min: vec2(glyph.x, 0.0),
                                        max: vec2(glyph.x + glyph.w, lh),
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
                    })
                    .collect_vec()
            })
    }

    pub(crate) fn layout_glyphs(&mut self) -> LayoutGlyphs {
        let lines = self.to_layout_lines().collect_vec();
        LayoutGlyphs::new(lines, self.buffer.metrics().line_height)
    }
}
