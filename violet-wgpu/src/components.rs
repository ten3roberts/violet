use std::sync::Arc;

use flax::component;

use crate::{
    graphics::texture::Texture,
    renderer::{DrawCommand, ObjectData},
    text::TextBufferState,
};

use violet_core::assets::Asset;

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
