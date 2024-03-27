use std::sync::Arc;

use cosmic_text::{Metrics, Wrap};
use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    BoxedSystem, CommandBuffer, Component, EntityIds, Fetch, FetchExt, Mutable, OptOr, Query,
    QueryBorrow, System,
};
use glam::Vec2;
use parking_lot::Mutex;

use puffin::profile_scope;
use violet_core::{
    components::{font_size, layout_bounds, layout_glyphs, rect, size_resolver, text, text_wrap},
    text::{LayoutGlyphs, TextSegment},
    Rect,
};

use crate::text::TextSystem;

use super::{
    components::text_buffer_state,
    text::{TextBufferState, TextSizeResolver},
};

#[derive(Fetch)]
#[fetch(transforms = [Modified])]
struct TextBufferQuery {
    state: Mutable<TextBufferState>,
    layout_glyphs: Mutable<LayoutGlyphs>,
    layout_bounds: Component<Vec2>,
    text: Component<Vec<TextSegment>>,
    rect: Component<Rect>,
    font_size: Component<f32>,
    wrap: OptOr<Component<Wrap>, Wrap>,
}

impl TextBufferQuery {
    fn new() -> Self {
        Self {
            state: text_buffer_state().as_mut(),
            layout_bounds: layout_bounds(),
            text: text(),
            rect: rect(),
            font_size: font_size(),
            wrap: text_wrap().opt_or(Wrap::Word),
            layout_glyphs: layout_glyphs().as_mut(),
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
                puffin::profile_scope!("update_text_buffers");
                let text_system = &mut *text_system.lock();
                query.iter().for_each(|item| {
                    item.state
                        .update_text(&mut text_system.font_system, item.text);

                    let buffer = &mut item.state.buffer;
                    buffer.set_wrap(&mut text_system.font_system, *item.wrap);

                    // let size = item.rect.size();

                    let mut buffer = item.state.buffer.borrow_with(&mut text_system.font_system);
                    buffer.set_metrics_and_size(
                        Metrics {
                            font_size: *item.font_size,
                            line_height: *item.font_size,
                        },
                        item.rect.size().x,
                        item.rect.size().y,
                    );

                    buffer.shape_until_scroll(true);

                    *item.layout_glyphs = item.state.layout_glyphs();
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
                profile_scope!("register_text_buffers");
                let mut text_system_ref = text_system.lock();
                for (id, _) in &mut query {
                    let state = TextBufferState::new(&mut text_system_ref.font_system);

                    let resolver = TextSizeResolver::new(text_system.clone());

                    cmd.set(id, text_buffer_state(), state)
                        .set(id, layout_glyphs(), LayoutGlyphs::default())
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
