use std::{convert::Infallible, sync::Arc};

use flax::{
    entity_ids,
    filter::{All, With},
    CommandBuffer, Component, ComponentMut, EntityIds, Fetch, FetchExt, Opt, OptOr, Query,
};
use glam::{vec2, vec3, vec4, Mat4, Quat, Vec4};
use image::{DynamicImage, ImageBuffer};
use palette::Srgba;
use violet_core::{
    assets::{map::HandleMap, Asset, AssetCache, AssetKey},
    components::{
        color, computed_opacity, draw_shape, image, rect, screen_transform, widget_corner_radius,
    },
    shape::{self, shape_rectangle},
    stored::{self, WeakHandle},
    unit::Unit,
    Frame, Rect,
};
use wgpu::{
    BindGroup, BindGroupLayout, SamplerDescriptor, ShaderStages, TextureFormat, TextureView,
};

use super::{DrawCommand, ObjectData, RendererStore};
use crate::{
    components::{draw_cmd, object_data, texture_handle},
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Shader,
        Vertex, VertexDesc,
    },
    mesh_buffer::MeshHandle,
    renderer::{srgba_to_vec4, RendererContext},
    Gpu,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ImageFromColor(pub [u8; 4]);

impl AssetKey<DynamicImage> for ImageFromColor {
    type Error = Infallible;

    fn load(&self, assets: &AssetCache) -> Result<Asset<DynamicImage>, Infallible> {
        Ok(
            assets.insert(DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
                32,
                32,
                image::Rgba(self.0),
            ))),
        )
    }
}

// impl Loadable<ImageFromColor> for DynamicImage {
//     type Error = Infallible;

//     fn load(key: &ImageFromColor, _: &AssetCache) -> Result<DynamicImage, Infallible> {
//         Ok(DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
//             32,
//             32,
//             image::Rgba(key.0),
//         )))
//     }
// }

#[derive(Fetch)]
struct RectObjectQuery {
    transform: Component<Mat4>,
    rect: Component<Rect>,
    // screen_rect: Component<Rect>,
    // rotation: OptOr<Component<f32>, f32>,
    // anchor: OptOr<Component<Unit<Vec2>>, Unit<Vec2>>,
    // pos: Component<Vec2>,
    // local_pos: Component<Vec2>,
    color: OptOr<Component<Srgba>, Srgba>,
    opacity: Component<f32>,
    object_data: ComponentMut<ObjectData>,
    corner_radius: OptOr<Component<Unit<f32>>, Unit<f32>>,
}

impl RectObjectQuery {
    fn new() -> Self {
        Self {
            // screen_rect: screen_rect(),
            // rotation: rotation().opt_or(0.0),
            rect: rect(),
            transform: screen_transform(),
            object_data: object_data().as_mut(),
            color: color().opt_or(Srgba::new(1.0, 1.0, 1.0, 1.0)),
            corner_radius: widget_corner_radius().opt_or_default(),
            opacity: computed_opacity(),
        }
    }
}

#[derive(Fetch)]
#[fetch(transforms = [Modified])]
struct RectDrawQuery {
    #[fetch(ignore)]
    id: EntityIds,
    image: Opt<Component<Asset<DynamicImage>>>,
    texture_handle: Opt<Component<Option<Asset<TextureView>>>>,
    shape: Component<()>,
}

impl RectDrawQuery {
    fn new() -> Self {
        Self {
            id: entity_ids(),
            image: image().opt(),
            texture_handle: texture_handle().opt(),
            shape: draw_shape(shape::shape_rectangle()),
        }
    }
}

pub struct RectRenderer {
    white_image: Asset<DynamicImage>,

    layout: BindGroupLayout,
    sampler: wgpu::Sampler,

    rect_query: Query<RectDrawQuery>,
    object_query: Query<RectObjectQuery, (All, With)>,

