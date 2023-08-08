use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    filter::{All, With},
    CommandBuffer, Component, EntityIds, FetchExt, Mutable, Query,
};
use glam::{Mat4, Quat, Vec2};
use image::{DynamicImage, ImageBuffer};
use wgpu::{BindGroup, BindGroupLayout, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    assets::{map::HandleMap, Handle},
    components::{filled_rect, rect, screen_position, Rect},
    shapes::FilledRect,
    Frame,
};

use super::{
    components::{draw_cmd, model_matrix},
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Mesh,
        Shader, Vertex, VertexDesc,
    },
    shape_renderer::DrawCommand,
    Gpu,
};

pub struct RectRenderer {
    white_image: Handle<DynamicImage>,

    layout: BindGroupLayout,
    sampler: wgpu::Sampler,

    rect_query: Query<(
        EntityIds,
        <Component<FilledRect> as TransformFetch<Modified>>::Output,
    )>,

    object_query: Query<(Component<Rect>, Component<Vec2>, Mutable<Mat4>), (All, With)>,

    bind_groups: HandleMap<DynamicImage, Handle<BindGroup>>,

    mesh: Handle<Mesh>,

    shader: Handle<Shader>,
}

impl RectRenderer {
    pub fn new(
        gpu: &Gpu,
        frame: &mut Frame,
        color_format: TextureFormat,
        global_layout: &BindGroupLayout,
        object_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let layout = BindGroupLayoutBuilder::new("RectRenderer::layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(gpu);

        let white_image = frame
            .assets
            .insert(DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
                256,
                256,
                image::Rgba([255, 255, 255, 255]),
            )));

        let sampler = gpu.device.create_sampler(&SamplerDescriptor {
            label: Some("ShapeRenderer::sampler"),
            anisotropy_clamp: 16,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let mesh = frame.assets.insert(Mesh::quad(gpu));

        let shader = frame.assets.insert(Shader::new(
            gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/solid.wgsl").into(),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[global_layout, &object_bind_group_layout, &layout],
            },
        ));

        Self {
            white_image,
            layout,
            sampler,
            rect_query: Query::new((entity_ids(), filled_rect().modified())),
            object_query: Query::new((rect(), screen_position(), model_matrix().as_mut()))
                .with(filled_rect()),
            bind_groups: HandleMap::new(),
            mesh,
            shader,
        }
    }

    pub fn build_commands(&mut self, gpu: &Gpu, frame: &mut Frame) {
        let mut cmd = CommandBuffer::new();
        self.rect_query
            .borrow(&frame.world)
            .iter()
            .for_each(|(id, rect)| {
                let image = rect.fill_image.as_ref().unwrap_or(&self.white_image);

                let bind_group = self.bind_groups.entry(&image.clone()).or_insert_with(|| {
                    let texture = Texture::from_image(gpu, image);

                    let bind_group = BindGroupBuilder::new("ShapeRenderer::textured_bind_group")
                        .bind_sampler(&self.sampler)
                        .bind_texture(&texture.view(&Default::default()))
                        .build(gpu, &self.layout);

                    frame.assets.insert(bind_group)
                });

                cmd.set(
                    id,
                    draw_cmd(),
                    DrawCommand {
                        mesh: self.mesh.clone(),
                        bind_group: bind_group.clone(),
                        shader: self.shader.clone(),
                        index_count: 6,
                    },
                );
            });

        cmd.apply(&mut frame.world).unwrap();
    }

    pub fn update(&mut self, _: &Gpu, frame: &mut Frame) {
        self.object_query
            .borrow(&frame.world)
            .iter()
            .for_each(|(&rect, &pos, model)| {
                let pos = pos + rect.pos();
                let size = rect.size();
                *model = Mat4::from_scale_rotation_translation(
                    size.extend(1.0),
                    Quat::IDENTITY,
                    pos.extend(0.1),
                );
            })
    }
}
