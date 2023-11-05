use std::{
    collections::{btree_map, BTreeMap},
    sync::Arc,
};

use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    filter::{All, With},
    CommandBuffer, Component, Debuggable, EntityIds, Fetch, FetchExt, Mutable, Opt, OptOr, Query,
};
use fontdue::{
    layout::{Layout, TextStyle},
    Font,
};
use glam::{vec2, vec3, Mat4, Quat, Vec2, Vec3};
use itertools::Itertools;
use wgpu::{BindGroup, BindGroupLayout, Sampler, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    assets::{AssetCache, Handle},
    components::{font_size, intrinsic_size, rect, screen_position, text, Rect},
    wgpu::{
        font::FontAtlas,
        graphics::{allocator::Allocation, BindGroupBuilder},
        shape_renderer::DrawCommand,
    },
    Frame,
};

use super::{
    components::{draw_cmd, font, mesh_handle, model_matrix},
    graphics::{shader::ShaderDesc, BindGroupLayoutBuilder, Shader, Vertex, VertexDesc},
    mesh_buffer::MeshHandle,
    renderer::RendererContext,
    Gpu,
};

flax::component! {
    text_mesh: Allocation => [ Debuggable ],
}

#[derive(Fetch)]
struct ObjectQuery {
    rect: Component<Rect>,
    pos: Component<Vec2>,
    model_matrix: Mutable<Mat4>,
}

impl ObjectQuery {
    pub fn new() -> Self {
        Self {
            rect: rect(),
            pos: screen_position(),
            model_matrix: model_matrix().as_mut(),
        }
    }
}

struct FontRasterizer {
    rasterized: BTreeMap<(Handle<Font>, u32), RasterizedFont>,
    sampler: Handle<Sampler>,
    text_layout: BindGroupLayout,
}

impl FontRasterizer {
    pub fn get(
        &mut self,
        ctx: &RendererContext,
        assets: &AssetCache,
        font: Handle<Font>,
        px: f32,
        text: &str,
    ) -> &RasterizedFont {
        match self.rasterized.entry((font, px as u32)) {
            btree_map::Entry::Vacant(slot) => {
                let font = &slot.key().0;
                let atlas = FontAtlas::new(assets, &ctx.gpu, font, px, text.chars()).unwrap();

                let bind_group = assets.insert(
                    BindGroupBuilder::new("TextRenderer::bind_group")
                        .bind_sampler(&self.sampler)
                        .bind_texture(&atlas.texture.view(&Default::default()))
                        .build(&ctx.gpu, &self.text_layout),
                );

                &*slot.insert(RasterizedFont { atlas, bind_group })
            }
            btree_map::Entry::Occupied(mut slot) => {
                let font = slot.key().0.clone();

                let rasterized = slot.get_mut();

                if !text.chars().all(|c| rasterized.atlas.contains_char(c)) {
                    let missing = text
                        .chars()
                        .filter(|&c| !rasterized.atlas.contains_char(c))
                        .sorted();

                    tracing::info!(
                        missing = ?missing.collect_vec(),
                        "Atlas missing characters, re-rasterizing"
                    );

                    let mut chars = std::mem::take(&mut rasterized.atlas.chars);

                    chars.extend(text.chars());

                    let atlas = FontAtlas::new(assets, &ctx.gpu, &font, px, chars).unwrap();

                    let bind_group = assets.insert(
                        BindGroupBuilder::new("TextRenderer::bind_group")
                            .bind_sampler(&self.sampler)
                            .bind_texture(&atlas.texture.view(&Default::default()))
                            .build(&ctx.gpu, &self.text_layout),
                    );

                    let rasterized = slot.into_mut();
                    *rasterized = RasterizedFont { atlas, bind_group };
                    rasterized
                } else {
                    slot.into_mut()
                }
            }
        }
    }
}

struct MeshGenerator {
    rasterizer: FontRasterizer,
    shader: Handle<Shader>,
}