    bind_groups: HandleMap<DynamicImage, WeakHandle<BindGroup>>,
    textured_bind_groups: HandleMap<TextureView, WeakHandle<BindGroup>>,

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

        let white_image = frame.assets.load(&ImageFromColor([255, 255, 255, 255]));

        let sampler = ctx.gpu.device.create_sampler(&SamplerDescriptor {
            label: Some("ShapeRenderer::sampler"),
            anisotropy_clamp: 16,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let vertices = [
            Vertex::new(vec3(0.0, 0.0, 0.0), Vec4::ONE, vec2(0.0, 0.0)),
            Vertex::new(vec3(1.0, 0.0, 0.0), Vec4::ONE, vec2(1.0, 0.0)),
            Vertex::new(vec3(1.0, 1.0, 0.0), Vec4::ONE, vec2(1.0, 1.0)),
            Vertex::new(vec3(0.0, 1.0, 0.0), Vec4::ONE, vec2(0.0, 1.0)),
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let mesh = Arc::new(ctx.mesh_buffer.insert(&ctx.gpu, &vertices, &indices));

        let shader = store.shaders.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../../assets/shaders/solid.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, object_bind_group_layout, &layout],
            },
        ));

        Self {
            white_image,
            layout,
            sampler,
            rect_query: Query::new(RectDrawQuery::new()),
            object_query: Query::new(RectObjectQuery::new()).with(draw_shape(shape_rectangle())),
            bind_groups: HandleMap::new(),
            mesh,
            shader,
            textured_bind_groups: Default::default(),
        }
    }

    pub fn build_commands(&mut self, gpu: &Gpu, frame: &mut Frame, store: &mut RendererStore) {
        puffin::profile_function!();
        let mut cmd = CommandBuffer::new();
        self.rect_query
            .borrow(&frame.world)
            .iter()
            .for_each(|item| {
                let bind_group;

                if let Some(Some(handle)) = item.texture_handle {
                    bind_group = self
                        .textured_bind_groups
                        .get(handle)
                        .and_then(|v| v.upgrade(&store.bind_groups))
                        .unwrap_or_else(|| {
                            let bind_group =
                                BindGroupBuilder::new("ShapeRenderer::textured_bind_group")
                                    .bind_sampler(&self.sampler)
                                    .bind_texture(handle)
                                    .build(gpu, &self.layout);

                            let bind_group = store.bind_groups.insert(bind_group);
                            self.textured_bind_groups
                                .insert(handle.clone(), bind_group.downgrade());
                            bind_group
                        });
                } else {
                    let image = item.image.unwrap_or(&self.white_image);

                    bind_group = self
                        .bind_groups
                        .get(image)
                        .and_then(|v| v.upgrade(&store.bind_groups))
                        .unwrap_or_else(|| {
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
                }

                cmd.set(
                    item.id,
                    draw_cmd(),
                    DrawCommand {
                        bind_group: bind_group.clone(),
                        shader: self.shader.clone(),
                        mesh: self.mesh.clone(),
                        index_count: 6,
                    },
                );
            });

        cmd.apply(&mut frame.world).unwrap();
    }

    pub fn update(&mut self, _: &Gpu, frame: &Frame) {
        puffin::profile_function!();
        let _span = tracing::debug_span!("RectRenderer::update").entered();
        self.object_query
            .borrow(&frame.world)
            .iter()
            .for_each(|item| {
                let rect = item.rect.align_to_grid();

                let model_matrix = *item.transform
                    * Mat4::from_scale_rotation_translation(
                        rect.size().extend(1.0),
                        Quat::IDENTITY,
                        rect.pos().extend(0.0),
                    );

                *item.object_data = ObjectData {
                    model_matrix,
                    color: srgba_to_vec4(*item.color) * vec4(1.0, 1.0, 1.0, *item.opacity),
                    corner_radius: item.corner_radius.resolve(rect.size().min_element() / 2.0),
                    _padding: Default::default(),
                };
            })
    }
}
