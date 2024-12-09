use std::{str::FromStr, sync::Arc};

use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{vec3, BVec2, IVec2, Vec2, Vec3};
use itertools::Itertools;
use palette::{
    FromColor, Hsl, Hsv, IntoColor, Oklab, OklabHue, Oklch, RgbHue, Srgb, Srgba, WithAlpha,
};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    state::{State, StateDuplex, StateStream, StateStreamRef},
    style::{colors::*, primary_surface, spacing_medium, spacing_small, SizeExt},
    to_owned,
    unit::Unit,
    widget::{
        card, col, label, panel, row, Rectangle, ScrollArea, SliderValue, SliderWithLabel,
        StreamWidget, TextInput,
    },
    Scope, Widget,
};

pub fn main() -> anyhow::Result<()> {
    registry()
        .with(
            HierarchicalLayer::default()
                .with_deferred_spans(true)
                .with_span_retrace(true)
                .with_indent_lines(true),
        )
        .with(EnvFilter::from_default_env())
        .init();

    violet_wgpu::AppBuilder::new().run(main_app())
}

pub struct Palette {
    colors: Vec<Mutable<Srgb>>,
}

pub struct PaletteCollection {
    palettes: Vec<Palette>,
}

pub fn main_app() -> impl Widget {
    let palettes = Mutable::new(PaletteCollection {
        palettes: vec![
            Palette {
                colors: vec![
                    Mutable::new(EMERALD_50.without_alpha()),
                    Mutable::new(EMERALD_100.without_alpha()),
                    Mutable::new(EMERALD_200.without_alpha()),
                    Mutable::new(EMERALD_300.without_alpha()),
                    Mutable::new(EMERALD_400.without_alpha()),
                    Mutable::new(EMERALD_500.without_alpha()),
                    Mutable::new(EMERALD_600.without_alpha()),
                    Mutable::new(EMERALD_700.without_alpha()),
                    Mutable::new(EMERALD_800.without_alpha()),
                    Mutable::new(EMERALD_900.without_alpha()),
                    Mutable::new(EMERALD_950.without_alpha()),
                ],
            },
            Palette {
                colors: vec![
                    Mutable::new(TEAL_50.without_alpha()),
                    Mutable::new(TEAL_100.without_alpha()),
                    Mutable::new(TEAL_200.without_alpha()),
                    Mutable::new(TEAL_300.without_alpha()),
                    Mutable::new(TEAL_400.without_alpha()),
                    Mutable::new(TEAL_500.without_alpha()),
                    Mutable::new(TEAL_600.without_alpha()),
                    Mutable::new(TEAL_700.without_alpha()),
                    Mutable::new(TEAL_800.without_alpha()),
                    Mutable::new(TEAL_900.without_alpha()),
                    Mutable::new(TEAL_950.without_alpha()),
                ],
            },
            Palette {
                colors: vec![
                    Mutable::new(OCEAN_50.without_alpha()),
                    Mutable::new(OCEAN_100.without_alpha()),
                    Mutable::new(OCEAN_200.without_alpha()),
                    Mutable::new(OCEAN_300.without_alpha()),
                    Mutable::new(OCEAN_400.without_alpha()),
                    Mutable::new(OCEAN_500.without_alpha()),
                    Mutable::new(OCEAN_600.without_alpha()),
                    Mutable::new(OCEAN_700.without_alpha()),
                    Mutable::new(OCEAN_800.without_alpha()),
                    Mutable::new(OCEAN_900.without_alpha()),
                    Mutable::new(OCEAN_950.without_alpha()),
                ],
            },
            Palette {
                colors: vec![
                    Mutable::new(EMERALD_800.without_alpha()),
                    Mutable::new(OCEAN_800.without_alpha()),
                    Mutable::new(TEAL_800.without_alpha()),
                    Mutable::new(REDWOOD_800.without_alpha()),
                    Mutable::new(COPPER_800.without_alpha()),
                ],
            },
        ],
    });

    let current_selection = Mutable::new((0_usize, 0_usize));

    let current_selection = current_selection
        .stream()
        .map({
            to_owned![palettes];
            move |index| palettes.lock_ref().palettes[index.0].colors[index.1].clone()
        })
        .map(swatch_editor);

    let palettes = palettes
        .stream_ref(move |v| {
            let values = v
                .palettes
                .iter()
                .map(move |palette| {
                    card(row(palette
                        .colors
                        .iter()
                        .map(|color| {
                            StreamWidget::new(color.stream().map(|color| {
                                Rectangle::new(color.with_alpha(1.0))
                                    .with_min_size(Unit::px2(60.0, 60.0))
                                    .with_margin(spacing_medium())
                            }))
                        })
                        .collect_vec()))
                })
                .collect_vec();

            col(values)
        })
        .boxed();

    panel(col((
        StreamWidget::new(current_selection),
        ScrollArea::new(BVec2::new(true, true), StreamWidget::new(palettes)),
    )))
    .with_background(primary_surface())
    .with_maximize(Vec2::ONE)
    .with_contain_margins(true)
}

pub fn swatch_editor(rgb_color: Mutable<Srgb>) -> impl Widget {
    let color = Arc::new(
        rgb_color
            .map(ColorValue::Rgb, |v| v.as_rgb())
            .memo(ColorValue::Rgb(Default::default())),
    );

    // let color = Mutable::new(ColorValue::Rgb(EMERALD_500.without_alpha().into_format()));

    let color_swatch = color.clone().stream().map(|v| {
        Rectangle::new(v.as_rgb().into_format().with_alpha(1.0))
            .with_aspect_ratio(1.0)
            .with_min_size(Unit::px2(200.0, 200.0))
            .with_margin(spacing_small())
    });

    panel(row((
        card(col((
            StreamWidget::new(color_swatch),
            color_hex_editor(color.clone()),
        ))),
        col((
            rgb_picker(color.clone()),
            hsl_picker(color.clone()),
            oklab_picker(color),
        )),
    )))
}

