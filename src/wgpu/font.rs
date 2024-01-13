use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use cosmic_text::{CacheKey, Placement, SwashImage};
use fontdue::Font;
use glam::{uvec2, vec3, UVec2};
use guillotiere::{size2, AtlasAllocator};
use image::{ImageBuffer, Luma};
use wgpu::{util::DeviceExt, Extent3d, TextureDescriptor, TextureDimension, TextureUsages};

use crate::assets::{fs::BytesFromFile, Asset, AssetCache, AssetKey};

use super::{graphics::texture::Texture, text_renderer::TextSystem, Gpu};

/// Loads a font from memory
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FontFromBytes {
    pub bytes: Asset<Vec<u8>>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FontFromFile {
    pub path: PathBuf,
}

impl AssetKey for FontFromFile {
    type Output = Font;
    type Error = anyhow::Error;

    fn load(self, assets: &AssetCache) -> anyhow::Result<Self::Output> {
        let bytes = assets.try_load(&BytesFromFile(self.path))?;

        FontFromBytes { bytes }.load(assets)
    }
}

impl AssetKey for FontFromBytes {
    type Output = Font;
    type Error = anyhow::Error;

    fn load(self, _assets: &crate::assets::AssetCache) -> anyhow::Result<Self::Output> {
        let bytes = &*self.bytes;
        fontdue::Font::from_bytes(bytes.as_ref(), fontdue::FontSettings::default())
            .map_err(|v| anyhow::anyhow!("Error loading font: {v:?}"))
    }
}

/// A glyphs location in the text atlas
#[derive(Copy, Clone)]
pub struct GlyphLocation {
    pub min: UVec2,
    pub max: UVec2,
}

pub struct FontAtlas {
    /// The backing GPU texture of the rasterized fonts
    pub texture: Texture,
    pub glyphs: BTreeMap<CacheKey, (Placement, GlyphLocation)>,
}

impl FontAtlas {
    pub fn empty(gpu: &Gpu) -> Self {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("FontAtlas"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        Self {
            texture: Texture::from_texture(texture),
            glyphs: Default::default(),
        }
    }

    pub fn new(
        _assets: &AssetCache,
        gpu: &Gpu,
        text_system: &mut TextSystem,
        glyphs: impl IntoIterator<Item = CacheKey>,
    ) -> anyhow::Result<Self> {
        let mut atlas = AtlasAllocator::new(size2(128, 128));

        let glyphs = glyphs.into_iter().collect::<BTreeSet<_>>();

        // let images = glyphs
        //     .iter()
        //     .map(|&glyph| {
        //         let image = swash_cache.get_image_uncached(font_system, glyph).unwrap();

        //         (glyph, image)
        //     })
        //     .collect_vec();

        let mut images = Vec::new();
        let glyphs = glyphs
            .iter()
            .map(|&glyph| {
                let image = text_system
                    .swash_cache
                    .get_image_uncached(&mut text_system.font_system, glyph)
                    .unwrap();
                // let index = font.lookup_glyph_index(c);

                let metrics = image.placement;
                let padding = 2;

                let requested_size = size2(
                    metrics.width as i32 + padding * 2,
                    metrics.height as i32 + padding * 2,
                );
                let v = loop {
                    if let Some(v) = atlas.allocate(requested_size) {
                        break v;
                    } else {
                        atlas.grow(atlas.size() * 2)
                    }
                };

                let min = uvec2(
                    (v.rectangle.min.x + padding) as u32,
                    (v.rectangle.min.y + padding) as u32,
                );

                let max = uvec2(
                    (v.rectangle.max.x - padding) as u32,
                    (v.rectangle.max.y - padding) as u32,
                );

                let loc = GlyphLocation { min, max };
                images.push((glyph, image, loc));
                (glyph, (metrics, loc))
            })
            .collect::<BTreeMap<_, _>>();

        let size = atlas.size();
        let size = uvec2(size.width as _, size.height as _);
        let mut image = ImageBuffer::from_pixel(size.x, size.y, Luma([0]));

        images.iter().for_each(|(_, src_image, loc)| {
            if src_image.placement.width > 0 {
                blit_to_image(
                    src_image,
                    &mut image,
                    loc.min.x as i32,
                    loc.min.y as i32,
                    size.x,
                );
            }
        });

        let texture = gpu.device.create_texture_with_data(
            &gpu.queue,
            &TextureDescriptor {
                label: Some("FontAtlas"),
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            &image,
        );

        Ok(Self {
            texture: Texture::from_texture(texture),
            glyphs,
        })
    }

    pub(crate) fn size(&self) -> Extent3d {
        self.texture.size()
    }
}

pub fn blit_to_image(src: &SwashImage, dst: &mut [u8], x: i32, y: i32, dst_width: u32) {
    match src.content {
        cosmic_text::SwashContent::Mask => {
            for (row_index, row) in src
                .data
                .chunks_exact(src.placement.width as usize)
                .enumerate()
            {
                let dst_index = x as usize + (y as usize + row_index) * dst_width as usize;

                dst[dst_index..(dst_index + src.placement.width as usize)].copy_from_slice(row);
            }
        }
        cosmic_text::SwashContent::SubpixelMask => todo!(),
        cosmic_text::SwashContent::Color => {
            for (row_index, row) in src
                .data
                .chunks_exact(src.placement.width as usize * 4)
                .enumerate()
            {
                let dst_index = x as usize + (y as usize + row_index) * dst_width as usize;

                for (pixel_index, pixel) in row.chunks_exact(4).enumerate() {
                    let dst_index = dst_index + pixel_index;

                    let l = vec3(
                        pixel[0] as f32 / 255.0,
                        pixel[1] as f32 / 255.0,
                        pixel[2] as f32 / 255.0,
                    )
                    .dot(vec3(0.2126, 0.7152, 0.0722));

                    dst[dst_index] = ((l * pixel[3] as f32 / 255.0) * 255.0) as u8;
                }
            }
        }
    }
}
