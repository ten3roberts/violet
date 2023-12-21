use std::sync::Arc;

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    BoxedSystem, CommandBuffer, Component, EntityIds, Fetch, FetchExt, Mutable, Query, QueryBorrow,
    System,
};
use glam::{vec2, Vec2};
use parking_lot::Mutex;

use crate::{
    components::{font_size, rect, size_resolver, text, Rect},
    layout::{LayoutLimits, SizeResolver},
};

use super::components::{text_buffer_state, TextBufferState};

// pub fn load_fonts_system(font_map: FontMap) -> BoxedSystem {
//     System::builder()
//         .with_cmd_mut()
//         .with_query(Query::new((entity_ids(), font_family().modified())))
//         .build(
//             move |cmd: &mut CommandBuffer, mut query: QueryBorrow<_, _>| {
//                 for (id, font) in &mut query {
//                     let font = match font_map.get(font) {
//                         Ok(v) => v,
//                         Err(err) => {
//                             tracing::error!("Error loading font: {:?}", err);
//                             continue;
//                         }
//                     };

//                     cmd.set(id, components::font_handle(), font);
//                 }
//             },
//         )
//         .boxed()
// }

#[derive(Fetch)]
#[fetch(transforms = [Modified])]
struct TextBufferQuery {
    #[fetch(ignore)]
    state: Mutable<TextBufferState>,
    text: Component<String>,
    rect: Component<Rect>,
    font_size: Component<f32>,
}

impl TextBufferQuery {
    fn new() -> Self {
        Self {
            state: text_buffer_state().as_mut(),
            text: text(),
            rect: rect(),
            font_size: font_size(),
        }
    }
}

pub fn update_text_buffers(font_system: Arc<Mutex<FontSystem>>) -> BoxedSystem {
    System::builder()
        .with_query(Query::new(TextBufferQuery::new().modified()))
        .build(
            move |mut query: QueryBorrow<
                <TextBufferQuery as TransformFetch<Modified>>::Output,
                _,
            >| {
                let font_system = &mut *font_system.lock();
                query.iter().for_each(|item| {
                    item.state.update(font_system, item.text);
                    let buffer = &mut item.state.buffer;
                    let metrics = Metrics::new(*item.font_size, *item.font_size);
                    buffer.set_metrics(font_system, metrics);

                    let size = item.rect.size();

                    buffer.set_size(font_system, size.x, size.y);
                    buffer.shape_until_scroll(font_system);
                });
            },
        )
        .boxed()
}

pub fn register_text_buffers(font_system: Arc<Mutex<FontSystem>>) -> BoxedSystem {
    System::builder()
        .with_cmd_mut()
        .with_query(Query::new((entity_ids(), text())).without(size_resolver()))
        .build(
            move |cmd: &mut CommandBuffer,
                  mut query: QueryBorrow<'_, (EntityIds, Component<String>), _>| {
                let mut font_system_ref = font_system.lock();
                for (id, _) in &mut query {
                    let state = TextBufferState::new(&mut font_system_ref);

                    let resolver = TextSizeResolver {
                        font_system: font_system.clone(),
                        buffer: Buffer::new(&mut font_system_ref, Metrics::new(14.0, 14.0)),
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
    font_system: Arc<Mutex<FontSystem>>,
    buffer: Buffer,
}

impl SizeResolver for TextSizeResolver {
    fn resolve(
        &mut self,
        entity: &flax::EntityRef,
        content_area: Rect,
        limits: Option<LayoutLimits>,
    ) -> (glam::Vec2, glam::Vec2) {
        let query = (text_buffer_state().as_mut(), font_size(), text());
        if let Some((state, &font_size, text)) = entity.query(&query).get() {
            let font_system = &mut *self.font_system.lock();
            let preferred =
                Self::resolve_text_size(state, font_system, text, font_size, content_area, limits);

            let min = Self::resolve_text_size(
                state,
                font_system,
                text,
                font_size,
                content_area,
                Some(LayoutLimits {
                    min_size: Vec2::ZERO,
                    max_size: Vec2::ZERO,
                }),
            );

            return (min, preferred);
        }

        todo!()
    }
}

impl TextSizeResolver {
    fn resolve_text_size(
        state: &mut TextBufferState,
        font_system: &mut FontSystem,
        text: &str,
        font_size: f32,
        _content_area: Rect,
        limits: Option<LayoutLimits>,
    ) -> Vec2 {
        let _span = tracing::debug_span!("resolve_text_size", font_size, ?text, ?limits).entered();

        {
            let mut buffer = state.buffer.borrow_with(font_system);

            let metrics = Metrics::new(font_size, font_size);
            buffer.set_metrics(metrics);

            if let Some(limits) = limits {
                buffer.set_size(limits.max_size.x, limits.max_size.y);
            } else {
                buffer.set_size(f32::MAX, f32::MAX);
            }

            // buffer.shape_until_scroll();
        }

        let size = measure(&state.buffer);

        tracing::debug!(%size);

        size
    }
}

fn measure(buffer: &Buffer) -> Vec2 {
    let (width, total_lines) = buffer
        .layout_runs()
        .fold((0.0, 0), |(width, total_lines), run| {
            (run.line_w.max(width), total_lines + 1)
        });

    vec2(width, total_lines as f32 * buffer.metrics().line_height)
}