impl MeshGenerator {
    fn new(
        ctx: &mut RendererContext,
        frame: &mut Frame,
        color_format: TextureFormat,
        object_layout: &BindGroupLayout,
    ) -> Self {
        let text_layout = BindGroupLayoutBuilder::new("TextRenderer::text_layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(&ctx.gpu);

        let shader = frame.assets.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/text.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, object_layout, &text_layout],
            },
        ));

        let sampler = frame
            .assets
            .insert(ctx.gpu.device.create_sampler(&SamplerDescriptor {
                label: Some("ShapeRenderer::sampler"),
                anisotropy_clamp: 1,
                mag_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,

                ..Default::default()
            }));

        Self {
            rasterizer: FontRasterizer {
                rasterized: BTreeMap::new(),
                sampler,
                text_layout,
            },
            shader,
        }
    }

    fn update_mesh(
        &mut self,
        ctx: &mut RendererContext,
        assets: &AssetCache,
        font: &Handle<Font>,
        font_size: f32,
        layout: Layout,
        text: &str,
        mesh: &mut Arc<MeshHandle>,
    ) -> (&RasterizedFont, u32) {
        let rasterized = self
            .rasterizer
            .get(ctx, assets, font.clone(), font_size, text);

        let glyph_count = layout.glyphs().len();

        let vertices = layout
            .glyphs()
            .iter()
            .flat_map(|glyph| {
                let atlas_glyph = rasterized.atlas.glyphs.get(&glyph.key.glyph_index).unwrap();

                let glyph_width = glyph.width as f32;
                let glyph_height = glyph.height as f32;

                let atlas_size = rasterized.atlas.size();
                let atlas_size = vec2(atlas_size.width as f32, atlas_size.height as f32);

                let uv_min = atlas_glyph.min.as_vec2() / atlas_size;
                let uv_max = atlas_glyph.max.as_vec2() / atlas_size;

                [
                    // Bottom left
                    Vertex::new(
                        vec3(glyph.x, glyph.y + glyph_height, 0.0),
                        vec2(uv_min.x, uv_max.y),
                    ),
                    Vertex::new(
                        vec3(glyph.x + glyph_width, glyph.y + glyph_height, 0.0),
                        vec2(uv_max.x, uv_max.y),
                    ),
                    Vertex::new(
                        vec3(glyph.x + glyph_width, glyph.y, 0.0),
                        vec2(uv_max.x, uv_min.y),
                    ),
                    Vertex::new(vec3(glyph.x, glyph.y, 0.0), vec2(uv_min.x, uv_min.y)),
                ]
            })
            .collect_vec();

        let indices = (0..)
            .step_by(4)
            .take(glyph_count)
            .flat_map(|i| [i, 1 + i, 2 + i, 2 + i, 3 + i, i])
            .collect_vec();

        if mesh.vb().size() >= vertices.len() && mesh.ib().size() >= indices.len() {
            ctx.mesh_buffer.write(&ctx.gpu, mesh, &vertices, &indices);
        } else {
            tracing::info!(?glyph_count, "Allocating new mesh for text");
            *mesh = Arc::new(ctx.mesh_buffer.insert(&ctx.gpu, &vertices, &indices));
        }

        (rasterized, indices.len() as u32)
    }
}

#[derive(Fetch, Debug, Clone)]
#[fetch(transforms = [Modified])]
/// Query text entities in the world and allocate them a slot in the mesh and atlas
pub struct TextMeshQuery {
    #[fetch(ignore)]
    id: EntityIds,
    #[fetch(ignore)]
    mesh: Opt<Mutable<Arc<MeshHandle>>>,

    rect: Component<Rect>,
    intrinsic_size: Component<Vec2>,
    text: Component<String>,
    font: Component<Handle<Font>>,
    #[fetch(ignore)]
    font_size: OptOr<Component<f32>, f32>,
}

impl TextMeshQuery {
    fn new() -> Self {
        Self {
            id: entity_ids(),
            mesh: mesh_handle().as_mut().opt(),
            intrinsic_size: intrinsic_size(),
            rect: rect(),
            text: text(),
            font: font(),
            font_size: font_size().opt_or(16.0),
        }
    }
}

pub struct RasterizedFont {
    /// Stored to retrieve *where* the character is located
    atlas: FontAtlas,
    bind_group: Handle<BindGroup>,
}

pub struct RenderFont {
    font: Handle<Font>,
    sampler: Handle<Sampler>,
    rasterized: BTreeMap<u32, RasterizedFont>,
}

