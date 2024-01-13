use image::DynamicImage;
use palette::{IntoColor, Srgba};

use crate::assets::Handle;

/// Shape to use when drawing a widget
#[derive(Debug, Clone)]
pub enum Shape {
    /// The widget will be drawn as a filled rectangle
    Rectangle,
}
