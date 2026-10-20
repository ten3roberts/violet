use std::sync::Arc;

use cosmic_text::{
    fontdb::{Database, Source},
    Attrs, Buffer, FontSystem, LayoutGlyph, Metrics, Shaping, SwashCache,
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

/// Warns once per family when text names a font family that was never loaded.
///
/// The text system silently substitutes another font for a missing family, which for icon fonts
/// means drawing a glyph from whichever other icon font shares the codepoint.
#[cfg(debug_assertions)]
fn warn_missing_families(font_system: &FontSystem, text: &[TextSegment]) {
    use std::collections::BTreeSet;
    use violet_core::text::FontFamily;

    static CHECKED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

    for segment in text {
        let FontFamily::Named(name) = &segment.family else {
            continue;
        };

        let mut checked = CHECKED.lock();
        if checked.contains(&**name) {
            continue;
        }
        checked.insert(name.to_string());

        let loaded = font_system
            .db()
            .faces()
            .any(|face| face.families.iter().any(|(family, _)| family == name));

        if !loaded {
            tracing::warn!(
                "Font family {name:?} is not loaded; text using it will fall back to another font"
            );
        }
    }
}

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
        let mut font_db = Database::new();
        for source in sources {
            font_db.load_font_source(source);
        }

        // Without a loaded "Arial" (fontdb's default), generic families resolve by falling back
        // through fonts in load order. Icon fonts such as Phosphor cover Latin letters for
        // ligatures, and would then render plain text.
        font_db.set_sans_serif_family("Inter");

        let font_system = FontSystem::new_with_locale_and_db(String::from("en-US"), font_db);

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
    fn query_size(
        &mut self,
        entity: &flax::EntityRef,
        args: QueryArgs,
    ) -> (Vec2, Vec2, SizingHints) {
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
            vec2(1.0, args.limits.layout_max_size.y.max(line_height)),
        );

        let (desired, can_grow, preferred_lines) = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            args.limits.layout_max_size.max(vec2(10.0, line_height)),
        );
        // + vec2(5.0, 5.0);

        // if min.dot(args.direction.to_axis()) > preferred.dot(args.direction.to_axis()) {
        //     tracing::error!(%entity, text=?state.text(), %min, %preferred, ?args.direction, %args.limits.max_size, "Text wrapping failed");
        // }

        (
            if args.direction.is_horizontal() {
                most_wrapped
            } else {
                desired
            },
            desired,
            SizingHints {
                can_grow,
                relative_size: BVec2::FALSE,
                coupled_size: wrapped_lines != preferred_lines,
            },
        )
    }

    fn apply_layout(&mut self, entity: &flax::EntityRef, args: LayoutArgs) -> (Vec2, BVec2) {
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
            args.limits.layout_max_size.max(vec2(0.0, line_height)) + vec2(5.0, 5.0),
        );

        if size.x > args.limits.layout_max_size.x || size.y > args.limits.layout_max_size.y {
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
        layout_size_limit: Vec2,
    ) -> (Vec2, BVec2, usize) {
        // let _span = tracing::debug_span!("resolve_text_size", font_size, ?text, ?limits).entered();

        let mut buffer = state.buffer.borrow_with(&mut text_system.font_system);

        let metrics = Metrics::new(font_size, font_size);
        buffer.set_metrics(metrics);
        buffer.set_size(Some(layout_size_limit.x), Some(layout_size_limit.y));

        buffer.shape_until_scroll(true);

        let (size, overflow, total_lines) = measure(&state.buffer);

        // Warn if overflow would have occurred with the original height
        let max_lines = (layout_size_limit.y / state.buffer.metrics().line_height).floor() as usize;
        if total_lines > max_lines {
            tracing::warn!(
                total_lines,
                max_lines,
                "Text buffer overflow: some lines would be skipped. Allowing overflow."
            );
        }

        (size, overflow, total_lines)
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
        base_color: Srgba,
    ) {
        puffin::profile_function!();

        #[cfg(debug_assertions)]
        warn_missing_families(font_system, text);

        self.buffer.set_rich_text(
            font_system,
            text.iter().map(|v| {
                let color = v
                    .color
                    .map(|v| v.resolve(*stylesheet))
                    .unwrap_or(base_color)
                    .into_format();
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
            &Attrs::new(),
            Shaping::Advanced,
            None,
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
                let Some(layout) = line.layout_opt() else {
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

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, sync::Arc};

    use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Style, Weight};
    use violet_core::icon::{IconPack, IconSet};

    use super::*;
    use crate::app::AppBuilder;

    /// Icon fonts may cover Latin letters (Phosphor does, for ligatures). Plain text of any weight
    /// or style must still be drawn with the default font.
    ///
    /// The text system only uses the default family when a face matches the weight exactly, and
    /// otherwise falls back through every loaded font, so this covers weights Inter has no face for.
    #[test]
    fn plain_text_ignores_icon_fonts() {
        let pack = IconPack {
            font: Source::Binary(Arc::new(
                include_bytes!("../../violet-phosphor/bin/Phosphor.ttf").to_vec(),
            )),
            icons: IconSet::default(),
        };

        let mut text_system =
            TextSystem::new_with_fonts(AppBuilder::new().with_icon_pack(pack).fonts);
        let font_system = &mut text_system.font_system;

        for weight in [100, 300, 400, 500, 600, 700, 800, 900].map(Weight) {
            for style in [Style::Normal, Style::Italic] {
                let mut buffer = Buffer::new(font_system, Metrics::new(14.0, 14.0));
                buffer.set_text(
                    font_system,
                    "Editor folder plus text",
                    &Attrs::new()
                        .family(Family::SansSerif)
                        .weight(weight)
                        .style(style),
                    Shaping::Advanced,
                );
                buffer.shape_until_scroll(font_system, false);

                let families = buffer
                    .layout_runs()
                    .flat_map(|run| run.glyphs.iter())
                    .map(|glyph| {
                        let face = font_system.db().face(glyph.font_id).unwrap();
                        face.families[0].0.clone()
                    })
                    .collect::<BTreeSet<_>>();

                assert_eq!(
                    families,
                    BTreeSet::from(["Inter".to_string()]),
                    "{weight:?} {style:?}"
                );
            }
        }
    }
}
