use image::DynamicImage;
use palette::Srgba;
use winit::event::ElementState;

use crate::{
    assets::AssetKey,
    components::{self, color, draw_shape, font_size, text},
    input::{on_focus, on_mouse_input},
    shape,
    text::TextSegment,
    Frame, Scope, Widget,
};

/// A rectangular widget
pub struct Rectangle {
    color: Srgba,
}

impl Rectangle {
    pub fn new(color: Srgba) -> Self {
        Self { color }
    }
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(draw_shape(shape::shape_rectangle()), ())
            .set(color(), self.color);
    }
}

pub struct Image<K> {
    image: K,
}

impl<K> Image<K> {
    pub fn new(image: K) -> Self {
        Self { image }
    }
}

impl<K: AssetKey<Output = DynamicImage>> Widget for Image<K> {
    fn mount(self, scope: &mut Scope) {
        let image = scope.assets_mut().try_load(&self.image).ok();
        if let Some(image) = image {
            scope
                .set(draw_shape(shape::shape_rectangle()), ())
                .set(components::image(), image);
        } else {
            Text::new("Image not found")
                .with_color(Srgba::new(1.0, 0.0, 0.0, 1.0))
                .mount(scope);
        }
    }
}

pub struct Text {
    color: Option<Srgba>,
    text: Vec<TextSegment>,
    font_size: f32,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: vec![TextSegment::new(text.into())],
            color: None,
            font_size: 16.0,
        }
    }

    pub fn rich(text: impl IntoIterator<Item = TextSegment>) -> Self {
        Self {
            text: text.into_iter().collect(),
            color: None,
            font_size: 16.0,
        }
    }

    /// Set the font_size
    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Set the text color
    pub fn with_color(mut self, color: Srgba) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for Text {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(draw_shape(shape::shape_text()), ())
            .set(font_size(), self.font_size)
            .set(text(), self.text)
            .set_opt(color(), self.color);
    }
}

type ButtonCallback = Box<dyn Send + Sync + FnMut(&Frame, winit::event::MouseButton)>;

/// A button which invokes the callback when clicked
pub struct Button {
    normal_color: Srgba,
    pressed_color: Srgba,

    on_click: ButtonCallback,
}

impl Button {
    pub fn new(normal_color: Srgba, pressed_color: Srgba, on_click: ButtonCallback) -> Self {
        Self {
            normal_color,
            pressed_color,
            on_click,
        }
    }
}

impl Widget for Button {
    fn mount(mut self, scope: &mut Scope<'_>) {
        scope
            .set(draw_shape(shape::shape_rectangle()), ())
            .set(color(), self.normal_color)
            .set(
                on_focus(),
                Box::new(move |_, entity, focus| {
                    entity.update_dedup(
                        color(),
                        if focus {
                            self.pressed_color
                        } else {
                            self.normal_color
                        },
                    );
                }),
            )
            .set(
                on_mouse_input(),
                Box::new(move |frame, _, state, button| {
                    if state == ElementState::Released {
                        (self.on_click)(frame, button);
                    }
                }),
            );
    }
}
