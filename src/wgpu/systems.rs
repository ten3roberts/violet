use std::sync::Arc;

use cosmic_text::{Metrics, Wrap};
use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    BoxedSystem, CommandBuffer, Component, EntityIds, Fetch, FetchExt, Mutable, OptOr, Query,
    QueryBorrow, System,
};
use parking_lot::Mutex;

use crate::{
    components::{font_size, rect, size_resolver, text, text_wrap, Rect},
    text::TextSegment,
};

use super::{
    components::text_buffer_state,
    text::{TextBufferState, TextSizeResolver},
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

                    buffer.shape_until_scroll(&mut text_system.font_system, true);
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

                    let resolver = TextSizeResolver::new(text_system.clone());

                    cmd.set(id, text_buffer_state(), state)
                       .set(
                            id,
                            size_resolver(),
                            Box::new(resolver),
                       );

                }
            },
        )
        .boxed()
}
