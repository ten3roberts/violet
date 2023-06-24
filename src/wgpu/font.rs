use std::collections::BTreeMap;

use glam::{ivec2, uvec2, IVec2};
use guillotiere::{size2, AtlasAllocator};
use image::{DynamicImage, ImageBuffer, Luma};

use crate::assets::{fs::BytesFromFile, AssetCache, AssetKey, Handle};

/// Loads a font from memory
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FontFromBytes {
    pub bytes: Handle<Vec<u8>>,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct FontFromFile {
    path: BytesFromFile,
}

impl AssetKey for FontFromFile {
    type Output = Font;

    fn load(&self, assets: &AssetCache) -> Self::Output {
        let bytes = assets.load(&self.path);

        FontFromBytes { bytes }.load(assets)
    }
}

pub struct Font {
    pub(crate) font: fontdue::Font,
}

impl AssetKey for FontFromBytes {
    type Output = Font;

    fn load(&self, assets: &crate::assets::AssetCache) -> Self::Output {
        let bytes = &*self.bytes;
        let font = fontdue::Font::from_bytes(bytes.as_ref(), fontdue::FontSettings::default())
            .map_err(|v| anyhow::anyhow!("Error loading font: {v:?}"))
            .unwrap();

        Font { font }
    }
}

struct GlyphLocation {
    position: IVec2,
}

pub struct FontAtlas {
    pub image: Handle<DynamicImage>,
    glyphs: BTreeMap<char, GlyphLocation>,
}

impl FontAtlas {
    pub fn new(
        assets: &mut AssetCache,
        font: &Font,
        px: f32,
        glyphs: impl IntoIterator<Item = char>,
    ) -> anyhow::Result<Self> {
        let size = uvec2(512, 512);
        let mut atlas = AtlasAllocator::new(size2(size.x as _, size.y as _));

        let mut image = ImageBuffer::from_pixel(size.x as _, size.y as _, Luma([0]));

        let glyphs = glyphs
            .into_iter()
            .map(|glyph| {
                let (metrics, pixels) = font.font.rasterize(glyph, px);

                let padding = 10;

                let v = atlas
                    .allocate(size2(
                        metrics.width as i32 + padding * 2,
                        metrics.height as i32 + padding * 2,
                    ))
                    .unwrap();

                let position = ivec2(v.rectangle.min.x + padding, v.rectangle.min.y + padding);

                blit_to_image(
                    &pixels,
                    &mut image,
                    position,
                    metrics.width,
                    size.x as usize,
                );

                Ok((glyph, GlyphLocation { position })) as anyhow::Result<_>
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        // let texture = gpu.device.create_texture_with_data(
        //     &gpu.queue,
        //     &TextureDescriptor {
        //         label: Some("FontAtlas"),
        //         size: Extent3d {
        //             width: size.x,
        //             height: size.y,
        //             depth_or_array_layers: 1,
        //         },
        //         mip_level_count: 1,
        //         sample_count: 1,
        //         dimension: TextureDimension::D2,
        //         format: wgpu::TextureFormat::R8Unorm,
        //         usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        //         view_formats: &[],
        //     },
        //     &atlas_data,
        // );
        //
        // let texture = assets.insert(Texture::from_texture(texture));

        Ok(Self {
            image: assets.insert(image::DynamicImage::ImageLuma8(image)),
            glyphs,
        })
    }
}

pub fn blit_to_image(
    src: &[u8],
    dst: &mut [u8],
    position: IVec2,
    src_width: usize,
    dst_width: usize,
) {
    for (row_index, row) in src.chunks_exact(src_width).enumerate() {
        let dst_index = position.x as usize + (position.y as usize + row_index) * dst_width;

        dst[dst_index..(dst_index + src_width)].copy_from_slice(row);
    }
}
