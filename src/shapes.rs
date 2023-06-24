use image::DynamicImage;
use palette::Srgba;

use crate::assets::Handle;

/// A rectangle sized to the widget
#[derive(Clone)]
pub struct FilledRect {
    pub color: Srgba,
    pub fill_image: Option<Handle<DynamicImage>>,
}

impl std::fmt::Debug for FilledRect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilledRect")
            .field("color", &self.color)
            .field("fill_image", &self.fill_image.as_ref().map(Handle::id))
            .finish()
    }
}
