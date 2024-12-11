use std::{future::ready, iter::repeat, str::FromStr, sync::Arc};

use futures::StreamExt;
use futures_signals::signal::{Mutable, SignalExt};
use glam::{BVec2, Vec2};
use itertools::Itertools;
use palette::{Hsl, IntoColor, OklabHue, Oklch, RgbHue, Srgb, WithAlpha};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    layout::Align,
    state::{State, StateDuplex, StateMut, StateRef, StateSink, StateStream, StateStreamRef},
    style::{
        interactive_inactive, primary_surface, spacing_medium, spacing_small, Background, SizeExt,
    },
    to_owned,
    unit::Unit,
    utils::zip_latest,
    widget::{
        card, col, header, panel, row, Button, Checkbox, InteractiveExt, Rectangle, ScrollArea,
        SliderValue, SliderWithLabel, Stack, StreamWidget, TextInput,
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

#[derive(Copy, Clone)]
pub struct AutoPaletteSettings {
    enabled: bool,
    min_lum: f32,
    max_lum: f32,
    falloff: f32,
}

impl AutoPaletteSettings {
    pub fn tint(&self, base_chroma: f32, color: Oklch, tint: f32) -> Oklch {
        let chroma = base_chroma * (1.0 / (1.0 + self.falloff * (tint - 0.5).powi(2)));

        Oklch {
            chroma,
            l: (self.max_lum - self.min_lum) * (1.0 - tint) + self.min_lum,
            ..color
        }
    }
}

impl Default for AutoPaletteSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            min_lum: 0.2,
            max_lum: 0.8,
            falloff: 15.0,
        }
    }
}

pub fn auto_palette_settings(settings: Mutable<AutoPaletteSettings>) -> impl Widget {
    card(row((
        Checkbox::label(
            "Auto Tints",
            settings.clone().map_ref(|v| &v.enabled, |v| &mut v.enabled),
        ),
        StreamWidget::new(settings.signal_ref(|v| v.enabled).dedupe().to_stream().map(
            move |enabled| {
                if enabled {
                    Some(row((
                        col((header("Min L"), header("Max L"), header("Falloff"))),
                        col((
                            precise_slider(
                                settings.clone().map_ref(|v| &v.min_lum, |v| &mut v.min_lum),
                                0.0,
                                1.0,
                            )
                            .round(ROUNDING),
                            precise_slider(
                                settings.clone().map_ref(|v| &v.max_lum, |v| &mut v.max_lum),
                                0.0,
                                1.0,
                            )
                            .round(ROUNDING),
                            precise_slider(
                                settings.clone().map_ref(|v| &v.falloff, |v| &mut v.falloff),
                                0.0,
                                30.0,
                            )
                            .round(ROUNDING),
                        )),
                    )))
                } else {
                    None
                }
            },
        )),
    )))
}

pub struct Palette {
    colors: Vec<Mutable<ColorValue>>,
    auto: Mutable<AutoPaletteSettings>,
}

impl Palette {}

fn palette_controls(
    palettes: impl 'static + Send + Sync + StateMut<Item = Palette>,
    palette_index: usize,
    palette: &Palette,
    set_selection: impl 'static + Send + Sync + StateDuplex<Item = (usize, usize)>,
) -> impl Widget {
    let palettes = Arc::new(palettes);
    let set_selection = Arc::new(set_selection);

    let add_swatch = Button::label("+").on_press({
        to_owned!(palettes, set_selection);
        move |_, _| {
            palettes.write_mut(|palette| {
                let last = palette.colors.last().map(|v| v.get()).unwrap_or_default();
                palette.colors.push(Mutable::new(last));

                set_selection.send((palette_index, palette.colors.len() - 1))
            });
        }
    });

    let external_settings_change = palette.auto.stream().for_each({
        to_owned!(palettes);
        move |auto| {
            palettes.read_ref(|palette| {
                if palette.colors.is_empty() {
                    return;
                }

                let ref_color = palette.colors.len() / 2;

                let count = palette.colors.len();
                let base_tint = ref_color as f32 / count as f32;
                let new_color = palette.colors[ref_color].get().as_oklab();
                let base_chroma =
                    new_color.chroma * (1.0 + auto.falloff * (base_tint - 0.5).powi(2));

                update_palette_tints(palette, auto, base_chroma, new_color, count);
            });

            async move {}
        }
    });

    let widget = card(row((
        row(palette
            .colors
            .iter()
            .enumerate()
            .map(move |(i, color)| {
                // let palettes = palettes.clone();
                // let set_selection = set_selection.clone();
                to_owned!(color, palettes, set_selection);

                let current_selection = set_selection.stream().map(move |v| {
                    let palettes = palettes.clone();
                    let set_selection = set_selection.clone();

                    let is_selected = (palette_index, i) == v;

                    Stack::new((
                        StreamWidget::new(color.stream().map(|color| {
                            Rectangle::new(color.as_rgb().with_alpha(1.0))
                                .with_min_size(Unit::px2(60.0, 60.0))
                                .with_margin(spacing_medium())
                        })),
                        Button::label("-")
                            .with_padding(spacing_small())
                            .on_press(move |_, _| {
                                palettes.write_mut(|v| v.colors.remove(i));
                            }),
                    ))
                    .with_horizontal_alignment(Align::End)
                    .with_background_opt(if is_selected {
                        Some(Background::new(interactive_inactive()))
                    } else {
                        None
                    })
                    .with_padding(spacing_small())
                    .on_press(move |_| set_selection.send((palette_index, i)))
                });

                StreamWidget::new(current_selection)
            })
            .collect_vec()),
        add_swatch,
    )));

    move |scope: &mut Scope| {
        scope.spawn(external_settings_change);
        widget.mount(scope)
    }
}

