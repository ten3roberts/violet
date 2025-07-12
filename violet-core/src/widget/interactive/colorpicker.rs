use std::{fmt::Display, str::FromStr, sync::Arc};

use futures::StreamExt;
use palette::{IntoColor, Srgb, Srgba, WithAlpha};

use crate::{
    state::{StateDuplex, StateExt, StateStreamRef},
    style::{
        base_colors::{EMERALD_400, OCEAN_400, PLATINUM_100, PLATINUM_400, RUBY_400},
        default_corner_radius, spacing_small, surface_tertiary, SizeExt,
    },
    unit::Unit,
    widget::{card, col, row, InputBox, LabeledSlider, Rectangle, Stack, StreamWidget, TextInput},
    Widget,
};

pub struct RgbColorPicker {
    color: Box<dyn Send + Sync + StateDuplex<Item = Srgba>>,
    enable_alpha: bool,
}

impl RgbColorPicker {
    pub fn new(color: impl 'static + Send + Sync + StateDuplex<Item = Srgba>) -> Self {
        Self {
            color: Box::new(color),
            enable_alpha: true,
        }
    }
}

impl Widget for RgbColorPicker {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        let color = Arc::new(self.color.memo(Default::default()));

        let red = color.clone().project_ref(|v| &v.red, |v| &mut v.red);
        let green = color.clone().project_ref(|v| &v.green, |v| &mut v.green);
        let blue = color.clone().project_ref(|v| &v.blue, |v| &mut v.blue);
        let alpha = color.clone().project_ref(|v| &v.alpha, |v| &mut v.alpha);

        let sliders = col((
            LabeledSlider::input(red, 0.0, 1.0)
                .precision(2)
                .with_fill_color(RUBY_400),
            LabeledSlider::input(green, 0.0, 1.0)
                .precision(2)
                .with_fill_color(EMERALD_400),
            LabeledSlider::input(blue, 0.0, 1.0)
                .precision(2)
                .with_fill_color(OCEAN_400),
            self.enable_alpha.then(|| {
                LabeledSlider::input(alpha, 0.0, 1.0)
                    .precision(2)
                    .with_fill_color(PLATINUM_100)
            }),
        ));

        card(row((
            col((
                StreamWidget::new(color.stream_ref(|&v| {
                    Rectangle::new(v)
                        .with_padding(spacing_small())
                        .with_corner_radius(default_corner_radius())
                        .with_exact_size(Unit::px2(80.0, 80.0))
                })),
                color_hex_editor(color),
            )),
            sliders,
        )))
        .with_background(surface_tertiary())
        .mount(scope);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ColorAsHex(Srgba);

impl Display for ColorAsHex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex: Srgba<u8> = self.0.into_format();
        write!(f, "#{:0>2x}{:0>2x}{:0>2x}", hex.red, hex.green, hex.blue)
    }
}

impl FromStr for ColorAsHex {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let s = if s.starts_with("#") {
            &s[1..]
        } else if s.starts_with("0x") {
            &s[2..]
        } else {
            s
        };

        let rgb = if s.len() == 3 || s.len() == 4 {
            let r = u8::from_str_radix(&s[0..1], 8).map_err(|_| "Invalid red value")?;
            let g = u8::from_str_radix(&s[1..2], 8).map_err(|_| "Invalid green value")?;
            let b = u8::from_str_radix(&s[2..3], 8).map_err(|_| "Invalid blue value")?;
            let a = if s.len() == 4 {
                u8::from_str_radix(&s[3..4], 8).map_err(|_| "Invalid alpha value")?
            } else {
                15
            };

            Srgba::new(r * 17, g * 17, b * 17, a * 17) // Expand single hex digits to full range
        } else if s.len() == 6 || s.len() == 8 {
            let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| "Invalid red value")?;
            let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| "Invalid green value")?;
            let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| "Invalid blue value")?;
            let a = if s.len() == 8 {
                u8::from_str_radix(&s[6..8], 16).map_err(|_| "Invalid alpha value")?
            } else {
                255 // Default alpha to fully opaque if not provided
            };
            Srgba::new(r, g, b, a)
        } else {
            return Err("Color must be in the format #RRGGBB".to_string());
        };

        Ok(ColorAsHex(rgb.into_format()))
    }
}

fn color_hex_editor(color: impl 'static + Send + Sync + StateDuplex<Item = Srgba>) -> impl Widget {
    let color = Arc::new(color.memo(Default::default())).map_value(ColorAsHex, |v| v.0);

    InputBox::new(color)
        .with_max_size(Unit::px2(80.0, 24.0))
        .with_min_size(Unit::px2(80.0, 12.0))
}
