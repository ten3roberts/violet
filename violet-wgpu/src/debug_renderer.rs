use std::sync::Arc;

use image::DynamicImage;
use violet_core::{assets::Asset, stored};
use wgpu::BindGroupLayout;

use crate::{graphics::Shader, mesh_buffer::MeshHandle};

pub struct DebugRenderer {
    white_image: Asset<DynamicImage>,
    layout: BindGroupLayout,
    sampler: wgpu::Sampler,

    mesh: Arc<MeshHandle>,

    shader: stored::Handle<Shader>,
}

impl DebugRenderer {}
