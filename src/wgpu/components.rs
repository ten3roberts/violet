use std::{borrow::Borrow, sync::Arc};

use flax::component;

use crate::{
    assets::Asset,
    text::FontFamily,
    wgpu::{
        graphics::texture::Texture,
        shape_renderer::{DrawCommand, ObjectData},
        text::TextBufferState,
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
