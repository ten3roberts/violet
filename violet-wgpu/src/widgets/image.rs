use palette::Srgba;
use violet_core::{
    assets::Asset,
    components::{color, draw_shape},
    shape,
    style::{SizeExt, WidgetSizeProps},
    Scope, Widget,
};
use wgpu::TextureView;

use crate::components;

pub struct RawImage {
    size: WidgetSizeProps,
    texture: Asset<TextureView>,
}

impl RawImage {
    pub fn new(texture: Asset<TextureView>) -> Self {
        Self {
            texture,
            size: Default::default(),
        }
    }
}

impl Widget for RawImage {
    fn mount(self, scope: &mut Scope) {
        self.size.mount(scope);
        scope
            .set(color(), Srgba::new(1.0, 1.0, 1.0, 1.0))
            .set(draw_shape(shape::shape_rectangle()), ())
            .set(components::texture_handle(), Some(self.texture));
    }
}

impl SizeExt for RawImage {
    fn size_mut(&mut self) -> &mut WidgetSizeProps {
        &mut self.size
    }
}
