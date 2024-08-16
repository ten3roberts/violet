use std::sync::Arc;

use futures::StreamExt;
use glam::Vec2;
use violet::{
    core::{
        state::{DynStateDuplex, State, StateStream},
        style::{SizeExt, ValueOrRef},
        unit::Unit,
        widget::{
            card, col, label, row, Radio, Rectangle, SliderWithLabel, StreamWidget, Text,
            TextInput, WidgetExt,
        },
        Scope, Widget,
    },
    futures_signals::signal::Mutable,
    palette::{rgb::Rgb, FromColor, IntoColor, OklabHue, Oklch, Srgb},
};

use crate::{color_hex, PaletteColor};

#[derive(Debug, Clone, Copy)]
enum EditorMode {
    Oklch,
    Rgb,
}

impl EditorMode {
    /// Returns `true` if the editor mode is [`Oklch`].
    ///
    /// [`Oklch`]: EditorMode::Oklch
    #[must_use]
    fn is_oklch(&self) -> bool {
        matches!(self, Self::Oklch)
    }

    /// Returns `true` if the editor mode is [`Rgb`].
    ///
    /// [`Rgb`]: EditorMode::Rgb
    #[must_use]
    fn is_rgb(&self) -> bool {
        matches!(self, Self::Rgb)
    }
}

pub fn palette_editor(palette: Mutable<PaletteColor>) -> impl Widget {
    let falloff = palette.clone().map_ref(|v| &v.falloff, |v| &mut v.falloff);

    let color = Arc::new(palette.clone().map_ref(|v| &v.color, |v| &mut v.color));
    let color_rect = color.stream().map(|v| {
        Rectangle::new(ValueOrRef::value(v.into_color()))
            .with_min_size(Unit::px2(100.0, 100.0))
            .with_maximize(Vec2::X)
            // .with_min_size(Unit::new(vec2(0.0, 100.0), vec2(1.0, 0.0)))
            .with_name("ColorPreview")
    });

    let current_mode = Mutable::new(EditorMode::Oklch);

    card(col((
        row((
            Radio::new(
                label("Oklch"),
                current_mode
                    .clone()
                    .map(|v| v.is_oklch(), |_| EditorMode::Oklch),
            ),
            Radio::new(
                label("Rgb"),
                current_mode
                    .clone()
                    .map(|v| v.is_rgb(), |_| EditorMode::Rgb),
            ),
        )),
        StreamWidget(current_mode.stream().map(move |mode| match mode {
            EditorMode::Oklch => Box::new(oklch_editor(palette.clone())) as Box<dyn Widget>,
            EditorMode::Rgb => Box::new(rgb_editor(palette.clone())),
        })),
        ColorHexEditor {
            color: Box::new(color.clone()),
        },
        StreamWidget(color_rect),
        row((
            Text::new("Chroma falloff"),
            SliderWithLabel::new(falloff, 0.0, 100.0)
                .editable(true)
                .round(1.0),
        )),
    )))
    .with_name("PaletteEditor")
}

pub struct ColorHexEditor {
    color: DynStateDuplex<Oklch>,
}

impl Widget for ColorHexEditor {
    fn mount(self, scope: &mut Scope<'_>) {
        let value = self.color.prevent_feedback().filter_map(
            |v| Some(color_hex(v)),
            |v| {
                let v: Srgb<u8> = v.trim().parse().ok()?;

                let v = Oklch::from_color(v.into_format());
                Some(v)
            },
        );

        TextInput::new(value).mount(scope)
    }
}

fn oklch_editor(color: Mutable<PaletteColor>) -> impl Widget {
    let color = Arc::new(color.map_ref(|v| &v.color, |v| &mut v.color));

    let lightness = color.clone().map_ref(|v| &v.l, |v| &mut v.l);
    let chroma = color.clone().map_ref(|v| &v.chroma, |v| &mut v.chroma);
    let hue = color
        .clone()
        .map_ref(|v| &v.hue, |v| &mut v.hue)
        .map(|v| v.into_positive_degrees(), OklabHue::new);

    col((
        row((
            Text::new("Lightness"),
            SliderWithLabel::new(lightness, 0.0, 1.0)
                .editable(true)
                .round(0.01),
        )),
        row((
            Text::new("Chroma"),
            SliderWithLabel::new(chroma, 0.0, 0.37)
                .editable(true)
                .round(0.005),
        )),
        row((
            Text::new("Hue"),
            SliderWithLabel::new(hue, 0.0, 360.0)
                .editable(true)
                .round(1.0),
        )),
    ))
}

pub fn rgb_editor(color: Mutable<PaletteColor>) -> impl Widget {
    let rgb_color = Arc::new(
        color
            .map_ref(|v| &v.color, |v| &mut v.color)
            .map(Rgb::from_color, |v: Rgb| Oklch::from_color(v))
            .memo(Default::default()),
    );

    let r = rgb_color.clone().map_ref(|v| &v.red, |v| &mut v.red);
    let g = rgb_color.clone().map_ref(|v| &v.green, |v| &mut v.green);
    let b = rgb_color.clone().map_ref(|v| &v.blue, |v| &mut v.blue);

    card(col((
        row((
            Text::new("Red"),
            SliderWithLabel::new(r, 0.0, 1.0).editable(true).round(0.01),
        )),
        row((
            Text::new("Green"),
            SliderWithLabel::new(g, 0.0, 1.0).editable(true).round(0.01),
        )),
        row((
            Text::new("Blue"),
            SliderWithLabel::new(b, 0.0, 1.0).editable(true).round(0.01),
        )),
    )))
    .with_name("PaletteEditor")
}
