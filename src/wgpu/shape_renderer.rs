use std::{borrow::Cow, collections::HashMap};

use flax::{child_of, Entity, Query, World};
use glam::{vec4, Mat4, Vec4};
use image::{DynamicImage, ImageBuffer};
use itertools::Itertools;
use palette::Srgba;
use wgpu::{
    BindGroup, BindGroupLayout, BufferUsages, RenderPass, Sampler, SamplerDescriptor, ShaderStages,
    TextureFormat,
};

use crate::{
    assets::Handle,
    components::{children, rect, shape, Rect},
    shapes::{FilledRect, Shape},
    Frame,
};

use super::{
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Mesh,
        Shader, TypedBuffer, Vertex, VertexDesc,
    },
    texture::TextureFromImage,
    Gpu,
};

#[derive(PartialEq)]
enum DrawShape {
    Rect,
}

struct DrawCommand {
    first_instance: u32,
    count: u32,
    shape: DrawShape,
    fill_image: Handle<DynamicImage>,
}

/// Draws shapes from the frame
pub struct ShapeRenderer {
    quad: Mesh,
    objects: Vec<ObjectData>,
    object_buffer: TypedBuffer<ObjectData>,
    object_bind_group_layout: wgpu::BindGroupLayout,
    shader: Shader,

    commands: Vec<DrawCommand>,

    sampler: Sampler,
    white_image: Handle<DynamicImage>,

    bind_groups: HashMap<Handle<DynamicImage>, BindGroup>,
}

impl ShapeRenderer {
    pub fn new(
        gpu: &Gpu,
        frame: &mut Frame,
        global_layout: &BindGroupLayout,
        color_format: TextureFormat,
    ) -> Self {
        let object_bind_group_layout =
            BindGroupLayoutBuilder::new("ShapeRenderer::object_bind_group_layout")
                .bind_storage_buffer(ShaderStages::VERTEX)
                .bind_sampler(ShaderStages::FRAGMENT)
                .bind_texture(ShaderStages::FRAGMENT)
                .build(gpu);

        let object_buffer = TypedBuffer::new_uninit(
            gpu,
            "ShapeRenderer::object_buffer",
            BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            128,
        );

        let sampler = gpu.device.create_sampler(&SamplerDescriptor {
            label: Some("ShapeRenderer::sampler"),
            anisotropy_clamp: 16,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = Shader::new(
            gpu,
            ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/solid.wgsl").into(),
                format: color_format,
                vertex_layouts: Cow::Borrowed(&[Vertex::layout()]),
                layouts: &[global_layout, &object_bind_group_layout],
            },
        );

        let white_image = frame
            .assets
            .insert(DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
                256,
                256,
                image::Rgba([255, 255, 255, 255]),
            )));

        Self {
            quad: Mesh::quad(gpu),
            objects: Vec::new(),
            object_buffer,
            object_bind_group_layout,
            shader,
            sampler,
            commands: Vec::new(),
            white_image,
            bind_groups: HashMap::new(),
        }
    }

    pub fn update(&mut self, gpu: &Gpu, frame: &mut Frame) {
        let mut query = Query::new((rect(), shape())).topo(child_of);

        self.objects.clear();
        self.commands.clear();

        self.commands.extend(
            (&mut query.borrow(frame.world()))
                .into_iter()
                .map(|(rect, shape)| {
                    let pos = rect.pos();
                    let size = rect.size();

                    match shape {
                        Shape::FilledRect(FilledRect { color, fill_image }) => {
                            let instance = self.objects.len() as u32;

                            self.objects.push(ObjectData {
                                world_matrix: Mat4::from_scale_rotation_translation(
                                    size.extend(1.0),
                                    Default::default(),
                                    pos.extend(0.1),
                                ),
                                color: srgba_to_vec4(*color),
                            });

                            let fill_image = fill_image.as_ref().unwrap_or(&self.white_image);

                            self.bind_groups
                                .entry(fill_image.clone())
                                .or_insert_with_key(|image| {
                                    let texture = Texture::from_image(gpu, image);

                                    BindGroupBuilder::new("ShapeRenderer::textured_bind_group")
                                        .bind_buffer(&self.object_buffer)
                                        .bind_sampler(&self.sampler)
                                        .bind_texture(&texture.view(&Default::default()))
                                        .build(gpu, &self.object_bind_group_layout)
                                });

                            DrawCommand {
                                first_instance: instance,
                                count: 1,
                                shape: DrawShape::Rect,
                                fill_image: fill_image.clone(),
                            }
                        }
                    }
                })
                .coalesce(|prev, current| {
                    if prev.shape == current.shape && prev.fill_image == current.fill_image {
                        assert!(prev.first_instance + prev.count == current.first_instance);

                        Ok(DrawCommand {
                            first_instance: prev.first_instance,
                            count: prev.count + 1,
                            shape: prev.shape,
                            fill_image: prev.fill_image,
                        })
                    } else {
                        Err((prev, current))
                    }
                }),
        );
    }

    pub fn draw<'a>(
        &'a mut self,
        gpu: &Gpu,
        globals_bind_group: &'a wgpu::BindGroup,
        render_pass: &mut RenderPass<'a>,
    ) -> anyhow::Result<()> {
        self.object_buffer.write(&gpu.queue, &self.objects);

        render_pass.set_pipeline(self.shader.pipeline());
        render_pass.set_bind_group(0, globals_bind_group, &[]);

        self.quad.bind(render_pass);

        tracing::info!("Draw commands: {}", self.commands.len());

        for cmd in std::mem::take(&mut self.commands) {
            let bind_group = self.bind_groups.get(&cmd.fill_image).unwrap();
            match &cmd.shape {
                DrawShape::Rect => {
                    tracing::debug!(
                        "Drawing instances {}..{}",
                        cmd.first_instance,
                        cmd.first_instance + cmd.count
                    );
                    render_pass.set_bind_group(1, bind_group, &[]);

                    render_pass.draw_indexed(
                        0..6,
                        0,
                        cmd.first_instance..(cmd.first_instance + cmd.count),
                    )
                }
            }
        }

        Ok(())
    }
}

fn accumulate_shapes(world: &World, id: Entity, f: &mut impl FnMut(Rect, &Shape)) {
    let entity = world.entity(id).unwrap();
    if let Ok(shape) = entity.get(shape()) {
        let rect = entity.get(rect()).ok();
        (f)((rect.map(|v| *v)).unwrap_or_default(), &shape)
    }

    if let Ok(children) = entity.get(children()) {
        for &child in children.iter() {
            accumulate_shapes(world, child, f);
        }
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct ObjectData {
    world_matrix: Mat4,
    color: Vec4,
}

fn srgba_to_vec4(color: Srgba) -> Vec4 {
    let (r, g, b, a) = color.into_linear().into_components();

    vec4(r, g, b, a)
}
