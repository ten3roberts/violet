use std::{
    collections::{hash_map::Entry, HashMap},
    future::ready,
    iter::repeat,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use futures::StreamExt;
use glam::{BVec2, Vec2};
use heck::{ToKebabCase, ToShoutySnakeCase};
use indexmap::IndexMap;
use itertools::Itertools;
use rfd::AsyncFileDialog;
use serde::{Deserialize, Serialize};
use violet::core::{
    io::clipboard,
    layout::Align,
    state::{
        StateDuplex, StateExt, StateMut, StateOwned, StateRef, StateSink, StateStream,
        StateStreamRef,
    },
    style::{
        default_corner_radius, spacing_small, surface_disabled, surface_interactive,
        surface_pressed, surface_primary, Background, SizeExt,
    },
    time::sleep,
    to_owned,
    unit::Unit,
    utils::zip_latest,
    widget::{
        card, col, header, interactive::base::InteractiveWidget, label, panel, row, Button,
        Checkbox, Collapsible, Radio, Rectangle, ScrollArea, Selectable, SignalWidget, SliderStyle,
        SliderValue, SliderWithLabel, Stack, StreamWidget, Text, TextInput,
    },
    FutureEffect, Scope, ScopeRef, Widget,
};
use violet::futures_signals::signal::{Mutable, SignalExt};
use violet::palette::{Hsl, IntoColor, OklabHue, Oklch, RgbHue, Srgb, WithAlpha};

const TINTS: [usize; 11] = [950, 900, 800, 700, 600, 500, 400, 300, 200, 100, 50];

fn tint_name(i: usize, count: usize) -> usize {
    let tint = 5 + ((count) as u32 / 2 - i as u32);
    TINTS[tint as usize]
}

#[derive(Clone, Copy, Serialize, Deserialize)]
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
            enabled: false,
            min_lum: 0.15,
            max_lum: 0.95,
            falloff: 10.0,
        }
    }
}

pub fn auto_tint_settings(settings: Mutable<AutoPaletteSettings>) -> impl Widget {
    let settings_widget = col((
        Checkbox::label(
            "Enabled",
            settings.clone().map_ref(|v| &v.enabled, |v| &mut v.enabled),
        ),
        row((
            col((
                header("Min Lightness"),
                header("Max Lightness"),
                header("Chroma Falloff"),
            )),
            col((
                precise_slider(
                    settings.clone().transform(
                        |v| v.min_lum,
                        |settings, v| {
                            settings.min_lum = v;
                            settings.max_lum = settings.max_lum.max(v);
                        },
                    ),
                    0.0,
                    1.0,
                )
                .precision(ROUNDING),
                precise_slider(
                    settings.clone().transform(
                        |v| v.max_lum,
                        |settings, v| {
                            settings.max_lum = v;
                            settings.min_lum = settings.min_lum.min(v);
                        },
                    ),
                    0.0,
                    1.0,
                )
                .precision(ROUNDING),
                precise_slider(
                    settings.clone().map_ref(|v| &v.falloff, |v| &mut v.falloff),
                    0.0,
                    30.0,
                )
                .precision(ROUNDING),
            )),
        )),
    ));

    let enabled_indicator = InteractiveWidget::new(SignalWidget::new(
        settings
            .signal()
            .map(|v| v.enabled)
            .dedupe()
            .map(|enabled| {
                Rectangle::new(if enabled {
                    surface_pressed()
                } else {
                    surface_disabled()
                })
                .with_exact_size(Unit::px2(18.0, 18.0))
                .with_corner_radius(Unit::rel(1.0))
            }),
    ))
    .on_click(move |_| {
        let enabled = &mut settings.lock_mut().enabled;
        *enabled = !*enabled;
    });

    card(
        Collapsible::new(
            row((label("Configure auto shades"), enabled_indicator))
                .with_cross_align(Align::Center),
            settings_widget,
        )
        .with_size(Unit::px2(950.0, 0.0)),
    )
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    #[serde(default)]
    name: Mutable<String>,
    colors: Vec<Mutable<ColorValue>>,
    auto: Mutable<AutoPaletteSettings>,
}

impl Palette {}

