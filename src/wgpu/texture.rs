use std::convert::Infallible;

use image::{DynamicImage, ImageBuffer, Rgba};

use crate::assets::{AssetCache, AssetKey, Handle};

use super::{graphics::texture::Texture, Gpu};

/// Load a texture from in memory data
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct TextureFromImage {
    gpu: Handle<Gpu>,
    image: Handle<DynamicImage>,
}

impl AssetKey for TextureFromImage {
    type Output = Texture;
    type Error = Infallible;

    fn load(self, _: &AssetCache) -> Result<Self::Output, Infallible> {
        Ok(Texture::from_image(&self.gpu, &self.image))
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
struct SolidTextureKey {
    color: Rgba<u8>,
    gpu: Handle<Gpu>,
}

impl AssetKey for SolidTextureKey {
    type Output = Texture;
    type Error = Infallible;

    fn load(self, _: &AssetCache) -> Result<Self::Output, Infallible> {
        let contents = ImageBuffer::from_pixel(256, 256, self.color);

        Ok(Texture::from_image(
            &self.gpu,
            &DynamicImage::ImageRgba8(contents),
        ))
    }
}
