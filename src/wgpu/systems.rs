use std::sync::Arc;

use crate::layout::Direction;
use cosmic_text::{Buffer, LayoutGlyph, Metrics, Wrap};
use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    BoxedSystem, CommandBuffer, Component, EntityIds, Fetch, FetchExt, Mutable, OptOr, Query,
    QueryBorrow, System,
};
use glam::{vec2, Vec2};
use parking_lot::Mutex;

use crate::{
    components::{font_size, rect, size_resolver, text, text_wrap, Rect},
    layout::{LayoutLimits, SizeResolver},
    text::TextSegment,
};

use super::{
    components::{text_buffer_state, TextBufferState},
    text_renderer::TextSystem,
};

#[derive(Fetch)]
#[fetch(transforms = [Modified])]
struct TextBufferQuery {
    #[fetch(ignore)]
    state: Mutable<TextBufferState>,
    text: Component<Vec<TextSegment>>,
    rect: Component<Rect>,
    font_size: Component<f32>,
    wrap: OptOr<Component<Wrap>, Wrap>,
}

impl TextBufferQuery {
    fn new() -> Self {
        Self {
            state: text_buffer_state().as_mut(),
            text: text(),
            rect: rect(),
            font_size: font_size(),
            wrap: text_wrap().opt_or(Wrap::Word),
        }
    }
}

pub(crate) fn update_text_buffers(text_system: Arc<Mutex<TextSystem>>) -> BoxedSystem {
    System::builder()
        .with_query(Query::new(TextBufferQuery::new().modified()))
        .build(
            move |mut query: QueryBorrow<
                <TextBufferQuery as TransformFetch<Modified>>::Output,
                _,
            >| {
                let text_system = &mut *text_system.lock();
                query.iter().for_each(|item| {
                    let buffer = &mut item.state.buffer;
                    let metrics = Metrics::new(*item.font_size, *item.font_size);

                    buffer.set_metrics(&mut text_system.font_system, metrics);
                    buffer.set_wrap(&mut text_system.font_system, *item.wrap);

                    let metrics = Metrics::new(*item.font_size, *item.font_size);
                    buffer.set_metrics(&mut text_system.font_system, metrics);
                    item.state
                        .update_text(&mut text_system.font_system, item.text);

                    let buffer = &mut item.state.buffer;
                    let size = item.rect.size();

                    buffer.set_size(&mut text_system.font_system, size.x, size.y);

                    buffer.shape_until_scroll(&mut text_system.font_system);
                });
            },
        )
        .boxed()
}

pub(crate) fn register_text_buffers(text_system: Arc<Mutex<TextSystem>>) -> BoxedSystem {
    System::builder()
        .with_cmd_mut()
        .with_query(Query::new((entity_ids(), text())).without(size_resolver()))
        .build(
            move |cmd: &mut CommandBuffer,
                  mut query: QueryBorrow<'_, (EntityIds, Component<Vec<TextSegment>>), _>| {
                let mut text_system_ref = text_system.lock();
                for (id, _) in &mut query {
                    let state = TextBufferState::new(&mut text_system_ref.font_system);

                    let resolver = TextSizeResolver {
                        text_system: text_system.clone(),
                    };

                    cmd.set(id, text_buffer_state(), state).set(
                        id,
                        size_resolver(),
                        Box::new(resolver),
                    );
                }
            },
        )
        .boxed()
}

pub struct TextSizeResolver {
    text_system: Arc<Mutex<TextSystem>>,
}

impl SizeResolver for TextSizeResolver {
    fn query(
        &mut self,
        entity: &flax::EntityRef,
        content_area: Rect,
        _limits: LayoutLimits,
        squeeze: Direction,
    ) -> (glam::Vec2, glam::Vec2) {
        let _span =
            tracing::info_span!("TextSizeResolver::query", ?squeeze, ?content_area).entered();

        let query = (text_buffer_state().as_mut(), font_size());

        let mut query = entity.query(&query);
        let (state, &font_size) = query.get().unwrap();

        let text_system = &mut *self.text_system.lock();
        let preferred = Self::resolve_text_size(state, text_system, font_size, Vec2::MAX);

        let min = Self::resolve_text_size(
            state,
            text_system,
            font_size,
            match squeeze {
                Direction::Horizontal => vec2(1.0, content_area.size().y),
                Direction::Vertical => vec2(content_area.size().x, f32::MAX),
            },
        );

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
        let preferred = Self::resolve_text_size(state, text_system, font_size, Vec2::MAX);

        preferred
    }
}

impl TextSizeResolver {
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