fn palette_controls(
    palettes: impl 'static + Send + Sync + StateMut<Item = Palette>,
    palette_index: usize,
    palette: &Palette,
    selection: impl 'static + Send + Sync + StateDuplex<Item = (usize, usize)> + StateOwned,
    remove_self: impl 'static + Send + Sync + Fn(&ScopeRef<'_>),
) -> impl Widget {
    let palettes = Arc::new(palettes);
    let selection = Arc::new(selection);

    let add_swatch = Button::label("+").on_click({
        to_owned!(palettes, selection);
        move |_| {
            palettes.write_mut(|palette| {
                let last = palette.colors.last().map(|v| v.get()).unwrap_or_default();
                palette.colors.push(Mutable::new(last));

                selection.send((palette_index, palette.colors.len() - 1))
            });
        }
    });

    let external_settings_change = palette.auto.stream().for_each({
        to_owned!(palettes, selection);
        move |auto| {
            palettes.read_ref(|palette| {
                if palette.colors.is_empty() {
                    return;
                }

                let selection = selection.read();
                let ref_color = if selection.0 == palette_index {
                    selection.1
                } else {
                    palette.colors.len() / 2
                };

                let count = palette.colors.len();
                if auto.enabled {
                    let base_tint = ref_color as f32 / (count - 1) as f32;
                    let new_color = palette.colors[ref_color].get().as_oklab();
                    let base_chroma =
                        new_color.chroma * (1.0 + auto.falloff * (base_tint - 0.5).powi(2));

                    update_palette_tints(palette, auto, base_chroma, new_color, count);
                }
            });

            async move {}
        }
    });

    let widget = card(
        row((
            row(palette
                .colors
                .iter()
                .enumerate()
                .map(move |(i, color)| {
                    to_owned!(color, palettes, selection);

                    let current_selection = selection.stream().map(move |v| {
                        let palettes = palettes.clone();
                        let selection = selection.clone();

                        let is_selected = (palette_index, i) == v;

                        InteractiveWidget::new(
                            Stack::new((
                                StreamWidget::new(color.stream().map(|color| {
                                    Rectangle::new(color.as_rgb().with_alpha(1.0))
                                        .with_min_size(Unit::px2(60.0, 60.0))
                                        .with_corner_radius(default_corner_radius())
                                })),
                                Button::label("-")
                                    .with_padding(spacing_small())
                                    .with_margin(spacing_small())
                                    .on_click(move |_| {
                                        palettes.write_mut(|v| v.colors.remove(i));
                                    }),
                            ))
                            .with_horizontal_alignment(Align::End)
                            .with_background_opt(if is_selected {
                                Some(Background::new(surface_interactive()))
                            } else {
                                None
                            })
                            .with_corner_radius(default_corner_radius())
                            .with_padding(spacing_small()),
                        )
                        .on_click(move |_| {
                            selection.send((palette_index, i));
                        })
                    });

                    StreamWidget::new(current_selection)
                })
                .collect_vec()),
            col((add_swatch, Button::label("-").on_click(remove_self))),
            TextInput::new(palette.name.clone()),
        ))
        .with_min_size(Unit::px2(950.0, 0.0)),
    );

    move |scope: &mut Scope| {
        scope.spawn(external_settings_change);
        widget.mount(scope)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PaletteCollection {
    palettes: Vec<Palette>,
}

pub fn main_app() -> impl Widget {
    let palettes = Mutable::new(PaletteCollection {
        palettes: vec![create_palette(0, 11)],
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
                        let remove_row = {
                            to_owned!(palettes);
                            move |_: &ScopeRef| {
                                let mut palettes = palettes.lock_mut();

                                palettes.palettes.remove(i);
                            }
                        };

                        to_owned!(palettes, current_selection);

                        row((palette_controls(
                            palettes.map_ref(move |v| &v.palettes[i], move |v| &mut v.palettes[i]),
                            i,
                            palette,
                            current_selection,
                            remove_row,
                        ),))
                    })
                    .collect_vec();

                let normalize = Button::label("Normalize").on_click({
                    to_owned!(palettes);
                    let normalize_color_count = 11;
                    move |_| {
                        palettes.lock_mut().palettes.iter_mut().for_each(|palette| {
                            let ref_color = palette.colors.len() / 2;

                            let mut auto = palette.auto.lock_mut();
                            let base_tint = ref_color as f32 / (palette.colors.len() - 1) as f32;
                            let base_color = palette.colors[ref_color].get().as_oklab();
                            let base_chroma = base_color.chroma
                                * (1.0 + auto.falloff * (base_tint - 0.5).powi(2));

                            *auto = AutoPaletteSettings {
                                enabled: true,
                                ..Default::default()
                            };

                            if palette.colors.len() < normalize_color_count {
                                palette.colors.extend(
                                    repeat(ColorValue::OkLab(base_color))
                                        .map(Mutable::new)
                                        .take(normalize_color_count - palette.colors.len()),
                                );
                            }

                            if auto.enabled {
                                update_palette_tints(
                                    palette,
                                    *auto,
                                    base_chroma,
                                    base_color,
                                    palette.colors.len(),
                                );
                            }
                        });
                    }
                });

                let sort = Button::label("Sort").on_click({
                    to_owned!(palettes);
                    move |_| {
                        palettes.lock_mut().palettes.sort_by_key(|palette| {
                            let ref_color = palette.colors.len() / 2;
                            let color = palette.colors[ref_color].get().as_oklab();

                            let a = (color.chroma * 100.0) as u32;
                            let b = color.hue.into_positive_degrees() as u32;

                            (a, b)
                        })
                    }
                });

                let add_row = Button::label("+").on_click({
                    to_owned!(palettes, current_selection);
                    move |_| {
                        let mut palettes = palettes.lock_mut();

                        let num_colors = palettes
                            .palettes
                            .last()
                            .map(|v| v.colors.len())
                            .unwrap_or(11);

                        let index = palettes.palettes.len();

                        palettes.palettes.push(create_palette(index, num_colors));

                        current_selection.set((index, 0));
                    }
                });

                col((col(values), row((add_row, normalize, sort))))
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
    .map({
        to_owned!(palettes);
        move |(palette_index, auto, color)| {
            to_owned!(palettes, auto);

            let auto2 = auto.clone();
            let color_setter = color.map_value(
                |v| v,
                move |new_color| {
                    let auto = auto2.get();
                    if !auto.enabled {
                        return new_color;
                    }

                    let palette = &palettes.lock_ref().palettes[palette_index.0];

                    let count = palette.colors.len();
                    let base_tint = palette_index.1 as f32 / (count - 1) as f32;
                    let new_color = new_color.as_oklab();
                    let base_chroma =
                        new_color.chroma * (1.0 + auto.falloff * (base_tint - 0.5).powi(2));

                    update_palette_tints(palette, auto, base_chroma, new_color, count);

                    ColorValue::OkLab(auto.tint(base_chroma, new_color, base_tint))
                },
            );

            col((swatch_editor(color_setter), auto_tint_settings(auto)))
        }
    });

    panel(
        col((
            export_controls(palettes),
            StreamWidget::new(current_selection),
            ScrollArea::new(BVec2::new(true, true), StreamWidget::new(palettes_widget)),
        ))
        .with_contain_margins(true),
    )
    .with_background(surface_primary())
    .with_maximize(Vec2::ONE)
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
            i as f32 / (count - 1) as f32,
        )));
    }
}

