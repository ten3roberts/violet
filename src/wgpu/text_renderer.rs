use std::{
    collections::{btree_map, BTreeMap, BTreeSet},
    sync::Arc,
};

use cosmic_text::{
    Attrs, BorrowedWithFontSystem, Buffer, CacheKey, FontSystem, Metrics, Placement, Shaping,
    SwashCache,
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
use glam::{ivec3, vec2, vec3, Mat4, Quat, Vec2, Vec3};
use itertools::Itertools;
use parking_lot::Mutex;
use wgpu::{BindGroup, BindGroupLayout, Sampler, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    assets::{AssetCache, Handle},
    components::{font_size, rect, screen_position, text, text_limits, Rect},
    wgpu::{
        font::FontAtlas,
        graphics::{allocator::Allocation, BindGroupBuilder},
        shape_renderer::DrawCommand,
    },
    Frame,
};

use super::{
    components::{draw_cmd, font_handle, mesh_handle, model_matrix},
    font::GlyphLocation,
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
    rasterized: RasterizedFont,
    sampler: Handle<Sampler>,
    text_layout: BindGroupLayout,
}

impl FontRasterizer {
    fn new(
        assets: &AssetCache,
        gpu: &Gpu,
        sampler: Handle<Sampler>,
        text_layout: BindGroupLayout,
    ) -> Self {
        let atlas = FontAtlas::empty(gpu);

        let bind_group = assets.insert(
            BindGroupBuilder::new("TextRenderer::bind_group")
                .bind_sampler(&sampler)
                .bind_texture(&atlas.texture.view(&Default::default()))
                .build(&gpu, &text_layout),
        );

        Self {
            rasterized: RasterizedFont { atlas, bind_group },
            sampler,
            text_layout,
        }
    }

    pub fn add_glyphs(
        &mut self,
        assets: &AssetCache,
        ctx: &mut RendererContext,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        new_glyphs: &[CacheKey],
    ) -> anyhow::Result<()> {
        let glyphs = self
            .rasterized
            .atlas
            .glyphs
            .keys()
            .chain(new_glyphs)
            .copied();

        let atlas = FontAtlas::new(assets, &ctx.gpu, font_system, swash_cache, glyphs)?;

        let bind_group = assets.insert(
            BindGroupBuilder::new("TextRenderer::bind_group")
                .bind_sampler(&self.sampler)
                .bind_texture(&atlas.texture.view(&Default::default()))
                .build(&ctx.gpu, &self.text_layout),
        );

        self.rasterized = RasterizedFont { atlas, bind_group };

        Ok(())
    }

    pub fn get_glyph(&self, glyph: CacheKey) -> Option<&(Placement, GlyphLocation)> {
        self.rasterized.atlas.glyphs.get(&glyph)
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
            rasterizer: FontRasterizer::new(&frame.assets, &ctx.gpu, sampler, text_layout),
            shader,
        }
    }

    fn update_mesh(
        &mut self,
        ctx: &mut RendererContext,
        assets: &AssetCache,
        font_size: f32,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        buffer: &mut Buffer,
        // text: &str,
        mesh: &mut Arc<MeshHandle>,
    ) -> u32 {
        // let rasterized = self
        //     .rasterizer
        //     .get(ctx, assets, font.clone(), font_size, text);

        let mut vertices = Vec::new();

        // let color = cosmic_text::Color::rgb(0xFF, 0xFF, 0xFF);

        let mut missing = Vec::new();
        loop {
            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let physical_glyph = glyph.physical((0., 0.), 1.0);
                    let Some((placement, loc)) =
                        self.rasterizer.get_glyph(physical_glyph.cache_key)
                    else {
                        missing.push(physical_glyph.cache_key);
                        continue;
                    };

                    let atlas_size = self.rasterizer.rasterized.atlas.size();
                    let atlas_size = vec2(atlas_size.width as f32, atlas_size.height as f32);

                    let uv_min = loc.min.as_vec2() / atlas_size;
                    let uv_max = loc.max.as_vec2() / atlas_size;

                    let x = placement.left as f32 + physical_glyph.x as f32;
                    let y = run.line_y - placement.top as f32 + physical_glyph.y as f32;

                    vertices.extend_from_slice(&[
                        // Bottom left
                        Vertex::new(
                            vec3(x, y + placement.height as f32, 0.0),
                            vec2(uv_min.x, uv_max.y),
                        ),
                        Vertex::new(
                            vec3(x + placement.width as f32, y + placement.height as f32, 0.0),
                            vec2(uv_max.x, uv_max.y),
                        ),
                        Vertex::new(
                            vec3(x + placement.width as f32, y, 0.0),
                            vec2(uv_max.x, uv_min.y),
                        ),
                        Vertex::new(vec3(x, y, 0.0), vec2(uv_min.x, uv_min.y)),
                    ])
                }
            }

            if missing.is_empty() {
                break;
            } else {
                self.rasterizer
                    .add_glyphs(assets, ctx, font_system, swash_cache, &missing)
                    .unwrap();

                missing.clear();
            }
        }
        let glyph_count = vertices.len() / 4;

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

        indices.len() as u32
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
    text: Component<String>,
    text_limits: Component<Vec2>,
    #[fetch(ignore)]
    font_size: OptOr<Component<f32>, f32>,
}

