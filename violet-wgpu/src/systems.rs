use std::sync::Arc;

use cosmic_text::Wrap;
use flax::{
    components::child_of,
    entity_ids,
    fetch::{Modified, TransformFetch},
    BoxedSystem, CommandBuffer, Component, EntityIds, Fetch, FetchExt, ComponentMut, OptOr, Query,
    QueryBorrow, RelationExt, System, World,
};
use parking_lot::Mutex;

use puffin::profile_scope;
use violet_core::{
    components::{font_size, layout_glyphs, size_resolver, text, text_wrap},
    style::stylesheet,
    text::{LayoutGlyphs, TextSegment},
};

use crate::text::TextSystem;

use super::{
    components::text_buffer_state,
    text::{TextBufferState, TextSizeResolver},
};

#[derive(Fetch)]
#[fetch(transforms = [Modified])]
struct TextBufferQuery {
    #[fetch(ignore)]
    id: EntityIds,
    #[fetch(ignore)]
    state: ComponentMut<TextBufferState>,
    text: Component<Vec<TextSegment>>,
    font_size: Component<f32>,
    wrap: OptOr<Component<Wrap>, Wrap>,
}

impl TextBufferQuery {
    fn new() -> Self {
        Self {
            id: entity_ids(),
            state: text_buffer_state().as_mut(),
            // layout_bounds: layout_bounds(),
            text: text(),
            // rect: rect(),
            font_size: font_size(),
            wrap: text_wrap().opt_or(Wrap::Word),
            // layout_glyphs: layout_glyphs().as_mut(),
        }
    }
}

/// Updates text buffers with new text and layout information.
pub(crate) fn update_text_buffers(text_system: Arc<Mutex<TextSystem>>) -> BoxedSystem {
    let stylesheet_query = stylesheet.first_relation().traverse(child_of);
    System::builder()
        .with_world()
        .with_query(Query::new((
            stylesheet_query,
            TextBufferQuery::new().modified(),
        )))
        .build(
            move |world: &World,
                  mut query: QueryBorrow<
                (_, <TextBufferQuery as TransformFetch<Modified>>::Output),
                _,
            >| {
                puffin::profile_scope!("update_text_buffers");
                let text_system = &mut *text_system.lock();
                query.iter().for_each(|((stylesheet_id, &()), item)| {
                    let _span = tracing::debug_span!("update_text_buffers", %item.id).entered();

                    tracing::debug!(?item.text);

                    let stylesheet = world.entity(stylesheet_id).unwrap();

                    item.state
                        .update_text(&stylesheet, &mut text_system.font_system, item.text);

                    let buffer = &mut item.state.buffer;
                    buffer.set_wrap(&mut text_system.font_system, *item.wrap);

                    // let size = item.rect.size();

                    // let mut buffer = item.state.buffer.borrow_with(&mut text_system.font_system);
                    // buffer.set_metrics_and_size(
                    //     Metrics {
                    //         font_size: *item.font_size,
                    //         line_height: *item.font_size,
                    //     },
                    //     item.layout_bounds.x + 5.0,
                    //     item.layout_bounds.y + 5.0,
                    // );

                    // buffer.set_size(&mut text_system.font_system, Some(size.x), Some(size.y));

                    // *item.layout_glyphs = item.state.layout_glyphs();
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