fn create_palette(index: usize, num_colors: usize) -> Palette {
    let color = Oklch::new(0.5, 0.27, index as f32 * 60.0).into_color();

    Palette {
        colors: repeat(color)
            .enumerate()
            .map(|(i, v)| {
                ColorValue::OkLab(AutoPaletteSettings::default().tint(
                    0.27,
                    v,
                    i as f32 / (num_colors - 1) as f32,
                ))
            })
            .map(Mutable::new)
            .take(num_colors)
            .collect_vec(),
        auto: Default::default(),
        name: Mutable::new("Unnamed Color".into()),
    }
}

fn swatch_editor(
    color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>,
) -> impl Widget {
    let color = Arc::new(color);

    let color_swatch = color.clone().stream().map(|v| {
        Rectangle::new(v.as_rgb().into_format().with_alpha(1.0))
            .with_aspect_ratio(1.0)
            .with_min_size(Unit::px2(300.0, 300.0))
            .with_margin(spacing_small())
            .with_corner_radius(default_corner_radius())
    });

    card(
        row((
            card(col((
                StreamWidget::new(color_swatch),
                color_hex_editor(color.clone()),
            ))),
            col((
                rgb_picker(color.clone()),
                hsl_picker(color.clone()),
                oklab_picker(color),
            )),
        ))
        .with_min_size(Unit::px2(950.0, 0.0)),
    )
}

const ROUNDING: u32 = 4;

pub fn precise_slider<T>(
    value: impl 'static + Send + Sync + StateDuplex<Item = T>,
    min: T,
    max: T,
) -> SliderWithLabel<T>
where
    T: Default + FromStr + ToString + SliderValue,
{
    SliderWithLabel::new(value, min, max)
        .with_style(SliderStyle {
            track_size: Unit::px2(480.0, 4.0),
            ..Default::default()
        })
        .with_scrub_mode(true)
        .editable(true)
}

