use glam::{vec2, Vec2};
use image::DynamicImage;
use palette::Srgba;
use winit::event::{ElementState, MouseButton};

use crate::{
    assets::AssetKey,
    components::{self, color, draw_shape, font_size, size, text, text_wrap},
    input::{on_focus, on_mouse_input},
    shape,
    style::StyleExt,
    text::{self, TextSegment, Wrap},
    unit::Unit,
    Frame, Scope, Widget,
};

use super::{ContainerExt, Stack};

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

impl<K> Widget for Image<K>
where
    K: AssetKey<DynamicImage>,
{
    fn mount(self, scope: &mut Scope) {
        let image = scope.assets_mut().try_load(&self.image).ok();
        if let Some(image) = image {
            scope
                .set(color(), Srgba::new(1.0, 1.0, 1.0, 1.0))
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
    wrap: Wrap,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: vec![TextSegment::new(text.into())],
            color: None,
            font_size: 16.0,
            wrap: Wrap::Word,
        }
    }

    pub fn rich(text: impl IntoIterator<Item = TextSegment>) -> Self {
        Self {
            text: text.into_iter().collect(),
            color: None,
            font_size: 16.0,
            wrap: Wrap::Word,
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

    pub fn with_wrap(mut self, wrap: Wrap) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Widget for Text {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(draw_shape(shape::shape_text()), ())
            .set(font_size(), self.font_size)
            .set(text_wrap(), self.wrap)
            .set(text(), self.text)
            .set_opt(color(), self.color);
    }
}

type ButtonCallback = Box<dyn Send + Sync + FnMut(&Frame, winit::event::MouseButton)>;

/// A button which invokes the callback when clicked
pub struct Button<W = Text> {
    background_color: Srgba,
    pressed_color: Srgba,

    on_press: ButtonCallback,
    container: Stack<W>,
}

impl<W> Button<W> {
    pub fn new(label: W) -> Self {
        let pressed_color = Srgba::new(0.1, 0.1, 0.1, 1.0);
        let background_color = Srgba::new(0.2, 0.2, 0.2, 1.0);
        Self {
            pressed_color,
            background_color,
            on_press: Box::new(|_, _| {}),
            container: Stack::new(label).with_background(Rectangle::new(background_color)),
        }
    }

    /// Set the background color
    pub fn with_background_color(mut self, background_color: Srgba) -> Self {
        self.background_color = background_color;
        self
    }

    /// Set the pressed color
    pub fn with_pressed_color(mut self, pressed_color: Srgba) -> Self {
        self.pressed_color = pressed_color;
        self
    }

    /// Handle the button press
    pub fn on_press(
        mut self,
        on_press: impl 'static + Send + Sync + FnMut(&Frame, MouseButton),
    ) -> Self {
        self.on_press = Box::new(on_press);
        self
    }
}

impl<W> ContainerExt for Button<W> {
    fn with_background<B: 'static + Widget>(mut self, background: B) -> Self {
        self.container = self.container.with_background(background);
        self
    }
}

impl<W: Widget> Widget for Button<W> {
    fn mount(mut self, scope: &mut Scope<'_>) {
        self.container
            .with_component(
                on_focus(),
                Box::new(move |_, entity, focus| {
                    entity.update_dedup(
                        color(),
                        if focus {
                            self.pressed_color
                        } else {
                            self.background_color
                        },
                    );
                }),
            )
            .with_component(
                on_mouse_input(),
                Box::new(move |frame, _, state, button| {
                    if state == ElementState::Released {
                        (self.on_press)(frame, button);
                    }
                }),
            )
            .mount(scope);
    }
}

/// Manually position a widget
pub struct Positioned<W> {
    offset: Unit<Vec2>,
    anchor: Unit<Vec2>,
    widget: W,
}

impl<W> Positioned<W> {
    pub fn new(widget: W) -> Self {
        Self {
            offset: Unit::ZERO,
            anchor: Unit::ZERO,
            widget,
        }
    }

    /// Sets the anchor point of the widget
    pub fn with_anchor(mut self, anchor: Unit<Vec2>) -> Self {
        self.anchor = anchor;
        self
    }

    /// Offsets the widget relative to its original position
    pub fn with_offset(mut self, offset: Unit<Vec2>) -> Self {
        self.offset = offset;
        self
    }
}

impl<W> Widget for Positioned<W>
where
    W: Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);

        scope.set(components::anchor(), self.anchor);
        scope.set(components::offset(), self.offset);
    }
}
