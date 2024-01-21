use std::{borrow::Borrow, ffi::FromVecWithNulError, sync::Arc};

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, Wrap};
use flax::component;
use palette::Srgba;

use crate::{
    assets::Asset,
    text::{FontFamily, TextSegment},
    wgpu::{
        graphics::texture::Texture,
        shape_renderer::{DrawCommand, ObjectData},
    },
};

use super::mesh_buffer::MeshHandle;

component! {
    /// The gpu texture to use for rendering
    pub(crate) texture: Asset<Texture>,

    /// Renderer specific data for drawing a shape
    pub(crate) draw_cmd: DrawCommand,
    pub(crate) object_data: ObjectData,

    /// The mesh for a rendered shape
    pub(crate) text_mesh: Arc<MeshHandle>,

    // pub model_matrix: glam::Mat4,

    pub text_buffer_state: TextBufferState,
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

    pub(crate) fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub(crate) fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
}

impl<'a> From<&'a FontFamily> for cosmic_text::Family<'a> {
    fn from(value: &'a FontFamily) -> Self {
        match value {
            FontFamily::Named(name) => cosmic_text::Family::Name(name.borrow()),
            FontFamily::Serif => cosmic_text::Family::Serif,
            FontFamily::SansSerif => cosmic_text::Family::SansSerif,
            FontFamily::Cursive => cosmic_text::Family::Cursive,
            FontFamily::Fantasy => cosmic_text::Family::Fantasy,
            FontFamily::Monospace => cosmic_text::Family::Monospace,
        }
    }
}