fn rgb_picker(color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>) -> impl Widget {
    let color = Arc::new(
        color
            .map_value(|v| v.as_rgb(), ColorValue::Rgb)
            .map_value(Srgb::<u8>::from_format, |v| v.into_format())
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
            .map_value(|v| v.as_hsl(), ColorValue::Hsl)
            .memo(Default::default()),
    );

    let hue = color
        .clone()
        .map_ref(|v| &v.hue, |v| &mut v.hue)
        .map_value(|v| v.into_positive_degrees(), RgbHue::from_degrees)
        .memo(Default::default());

    let h = precise_slider(hue, 0.0, 360.0).precision(1);
    let s = precise_slider(
        color
            .clone()
            .map_ref(|v| &v.saturation, |v| &mut v.saturation),
        0.0,
        1.0,
    )
    .precision(ROUNDING);

    let l = precise_slider(
        color
            .clone()
            .map_ref(|v| &v.lightness, |v| &mut v.lightness),
        0.0,
        1.0,
    )
    .precision(ROUNDING);

    card(row((
        col((header("H"), header("S"), header("L"))),
        col((h, s, l)),
    )))
}

fn oklab_picker(color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>) -> impl Widget {
    let color = Arc::new(
        color
            .map_value(|v| v.as_oklab(), ColorValue::OkLab)
            .memo(Default::default()),
    );

    let hue = color
        .clone()
        .map_ref(|v| &v.hue, |v| &mut v.hue)
        .map_value(|v| v.into_positive_degrees(), OklabHue::from_degrees)
        .memo(Default::default());

    let h = precise_slider(hue, 0.0, 360.0).precision(1);
    let c = precise_slider(
        color.clone().map_ref(|v| &v.chroma, |v| &mut v.chroma),
        0.0,
        0.37,
    )
    .precision(3);

    let l =
        precise_slider(color.clone().map_ref(|v| &v.l, |v| &mut v.l), 0.0, 1.0).precision(ROUNDING);

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
            .map_value(|v| v.as_rgb(), ColorValue::Rgb)
            .memo(Default::default()),
    );

    let value = color.prevent_feedback().filter_map(
        |v| Some(color_hex(v)),
        |v| {
            let v = v.trim();

            if !v.starts_with("#") || v.len() != 7 {
                return None;
            }

            let v: Srgb<u8> = v.parse().ok()?;
            Some(v.into_format())
        },
    );

    TextInput::new(value)
}

#[derive(Clone, Copy, Serialize, Deserialize)]
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

fn local_dir() -> std::path::PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::current_dir().unwrap()
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::path::PathBuf::from(".")
    }
}

struct TimedWidget<W> {
    widget: W,
    lifetime: Duration,
}

impl<W: Widget> Widget for TimedWidget<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        let id = scope.attach(self.widget);

        scope.spawn_effect(FutureEffect::new(
            sleep(self.lifetime),
            move |scope: &mut Scope, _| {
                scope.detach(id);
            },
        ));
    }
}

impl<W> TimedWidget<W> {
    fn new(widget: W, lifetime: Duration) -> Self {
        Self { widget, lifetime }
    }
}

