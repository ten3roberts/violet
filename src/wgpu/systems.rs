use std::sync::Arc;

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use flax::{entity_ids, BoxedSystem, CommandBuffer, Query, QueryBorrow, System};
use glam::{vec2, Vec2};
use parking_lot::Mutex;

use crate::{
    components::{font_size, resolve_size, text, Rect},
    layout::{LayoutLimits, SizeResolver},
};

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

pub fn set_text_size_resolver(font_system: Arc<Mutex<FontSystem>>) -> BoxedSystem {
    System::builder()
        .with_cmd_mut()
        .with_query(Query::new((entity_ids(), text())).without(resolve_size()))
        .build(
            move |cmd: &mut CommandBuffer,
                  mut query: QueryBorrow<
                '_,
                (flax::EntityIds, flax::Component<String>),
                (flax::filter::All, flax::filter::Without),
            >| {
                let mut font_system_ref = font_system.lock();
                for (id, _) in &mut query {
                    let resolver = TextSizeResolver {
                        font_system: font_system.clone(),
                        buffer: Buffer::new(&mut font_system_ref, Metrics::new(14.0, 14.0)),
                    };

                    cmd.set(id, crate::components::resolve_size(), Box::new(resolver));
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
        content_area: crate::components::Rect,
        limits: Option<crate::layout::LayoutLimits>,
    ) -> (glam::Vec2, glam::Vec2) {
        let query = (font_size(), text());
        if let Some((&font_size, text)) = entity.query(&query).get() {
            let font_system = &mut *self.font_system.lock();
            let preferred = Self::resolve_text_size(
                font_system,
                &mut self.buffer,
                text,
                font_size,
                content_area,
                limits,
            );

            let min = Vec2::ZERO;

            return (min, preferred);
        }

        todo!()
    }
}

impl TextSizeResolver {
    fn resolve_text_size(
        font_system: &mut FontSystem,
        buffer: &mut Buffer,
        text: &str,
        font_size: f32,
        _content_area: Rect,
        limits: Option<LayoutLimits>,
    ) -> Vec2 {
        let _span = tracing::debug_span!("resolve_text_size", font_size, ?text, ?limits).entered();

        {
            let mut buffer = buffer.borrow_with(font_system);

            let metrics = Metrics::new(font_size, font_size);
            buffer.set_metrics(metrics);

            if let Some(limits) = limits {
                buffer.set_size(limits.max_size.x, limits.max_size.y);
            } else {
                buffer.set_size(1000.0, 100.0);
            }

            buffer.set_text(text, Attrs::new(), Shaping::Advanced);
            buffer.shape_until_scroll();
        }

        let size = measure(buffer);
        // dbg!(buffer.size());

        // let size = Vec2::ZERO;

        // for i in 0..dbg!(buffer.visible_lines()) as usize {
        //     let line = buffer.line_layout(i);
        //     dbg!(line);
        // }

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