pub struct PaletteCollection {
    palettes: Vec<Palette>,
}

pub fn main_app() -> impl Widget {
    let palettes = Mutable::new(PaletteCollection {
        palettes: vec![create_palette(0)],
    });

    let current_selection = Mutable::new((0_usize, 0_usize));

    let palettes_widget = palettes
        .clone()
        .stream_ref({
            to_owned!(palettes, current_selection);
            move |v| {
                let values = v
                    .palettes
                    .iter()
                    .enumerate()
                    .map(|(i, palette)| {
                        to_owned!(palettes, current_selection);

                        row((palette_controls(
                            palettes.map_ref(move |v| &v.palettes[i], move |v| &mut v.palettes[i]),
                            i,
                            palette,
                            current_selection,
                        ),))
                    })
                    .collect_vec();

                let add_row = Button::label("+").on_press({
                    to_owned!(palettes, current_selection);
                    move |_, _| {
                        let mut palettes = palettes.lock_mut();

                        let index = palettes.palettes.len();

                        palettes.palettes.push(create_palette(index));

                        current_selection.set((index, 0));
                    }
                });

                col((col(values), add_row))
            }
        })
        .boxed();

    let current_selection = zip_latest(
        current_selection.stream(),
        palettes.signal_ref(|_| {}).to_stream(),
    )
    .map({
        to_owned![palettes];
        move |((i, j), _)| {
            let palettes = palettes.lock_ref();
            let palette = palettes.palettes.get(i)?;
            Some(((i, j), palette.auto.clone(), palette.colors.get(j)?.clone()))
        }
    })
    .filter_map(ready)
    .map(move |(palette_index, auto, color)| {
        to_owned!(palettes, auto);

        let auto2 = auto.clone();
        let color_setter = color.map(
            |v| v,
            move |new_color| {
                let auto = auto2.get();
                if !auto.enabled {
                    return new_color;
                }

                let palette = &palettes.lock_ref().palettes[palette_index.0];

                let count = palette.colors.len();
                let base_tint = palette_index.1 as f32 / count as f32;
                let new_color = new_color.as_oklab();
                let base_chroma =
                    new_color.chroma * (1.0 + auto.falloff * (base_tint - 0.5).powi(2));

                update_palette_tints(palette, auto, base_chroma, new_color, count);

                ColorValue::OkLab(auto.tint(base_chroma, new_color, base_tint))
            },
        );

        row((swatch_editor(color_setter), auto_palette_settings(auto)))
    });

    panel(col((
        StreamWidget::new(current_selection),
        ScrollArea::new(BVec2::new(true, true), StreamWidget::new(palettes_widget)),
    )))
    .with_background(primary_surface())
    .with_maximize(Vec2::ONE)
    .with_contain_margins(true)
}

fn update_palette_tints(
    palette: &Palette,
    auto: AutoPaletteSettings,
    base_chroma: f32,
    new_color: Oklch,
    count: usize,
) {
    for (i, color) in palette.colors.iter().enumerate() {
        color.set(ColorValue::OkLab(auto.tint(
            base_chroma,
            new_color,
            i as f32 / count as f32,
        )));
    }
}

fn create_palette(index: usize) -> Palette {
    let color = Oklch::new(0.5, 0.27, index as f32 * 60.0).into_color();

    let num_colors = 8;
    Palette {
        colors: repeat(color)
            .enumerate()
            .map(|(i, v)| {
                ColorValue::OkLab(AutoPaletteSettings::default().tint(
                    0.27,
                    v,
                    i as f32 / num_colors as f32,
                ))
            })
            .map(Mutable::new)
            .take(num_colors)
            .collect_vec(),
        auto: Default::default(),
    }
}

fn swatch_editor(
    color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>,
) -> impl Widget {
    let color = Arc::new(color);

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
    SliderWithLabel::new(value, min, max)
        .with_scrub_mode(true)
        .editable(true)
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
        col((header("R"), header("G"), header("B"))),
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
    .round(ROUNDING)
    .editable(true);

    card(row((
        col((header("H"), header("S"), header("L"))),
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
        0.37,
    )
    .round(0.001);

    let l = precise_slider(color.clone().map_ref(|v| &v.l, |v| &mut v.l), 0.0, 1.0).round(ROUNDING);

    card(row((
        col((header("L"), header("C"), header("H"))),
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
    OkLab(Oklch),
}

impl Default for ColorValue {
    fn default() -> Self {
        Self::Rgb(Default::default())
    }
}

impl ColorValue {
    fn as_rgb(&self) -> Srgb {
        match *self {
            ColorValue::Rgb(rgb) => rgb,
            ColorValue::Hsl(hsl) => hsl.into_color(),
            ColorValue::OkLab(lch) => lch.into_color(),
        }
    }

    fn as_hsl(&self) -> Hsl {
        match *self {
            ColorValue::Rgb(rgb) => rgb.into_color(),
            ColorValue::Hsl(hsl) => hsl,
            ColorValue::OkLab(lch) => lch.into_color(),
        }
    }

    fn as_oklab(&self) -> Oklch {
        match *self {
            ColorValue::Rgb(rgb) => rgb.into_color(),
            ColorValue::Hsl(hsl) => hsl.into_color(),
            ColorValue::OkLab(lch) => lch,
        }
    }
}
