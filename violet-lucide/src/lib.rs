use std::borrow::Cow;

use palette::Srgba;
use violet_core::{
    style::{StyleExt, ValueOrRef},
    text::{FontFamily, TextSegment},
    widget::{Text, TextStyle},
    Widget,
};

pub mod lucide_icons;

pub struct LucideIcon {
    icon: Cow<'static, str>,
    style: TextStyle,
}

impl LucideIcon {
    pub fn new(icon: impl Into<Cow<'static, str>>) -> Self {
        Self {
            icon: icon.into(),
            style: TextStyle::default(),
        }
    }

    /// Set the font_size
    pub fn with_font_size(mut self, font_size: impl Into<ValueOrRef<f32>>) -> Self {
        self.style.font_size = font_size.into();
        self
    }

    /// Set the text color
    pub fn with_color(mut self, color: impl Into<ValueOrRef<Srgba>>) -> Self {
        self.style.color = color.into();
        self
    }
}

impl Widget for LucideIcon {
    fn mount(self, scope: &mut violet_core::Scope<'_>) {
        Text::formatted([
            TextSegment::new(self.icon).with_family(FontFamily::Named("lucide".into()))
        ])
        .with_style(self.style)
        .mount(scope);
    }
}

impl StyleExt for LucideIcon {
    type Style = TextStyle;

    fn with_style(mut self, style: Self::Style) -> Self {
        self.style = style;
        self
    }
}

pub enum LucideIconName {}
