use std::sync::Arc;

use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    filter::{All, With},
    CommandBuffer, Component, EntityIds, Fetch, FetchExt, Mutable, Opt, OptOr, Query,
};
use glam::{vec2, vec3, Mat4, Quat, Vec2};
use image::{DynamicImage, ImageBuffer};
use palette::Srgba;
use wgpu::{BindGroup, BindGroupLayout, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    assets::{map::HandleMap, Asset},
    components::{color, draw_shape, image, rect, screen_position, Rect},
    shape::{self, shape_rectangle},
    stored::{self, WeakHandle},
    Frame,
};

use super::{
    components::{draw_cmd, object_data},
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Shader,
        Vertex, VertexDesc,
    },
    mesh_buffer::MeshHandle,
    renderer::RendererContext,
    shape_renderer::{srgba_to_vec4, DrawCommand, ObjectData, RendererStore},
    Gpu,
};

#[derive(Fetch)]
struct RectObjectQuery {
    rect: Component<Rect>,
    pos: Component<Vec2>,
    color: OptOr<Component<Srgba>, Srgba>,
    object_data: Mutable<ObjectData>,
}

impl RectObjectQuery {
    fn new() -> Self {
        Self {
            rect: rect(),
            pos: screen_position(),
            object_data: object_data().as_mut(),
            color: color().opt_or(Srgba::new(1.0, 1.0, 1.0, 1.0)),
        }
    }
}

#[derive(Fetch)]
#[fetch(transforms = [Modified])]
struct RectDrawQuery {
    #[fetch(ignore)]
    id: EntityIds,
    image: Opt<Component<Asset<DynamicImage>>>,
    shape: Component<()>,
}

impl RectDrawQuery {
    fn new() -> Self {
        Self {
            id: entity_ids(),
            image: image().opt(),
            shape: draw_shape(shape::shape_rectangle()),
        }
    }
}

pub struct RectRenderer {
    white_image: Asset<DynamicImage>,

    layout: BindGroupLayout,
    sampler: wgpu::Sampler,

    rect_query: Query<<RectDrawQuery as TransformFetch<Modified>>::Output>,
    object_query: Query<RectObjectQuery, (All, With)>,

    bind_groups: HandleMap<DynamicImage, WeakHandle<BindGroup>>,

    mesh: Arc<MeshHandle>,

    shader: stored::Handle<Shader>,
}

impl RectRenderer {
    pub fn new(
        ctx: &mut RendererContext,
        frame: &Frame,
        color_format: TextureFormat,
        object_bind_group_layout: &BindGroupLayout,
        store: &mut RendererStore,
    ) -> Self {
        let layout = BindGroupLayoutBuilder::new("RectRenderer::layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(&ctx.gpu);

        let white_image = frame
            .assets
            .insert(DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
                256,
                256,
                image::Rgba([255, 255, 255, 255]),
            )));

        let sampler = ctx.gpu.device.create_sampler(&SamplerDescriptor {
            label: Some("ShapeRenderer::sampler"),
            anisotropy_clamp: 16,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let vertices = [
            Vertex::new(vec3(0.0, 0.0, 0.0), vec2(0.0, 0.0)),
            Vertex::new(vec3(1.0, 0.0, 0.0), vec2(1.0, 0.0)),
            Vertex::new(vec3(1.0, 1.0, 0.0), vec2(1.0, 1.0)),
            Vertex::new(vec3(0.0, 1.0, 0.0), vec2(0.0, 1.0)),
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let mesh = Arc::new(ctx.mesh_buffer.insert(&ctx.gpu, &vertices, &indices));

        let shader = store.shaders.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/solid.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, &object_bind_group_layout, &layout],
            },
        ));

        Self {
            white_image,
            layout,
            sampler,
            rect_query: Query::new(RectDrawQuery::new().modified()),
            object_query: Query::new(RectObjectQuery::new()).with(draw_shape(shape_rectangle())),
            bind_groups: HandleMap::new(),
            mesh,
            shader,
        }
    }

    pub fn build_commands(&mut self, gpu: &Gpu, frame: &mut Frame, store: &mut RendererStore) {
        let mut cmd = CommandBuffer::new();
        self.rect_query
            .borrow(&frame.world)
            .iter()
            .for_each(|item| {
                let image = item.image.unwrap_or(&self.white_image);

                let bind_group = self
                    .bind_groups
                    .get(image)
                    .and_then(|v| v.upgrade(&store.bind_groups))
                    .unwrap_or_else(|| {
                        tracing::info!(image = ?image.id(), "create bind group for image");
                        let texture = Texture::from_image(gpu, image);

                        let bind_group =
                            BindGroupBuilder::new("ShapeRenderer::textured_bind_group")
                                .bind_sampler(&self.sampler)
                                .bind_texture(&texture.view(&Default::default()))
                                .build(gpu, &self.layout);

                        let bind_group = store.bind_groups.insert(bind_group);
                        self.bind_groups
                            .insert(image.clone(), bind_group.downgrade());
                        bind_group
                    });

                cmd.set(
                    item.id,
                    draw_cmd(),
                    DrawCommand {
                        bind_group: bind_group.clone(),
                        shader: self.shader.clone(),
                        mesh: self.mesh.clone(),
                        index_count: 6,
                        vertex_offset: 0,
                    },
                );
            });

        cmd.apply(&mut frame.world).unwrap();
    }

    pub fn update(&mut self, _: &Gpu, frame: &Frame) {
        self.object_query
            .borrow(&frame.world)
            .iter()
            .for_each(|item| {
                let rect = item.rect.translate(*item.pos).align_to_grid();

                let model_matrix = Mat4::from_scale_rotation_translation(
                    rect.size().extend(1.0),
                    Quat::IDENTITY,
                    rect.pos().extend(0.1),
                );

                *item.object_data = ObjectData {
                    model_matrix,
                    color: srgba_to_vec4(*item.color),
                };
            })
    }
}
