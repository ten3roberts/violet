use image::{DynamicImage, ImageError, ImageResult};
use std::path::Path;

use crate::assets::{Asset, AssetCache, AssetKey};

// impl Loadable<Path> for DynamicImage {
//     type Error = ImageError;

//     fn load(&self, _: &AssetCache) -> ImageResult<DynamicImage> {
//         Ok(image::open(self)?)
//     }
// }

impl AssetKey<DynamicImage> for Path {
    type Error = ImageError;

    fn load(&self, assets: &AssetCache) -> ImageResult<Asset<DynamicImage>> {
        Ok(assets.insert(image::open(self)?))
    }
}

// #[derive(PartialEq, Eq, Hash, Debug, Clone)]
// struct SolidTextureKey {
//     color: Rgba<u8>,
//     gpu: Asset<Gpu>,
// }

// impl AssetKey for SolidTextureKey {
//     type Output = Texture;
//     type Error = Infallible;

//     fn load(self, _: &AssetCache) -> Result<Self::Output, Infallible> {
//         let contents = ImageBuffer::from_pixel(256, 256, self.color);

//         Ok(Texture::from_image(
//             &self.gpu,
//             &DynamicImage::ImageRgba8(contents),
//         ))
//     }
// }
