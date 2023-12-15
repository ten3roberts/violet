use std::sync::Arc;

use flax::component;
use fontdue::Font;

use crate::{
    assets::Handle,
    wgpu::{graphics::texture::Texture, shape_renderer::DrawCommand},
};

use super::mesh_buffer::MeshHandle;

component! {
    /// The gpu texture to use for rendering
    pub(crate) texture: Handle<Texture>,

    pub(crate) font_handle: Handle<Font>,

    /// Renderer specific data for drawing a shape
    pub(crate) draw_cmd: DrawCommand,

    pub(crate) mesh_handle: Arc<MeshHandle>,

    pub model_matrix: glam::Mat4 => [ ],
}