const ROUNDING: f32 = 0.01;

pub fn precise_slider<T>(
    value: impl 'static + Send + Sync + StateDuplex<Item = T>,
    min: T,
    max: T,
) -> SliderWithLabel<T>
where
    T: Default + FromStr + ToString + SliderValue,
{
    SliderWithLabel::new(value, min, max).with_scrub_mode(true)
}

fn rgb_picker(color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>) -> impl Widget {
    let color = Arc::new(
        color
            .map(|v| v.as_rgb(), ColorValue::Rgb)
            .map(Srgb::<u8>::from_format, |v| v.into_format())
            .memo(Default::default()),
    );

    let r = precise_slider(color.clone().map_ref(|v| &v.red, |v| &mut v.red), 0, 255);
    let g = precise_slider(
        color.clone().map_ref(|v| &v.green, |v| &mut v.green),
        0,
        255,
    );
    let b = precise_slider(color.clone().map_ref(|v| &v.blue, |v| &mut v.blue), 0, 255);

    card(row((
        col((label("R"), label("G"), label("B"))),
        col((r, g, b)),
    )))
}

fn hsl_picker(color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>) -> impl Widget {
    let color = Arc::new(
        color
            .map(|v| v.as_hsl(), ColorValue::Hsl)
            .memo(Default::default()),
    );

    let hue = color
        .clone()
        .map_ref(|v| &v.hue, |v| &mut v.hue)
        .map(|v| v.into_positive_degrees(), RgbHue::from_degrees)
        .memo(Default::default());

    let h = precise_slider(hue, 0.0, 360.0).round(1.0);
    let s = precise_slider(
        color
            .clone()
            .map_ref(|v| &v.saturation, |v| &mut v.saturation),
        0.0,
        1.0,
    )
    .round(ROUNDING);

    let l = SliderWithLabel::new(
        color
            .clone()
            .map_ref(|v| &v.lightness, |v| &mut v.lightness),
        0.0,
        1.0,
    )
    .round(ROUNDING);

    card(row((
        col((label("H"), label("S"), label("L"))),
        col((h, s, l)),
    )))
}

fn oklab_picker(color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>) -> impl Widget {
    let color = Arc::new(
        color
            .map(|v| v.as_oklab(), ColorValue::OkLab)
            .memo(Default::default()),
    );

    let hue = color
        .clone()
        .map_ref(|v| &v.hue, |v| &mut v.hue)
        .map(|v| v.into_positive_degrees(), OklabHue::from_degrees)
        .memo(Default::default());

    let h = precise_slider(hue, 0.0, 360.0).round(1.0);
    let c = precise_slider(
        color.clone().map_ref(|v| &v.chroma, |v| &mut v.chroma),
        0.0,
        1.0,
    )
    .round(ROUNDING);

    let l = precise_slider(color.clone().map_ref(|v| &v.l, |v| &mut v.l), 0.0, 1.0).round(ROUNDING);

    card(row((
        col((label("L"), label("C"), label("H"))),
        col((l, c, h)),
    )))
}

pub fn color_hex(color: impl IntoColor<Srgb>) -> String {
    let hex: Srgb<u8> = color.into_color().into_format();
    format!("#{:0>2x}{:0>2x}{:0>2x}", hex.red, hex.green, hex.blue)
}

fn color_hex_editor(
    color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>,
) -> impl Widget {
    let color = Arc::new(
        color
            .map(|v| v.as_rgb(), ColorValue::Rgb)
            .memo(Default::default()),
    );

    let value = color.prevent_feedback().filter_map(
        |v| Some(color_hex(v)),
        |v| {
            let v: Srgb<u8> = v.trim().parse().ok()?;
            Some(v.into_format())
        },
    );

    TextInput::new(value)
}

#[derive(Clone, Copy)]
enum ColorValue {
    Rgb(Srgb),
    Hsl(Hsl),
    Hsv(Hsv),
    OkLab(Oklch),
}

impl ColorValue {
    fn as_rgb(&self) -> Srgb {
        match *self {
            ColorValue::Rgb(rgb) => rgb,
            ColorValue::Hsl(hsl) => hsl.into_color(),
            ColorValue::Hsv(hsv) => hsv.into_color(),
            ColorValue::OkLab(lch) => lch.into_color(),
        }
    }

    fn as_hsl(&self) -> Hsl {
        match *self {
            ColorValue::Rgb(rgb) => rgb.into_color(),
            ColorValue::Hsl(hsl) => hsl,
            ColorValue::Hsv(hsv) => hsv.into_color(),
            ColorValue::OkLab(lch) => lch.into_color(),
        }
    }

    fn as_hsv(&self) -> Hsv {
        match *self {
            ColorValue::Rgb(rgb) => rgb.into_color(),
            ColorValue::Hsl(hsl) => hsl.into_color(),
            ColorValue::Hsv(hsv) => hsv,
            ColorValue::OkLab(lch) => lch.into_color(),
        }
    }

    fn as_oklab(&self) -> Oklch {
        match *self {
            ColorValue::Rgb(rgb) => rgb.into_color(),
            ColorValue::Hsl(hsl) => hsl.into_color(),
            ColorValue::Hsv(hsv) => hsv.into_color(),
            ColorValue::OkLab(lch) => lch,
        }
    }
}