pub fn export_controls(palettes: Mutable<PaletteCollection>) -> impl Widget {
    let (result_tx, result_rx) = flume::unbounded();

    fn set_result(result: &flume::Sender<TimedWidget<Text>>, text: impl Into<String>) {
        result
            .send(TimedWidget::new(label(text), Duration::from_secs(5)))
            .unwrap();
    }

    let save = {
        to_owned!(palettes, result_tx);
        move |scope: &ScopeRef| {
            to_owned!(result_tx);
            let data = serde_json::to_string_pretty(&palettes).unwrap();

            let fut = async move {
                let Some(file) = AsyncFileDialog::new()
                    .set_directory(local_dir())
                    .set_file_name("colors.json")
                    .save_file()
                    .await
                else {
                    anyhow::bail!("No file specified");
                };

                file.write(data.as_bytes())
                    .await
                    .context("Failed to write to save file")?;

                Ok(())
            };

            scope.spawn(async move {
                match fut.await {
                    Ok(_) => set_result(&result_tx, "Saved palettes"),
                    Err(e) => set_result(&result_tx, format!("{e:?}")),
                }
            });
        }
    };

    let load = {
        to_owned!(palettes, result_tx);
        move |scope: &ScopeRef| {
            to_owned!(palettes, result_tx);
            let fut = async move {
                let Some(file) = AsyncFileDialog::new()
                    .set_directory(local_dir())
                    .set_file_name("colors.json")
                    .pick_file()
                    .await
                else {
                    anyhow::bail!("No file specified");
                };

                let data = file.read().await;

                let data: PaletteCollection =
                    serde_json::from_slice(&data).context("Failed to deserialize state")?;

                let count = data.palettes.len();
                palettes.set(data);

                anyhow::Ok(count)
            };

            scope.spawn(async move {
                match fut.await {
                    Ok(count) => set_result(&result_tx, format!("Loaded {count} palettes")),
                    Err(e) => set_result(&result_tx, format!("{e:?}")),
                }
            });
        }
    };

    #[derive(Copy, Clone, PartialEq, PartialOrd)]
    enum ExportFormat {
        Tailwind,
        Rust,
    }

    let formatter = |palettes: &PaletteCollection, format: ExportFormat| match format {
        ExportFormat::Tailwind => serde_json::to_string_pretty(&palettes.export()).unwrap(),
        ExportFormat::Rust => export_hex_list(palettes),
    };

    let export_format = Mutable::new(ExportFormat::Tailwind);

    let export = {
        to_owned!(palettes, result_tx, export_format);
        move |scope: &ScopeRef| {
            to_owned!(result_tx);
            let data = formatter(&palettes.lock_ref(), export_format.get());
            let fut = async move {
                let Some(file) = AsyncFileDialog::new()
                    .set_directory(local_dir())
                    .set_file_name("color_palette.json")
                    .save_file()
                    .await
                else {
                    return anyhow::Ok(());
                };

                file.write(data.as_bytes())
                    .await
                    .context("Failed to write to file")?;

                Ok(())
            };

            scope.spawn(async move {
                match fut.await {
                    Ok(_) => set_result(&result_tx, "Exported palettes"),
                    Err(e) => set_result(&result_tx, format!("{e:?}")),
                }
            });
        }
    };

    let export_clipboard = {
        to_owned!(result_tx, export_format);
        move |scope: &ScopeRef<'_>| {
            let data = formatter(&palettes.lock_ref(), export_format.get());

            let clipboard = scope
                .get_atom(clipboard())
                .expect("Clipboard not available");

            let clipboard = scope.frame().store().get(&clipboard).clone();

            scope.spawn(async move { clipboard.set_text(data).await });
            set_result(&result_tx, "Copied palettes to clipboard");
        }
    };

    row((
        Button::label("Save").on_click(save),
        Button::label("Load").on_click(load),
        Button::label("Export").on_click(export),
        Button::label("Export To Clipboard").on_click(export_clipboard),
        Selectable::new_value(label("Json"), export_format.clone(), ExportFormat::Tailwind),
        Selectable::new_value(label("Rust"), export_format.clone(), ExportFormat::Rust),
        StreamWidget::new(result_rx.into_stream()),
    ))
    .with_cross_align(Align::Center)
}

pub fn export_hex_list(palettes: &PaletteCollection) -> String {
    let mut used: HashMap<_, usize> = HashMap::new();

    palettes
        .palettes
        .iter()
        .flat_map(|v| {
            let mut name = v.name.get_cloned().to_shouty_snake_case();
            match used.entry(name.clone()) {
                Entry::Occupied(mut slot) => {
                    let suffix = slot.get_mut();
                    *suffix += 1;
                    name = format!("{name}_{suffix}")
                }
                Entry::Vacant(slot) => {
                    slot.insert(0);
                }
            };

            const TINTS: [usize; 11] = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];
            v.colors.iter().enumerate().map(move |(i, color)| {
                let color = color.get();
                let tint = if v.colors.len() <= TINTS.len() {
                    tint_name(i, v.colors.len())
                } else {
                    i
                };

                let hex = color_hex(color.as_rgb());

                format!("pub const {name}_{tint}: Srgba = srgba!(\"{hex}\");")
            })
        })
        .join("\n")
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct PalettesExport {
    palettes: IndexMap<String, Vec<String>>,
}

impl PaletteCollection {
    pub fn export(&self) -> PalettesExport {
        let mut used: HashMap<_, usize> = HashMap::new();
        PalettesExport {
            palettes: self
                .palettes
                .iter()
                .map(|v| {
                    let mut name = v.name.get_cloned().to_kebab_case();
                    match used.entry(name.clone()) {
                        Entry::Occupied(mut slot) => {
                            let suffix = slot.get_mut();
                            *suffix += 1;
                            name = format!("{name}_{suffix}")
                        }
                        Entry::Vacant(slot) => {
                            slot.insert(0);
                        }
                    };

                    (
                        name,
                        v.colors
                            .iter()
                            .map(|v| color_hex(v.get().as_rgb()))
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}
