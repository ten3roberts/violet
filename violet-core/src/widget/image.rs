use image::DynamicImage;
use palette::Srgba;

use super::{label, Widget};
use crate::{
    assets::AssetKey,
    components::{self, color, draw_shape},
    shape,
    style::{surface_danger, SizeExt, WidgetSizeProps},
    Scope,
};

pub struct Image<K> {
    image: K,
    size: WidgetSizeProps,
    aspect_ratio: Option<f32>,
}

impl<K> Image<K> {
    pub fn new(image: K) -> Self {
        Self {
            image,
            size: Default::default(),
            aspect_ratio: None,
        }
    }

    pub fn with_aspect_ratio(mut self, aspect_ratio: f32) -> Self {
        self.aspect_ratio = Some(aspect_ratio);
        self
    }
}

impl<K> Widget for Image<K>
where
    K: AssetKey<DynamicImage>,
{
    fn mount(self, scope: &mut Scope) {
        let image = scope.assets_mut().try_load(&self.image).ok();
        if let Some(image) = image {
            self.size.mount(scope);
            scope
                .set(color(), Srgba::new(1.0, 1.0, 1.0, 1.0))
                .set(draw_shape(shape::shape_rectangle()), ())
                .set(components::image(), image)
                .set_opt(components::aspect_ratio(), self.aspect_ratio);
        } else {
            label("Image not found")
                .with_color(surface_danger())
                .mount(scope);
        }
    }
}

impl<K> SizeExt for Image<K> {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}
