use std::collections::HashMap;

use flax::{entity_ids, Query};
use fontdue::layout::TextStyle;
use palette::convert::FromColorUnclampedMutGuard;

use crate::{assets::Handle, components::text, Frame};

use super::{
    components::{draw_cmd, font_from_file},
    font::{Font, FontFromFile},
    shape_renderer::DrawCommand,
    Gpu,
};

pub struct TextRenderer {
    fonts: HashMap<FontFromFile, Handle<Font>>,
}

impl TextRenderer {
    pub fn update_text_meshes(&mut self, gpu: &Gpu, frame: &mut Frame) {
        let mut query = Query::new((entity_ids(), font_from_file(), text(), draw_cmd()));

        for (id, font_from_file, text, mesh) in &mut query.borrow(frame.world()) {
            tracing::info!("Updating mesh for text {id}");

            let font = self
                .fonts
                .entry(font_from_file.clone())
                .or_insert_with_key(|key| frame.assets.load(key));

            let mut layout = fontdue::layout::Layout::<()>::new(
                fontdue::layout::CoordinateSystem::PositiveYDown,
            );

            layout.append(
                &[&font.font],
                &TextStyle {
                    text: &*text,
                    px: 12.0,
                    font_index: 0,
                    user_data: (),
                },
            );
        }
    }
}