impl TextMeshQuery {
    fn new() -> Self {
        Self {
            id: entity_ids(),
            mesh: mesh_handle().as_mut().opt(),
            rect: rect(),
            text: text(),
            text_limits: text_limits(),
            font_size: font_size().opt_or(16.0),
        }
    }
}

pub struct RasterizedFont {
    /// Stored to retrieve *where* the character is located
    atlas: FontAtlas,
    bind_group: Handle<BindGroup>,
}

pub struct TextRenderer {
    mesh_generator: MeshGenerator,
    font_system: Arc<Mutex<FontSystem>>,
    swash_cache: SwashCache,

    object_query: Query<ObjectQuery, (All, With)>,
    mesh_query: Query<<TextMeshQuery as TransformFetch<Modified>>::Output, All>,
}

impl TextRenderer {
    pub fn new(
        ctx: &mut RendererContext,
        frame: &mut Frame,
        font_system: Arc<Mutex<FontSystem>>,
        color_format: TextureFormat,
        object_layout: &BindGroupLayout,
    ) -> Self {
        let mesh_generator = MeshGenerator::new(ctx, frame, color_format, object_layout);
        Self {
            object_query: Query::new(ObjectQuery::new()).with(text()),
            mesh_generator,
            mesh_query: Query::new(TextMeshQuery::new().modified()),
            font_system,
            swash_cache: SwashCache::new(),
        }
    }

    pub fn update_meshes(&mut self, ctx: &mut RendererContext, frame: &mut Frame) {
        let mut cmd = CommandBuffer::new();

        let font_system = &mut *self.font_system.lock();

        (self.mesh_query.borrow(&frame.world)).for_each(|item| {
            // tracing::debug!(%item.id, "updating mesh for {:?}", item.text);

            // Update intrinsic sizes

            // tracing::info!(?item.id, ?item.rect, "text rect");

            let metrics = Metrics::new(*item.font_size, *item.font_size);

            let mut buffer = Buffer::new(font_system, metrics);
            {
                let mut buffer = buffer.borrow_with(font_system);

                let limits = item.text_limits;

                buffer.set_text(item.text, Attrs::new(), Shaping::Advanced);
                buffer.set_size(limits.x, limits.y);

                buffer.shape_until_scroll();
            }
            // let mut vertices = Vec::new();
            // buffer.draw(
            //     &mut self.swash_cache,
            //     cosmic_text::Color::rbg(0xFF, 0xFF, 0xFF),
            //     |x, y, w, h, color| {},
            // );

            let mut layout = Layout::<()>::new(fontdue::layout::CoordinateSystem::PositiveYDown);

            // Due to padding the text may not fit exactly
            // let (max_width, max_height) = (Some(limits.x), Some(limits.y));

            // layout.reset(&fontdue::layout::LayoutSettings {
            //     // x: rect.min.x.round(),
            //     // y: rect.min.x.round(),
            //     x: 0.0,
            //     y: 0.0,
            //     max_width,
            //     max_height,
            //     horizontal_align: fontdue::layout::HorizontalAlign::Left,
            //     vertical_align: fontdue::layout::VerticalAlign::Top,
            //     line_height: 1.0,
            //     wrap_style: fontdue::layout::WrapStyle::Word,
            //     wrap_hard_breaks: true,
            // });

            // layout.append(
            //     &[&**item.font],
            //     &TextStyle {
            //         text: item.text,
            //         px: *item.font_size,
            //         font_index: 0,
            //         user_data: (),
            //     },
            // );

            let mut new_mesh = None;

            let mesh = match item.mesh {
                Some(v) => v,
                None => new_mesh.insert(Arc::new(ctx.mesh_buffer.allocate(&ctx.gpu, 0, 0))),
            };

            let index_count = self.mesh_generator.update_mesh(
                ctx,
                &frame.assets,
                *item.font_size,
                font_system,
                &mut self.swash_cache,
                &mut buffer,
                mesh,
            );

            cmd.set(
                item.id,
                draw_cmd(),
                DrawCommand {
                    bind_group: self.mesh_generator.rasterizer.rasterized.bind_group.clone(),
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