impl RenderFont {
    fn get(
        &mut self,
        gpu: &Gpu,
        assets: &AssetCache,
        px: u32,
        text: &str,
        text_layout: &BindGroupLayout,
    ) -> &mut RasterizedFont {
        let rasterize = || {
            let atlas = FontAtlas::new(assets, gpu, &self.font, px as f32, text.chars()).unwrap();

            let bind_group = assets.insert(
                BindGroupBuilder::new("TextRenderer::bind_group")
                    .bind_sampler(&self.sampler)
                    .bind_texture(&atlas.texture.view(&Default::default()))
                    .build(gpu, text_layout),
            );

            RasterizedFont { atlas, bind_group }
        };

        match self.rasterized.entry(px) {
            btree_map::Entry::Vacant(slot) => slot.insert(rasterize()),
            btree_map::Entry::Occupied(slot) => {
                let rasterized = slot.into_mut();
                if !text.chars().all(|c| rasterized.atlas.contains_char(c)) {
                    *rasterized = rasterize();
                }
                rasterized
            }
        }
    }
}

pub struct TextRenderer {
    mesh_generator: MeshGenerator,

    object_query: Query<ObjectQuery, (All, With)>,
    mesh_query: Query<<TextMeshQuery as TransformFetch<Modified>>::Output, All>,
}

impl TextRenderer {
    pub fn new(
        ctx: &mut RendererContext,
        frame: &mut Frame,
        color_format: TextureFormat,
        object_layout: &BindGroupLayout,
    ) -> Self {
        let mesh_generator = MeshGenerator::new(ctx, frame, color_format, object_layout);
        Self {
            object_query: Query::new(ObjectQuery::new()).with(text()),
            mesh_generator,
            mesh_query: Query::new(TextMeshQuery::new().modified()),
        }
    }

    pub fn update_meshes(&mut self, ctx: &mut RendererContext, frame: &mut Frame) {
        let mut cmd = CommandBuffer::new();

        (self.mesh_query.borrow(&frame.world)).for_each(|item| {
            // tracing::debug!(%item.id, "updating mesh for {:?}", item.text);

            // Update intrinsic sizes

            // tracing::info!(?item.id, ?item.rect, "text rect");
            let mut layout = Layout::<()>::new(fontdue::layout::CoordinateSystem::PositiveYDown);

            let rect = item.rect.align_to_grid();

            // Due to padding the text may not fit exactly
            let (max_width, max_height) =
                if rect.max.x >= item.intrinsic_size.x && rect.max.y >= item.intrinsic_size.y {
                    (None, None)
                } else {
                    (Some(rect.size().x), Some(rect.size().y))
                };

            layout.reset(&fontdue::layout::LayoutSettings {
                // x: rect.min.x.round(),
                // y: rect.min.x.round(),
                x: 0.0,
                y: 0.0,
                max_width,
                max_height,
                horizontal_align: fontdue::layout::HorizontalAlign::Left,
                vertical_align: fontdue::layout::VerticalAlign::Top,
                line_height: 1.0,
                wrap_style: fontdue::layout::WrapStyle::Word,
                wrap_hard_breaks: true,
            });

            layout.append(
                &[&**item.font],
                &TextStyle {
                    text: item.text,
                    px: *item.font_size,
                    font_index: 0,
                    user_data: (),
                },
            );

            let mut new_mesh = None;

            let mesh = match item.mesh {
                Some(v) => v,
                None => new_mesh.insert(Arc::new(ctx.mesh_buffer.allocate(&ctx.gpu, 0, 0))),
            };

            let (rasterized, index_count) = self.mesh_generator.update_mesh(
                ctx,
                &frame.assets,
                item.font,
                *item.font_size,
                layout,
                item.text,
                mesh,
            );

            cmd.set(
                item.id,
                draw_cmd(),
                DrawCommand {
                    bind_group: rasterized.bind_group.clone(),
                    shader: self.mesh_generator.shader.clone(),
                    index_count,
                    vertex_offset: 0,
                },
            );

            if let Some(v) = new_mesh {
                cmd.set(item.id, mesh_handle(), v);
            }
        });

        cmd.apply(&mut frame.world).unwrap();
    }

    pub fn update(&mut self, _: &Gpu, frame: &Frame) {
        self.object_query
            .borrow(&frame.world)
            .iter()
            .for_each(|item| {
                let rect = item.rect.translate(*item.pos).align_to_grid();
                *item.model_matrix = Mat4::from_scale_rotation_translation(
                    Vec3::ONE,
                    Quat::IDENTITY,
                    rect.pos().extend(0.1),
                );
            })
    }
}
