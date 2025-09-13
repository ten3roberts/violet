use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use futures::{Stream, StreamExt};
use glam::Vec2;
use heck::{ToKebabCase, ToShoutySnakeCase};
use indexmap::IndexMap;
use itertools::Itertools;
use rfd::{AsyncFileDialog, FileHandle};
use serde::{Deserialize, Serialize};
use violet::{
    core::{
        components::{self},
        io::clipboard,
        state::{StateDuplex, StateExt, StateOwned, StateStream, StateStreamRef},
        style::{default_corner_radius, spacing_small, surface_primary, SizeExt, StyleExt},
        time::sleep,
        to_owned,
        unit::Unit,
        utils::zip_latest,
        widget::{
            card, col, header, interactive::tooltip::Tooltip, label, panel, raised_card, row,
            Button, ButtonStyle, Collapsible, LabeledSlider, Rectangle, ScrollArea, Selectable,
            SliderStyle, SliderValue, StreamWidget, Text, TextInput,
        },
        Edges, FutureEffect, Scope, ScopeRef, Widget,
    },
    lucide::icons::{
        LUCIDE_ARROW_DOWN_NARROW_WIDE, LUCIDE_COPY, LUCIDE_DOWNLOAD, LUCIDE_FOLDER_OPEN,
        LUCIDE_PALETTE, LUCIDE_SAVE, LUCIDE_SUN_MEDIUM, LUCIDE_TRASH, LUCIDE_TRASH_2,
    },
    palette::Srgba,
};
use violet::{
    core::{style::spacing_medium, widget::interactive::dropdown::Dropdown},
    palette::{Hsl, IntoColor, OklabHue, Oklch, RgbHue, Srgb, WithAlpha},
};
use violet::{
    futures_signals::signal::{Mutable, SignalExt},
    lucide::icons::LUCIDE_PLUS,
};

const TINTS: [usize; 11] = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ShadeSettings {
    enabled: bool,
    min_lum: f32,
    max_lum: f32,
    falloff: f32,
}

impl ShadeSettings {
    pub fn tint(&self, base_chroma: f32, color: Oklch, tint: f32) -> Oklch {
        let chroma = base_chroma * (1.0 / (1.0 + self.falloff * (tint - 0.5).powi(2)));

        Oklch {
            chroma,
            l: (self.max_lum - self.min_lum) * (1.0 - tint) + self.min_lum,
            ..color
        }
    }

    pub fn tint_from_base(&self, base: Oklch, tint: f32) -> Oklch {
        let base_chroma = base.chroma;

        self.tint(base_chroma, base, tint)
    }
}

impl Default for ShadeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            min_lum: 0.12,
            max_lum: 0.97,
            falloff: 10.0,
        }
    }
}

pub fn shade_settings(settings: Mutable<ShadeSettings>) -> impl Widget {
    let settings_widget = row((
        col((
            Tooltip::label(header("Dark"), "Control how dark the darkest shade can be."),
            Tooltip::label(
                header("Bright"),
                "Control how light the lightest shade can be.",
            ),
            Tooltip::label(
                header("Chroma Falloff"),
                "Control how much chroma falls off towards lighter and darker shades.",
            ),
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
            .precision(2),
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
            .precision(2),
            precise_slider(
                settings
                    .clone()
                    .project_ref(|v| &v.falloff, |v| &mut v.falloff),
                0.0,
                30.0,
            )
            .precision(1),
        )),
    ));

    card(Collapsible::new(
        icon_header(LUCIDE_SUN_MEDIUM, "Configure Shades"),
        settings_widget,
    ))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    #[serde(default)]
    name: Mutable<String>,
    base_color: Mutable<ColorValue>,
}

impl Palette {
    pub fn all_shades<'a>(
        &'a self,
        shade_settings: &'a ShadeSettings,
    ) -> impl Iterator<Item = Oklch> + 'a {
        let base_color = self.base_color.get().as_oklab();

        (0..SHADE_COUNT).map(move |i| {
            shade_settings.tint_from_base(base_color, i as f32 / (SHADE_COUNT - 1) as f32)
        })
    }

    pub fn widget(
        &self,
        palette_index: usize,
        palettes: Mutable<PaletteCollection>,
        shade_settings: Mutable<ShadeSettings>,
        selection: impl 'static + Send + Sync + StateDuplex<Item = usize> + StateOwned,
    ) -> impl 'static + Widget {
        let selection = Arc::new(selection);

        let shades = (0..SHADE_COUNT)
            .map(|i| {
                let tint = i as f32 / (SHADE_COUNT - 1) as f32;

                let color = zip_latest(self.base_color.stream(), shade_settings.stream()).map({
                    move |(base_color, shade_settings)| {
                        let base_color = base_color.as_oklab();
                        let color = shade_settings.tint_from_base(base_color, tint);
                        let color: Srgba = color.into_color();
                        color
                    }
                });

                color_swatch(color, 60.0)
            })
            .collect_vec();

        let color_info = self.base_color.stream().map(|color| {
            let oklch = color.as_oklab();
            let rgb = color.as_rgb();
            let hex = label(color_hex(rgb));
            let lch = label(format!(
                "L {:.3} C {:.3} H {:.2}",
                oklch.l,
                oklch.chroma,
                oklch.hue.into_positive_degrees()
            ));

            row((raised_card(hex), raised_card(lch)))
        });

        let widget = card(
            Selectable::new_value(
                row((
                    col((
                        color_swatch(
                            self.base_color
                                .stream()
                                .map(|color| color.as_rgb().with_alpha(1.0)),
                            80.0,
                        ),
                        TextInput::new(self.name.clone()),
                    ))
                    .center(),
                    col((row(shades), StreamWidget::new(color_info))),
                    Button::label(LUCIDE_TRASH_2)
                        .with_tooltip_text("Remove Palette")
                        .on_click(move |_| {
                            palettes.lock_mut().palettes.remove(palette_index);
                        }),
                )),
                selection,
                palette_index,
            )
            .with_padding(spacing_medium())
            .with_margin(spacing_medium())
            .with_style(ButtonStyle::muted()),
        )
        // Selectable has the padding. This makes the selectable appear as the entire card
        .with_padding(Edges::ZERO);

        widget
    }
}

const SHADE_COUNT: usize = 11;

fn color_swatch(color: impl 'static + Stream<Item = Srgba>, size: f32) -> impl Widget {
    move |scope: &mut Scope<'_>| {
        let latest_color = scope.store(Srgba::<f32>::default());

        scope.spawn_stream(color, move |scope, new_color| {
            *scope.write(latest_color) = new_color;
            scope.update_dedup(components::color(), new_color).unwrap();
        });

        Tooltip::new(
            Rectangle::new(Srgba::default())
                .with_margin(spacing_small())
                .with_corner_radius(default_corner_radius())
                .with_exact_size(Unit::px2(size, size)),
            move |scope| {
                let color = *scope.read(latest_color);
                let hex = color_hex(color.without_alpha());

                let oklch: Oklch = color.into_color();

                let lch = label(format!(
                    "L {:.3} C {:.3} H {:.2}",
                    oklch.l,
                    oklch.chroma,
                    oklch.hue.into_positive_degrees()
                ));
                col((label(hex), lch)).with_margin(spacing_medium())
            },
        )
        .mount(scope);
    }
}

fn icon_header(icon: impl Into<String>, text: impl Into<String>) -> impl Widget {
    row((label(icon), label(text))).center()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PaletteCollection {
    palettes: Vec<Palette>,
    shade_settings: Mutable<ShadeSettings>,
}

pub fn main_app() -> impl Widget {
    let palettes = {
        let shade_settings = Mutable::new(ShadeSettings::default());
        Mutable::new(PaletteCollection {
            palettes: (0..3).map(|i| create_rainbow_palette(i)).collect(),
            shade_settings: shade_settings.clone(),
        })
    };

    let current_selection: Mutable<usize> = Mutable::new(0);

    let palette_list = palettes
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

                        palette.widget(i, palettes, v.shade_settings.clone(), current_selection)
                    })
                    .collect_vec();

                ScrollArea::vertical(col(values)).with_max_size(Unit::px2(f32::MAX, 600.0))
            }
        })
        .boxed();

    let sort = Button::label(LUCIDE_ARROW_DOWN_NARROW_WIDE)
        .with_tooltip_text("Sort by Hue")
        .on_click({
            to_owned!(palettes);
            move |_| {
                palettes.lock_mut().palettes.sort_by_key(|palette| {
                    let color = palette.base_color.get().as_oklab();

                    let a = (color.chroma * 100.0) as u32;
                    let b = color.hue.into_positive_degrees() as u32;

                    (a, b)
                })
            }
        });

    let add_palette = Button::label(LUCIDE_PLUS)
        .with_tooltip_text("New Palette")
        .on_click({
            to_owned!(palettes, current_selection);
            move |_| {
                let mut palettes = palettes.lock_mut();

                let index = palettes.palettes.len();

                let new_palette = palettes
                    .palettes
                    .last()
                    .map(|v| {
                        let base_color = v.base_color.get();

                        Palette {
                            name: Mutable::new(format!("Color {}", index + 1)),
                            base_color: Mutable::new(base_color),
                        }
                    })
                    .unwrap_or_else(|| create_rainbow_palette(index));

                palettes.palettes.push(new_palette);

                current_selection.set(index);
            }
        });

    let active_color = zip_latest(
        current_selection.stream(),
        palettes.signal_ref(|_| {}).to_stream(),
    )
    .map({
        to_owned!(palettes);
        move |(palette_index, ())| {
            palettes
                .lock_ref()
                .palettes
                .get(palette_index)
                .map(|v| v.base_color.clone())
        }
    });

    let widget = palettes
        .clone()
        .project_ref(|v| &v.shade_settings, |v| &mut v.shade_settings)
        .stream()
        .map(shade_settings);

    panel(
        col((
            export_controls(palettes),
            StreamWidget::new(active_color.map(|v| v.map(|color| color_editor(color)))),
            StreamWidget::new(widget),
            card(Collapsible::new(
                icon_header(LUCIDE_PALETTE, "Palettes"),
                col((StreamWidget::new(palette_list), row((add_palette, sort)))),
            )),
        ))
        .with_stretch(true)
        .with_contain_margins(true),
    )
    .with_background(surface_primary())
    .with_maximize(Vec2::ONE)
}

fn create_rainbow_palette(index: usize) -> Palette {
    let base_color = Oklch::new(0.5, 0.27, index as f32 * 60.0).into_color();

    Palette {
        base_color: Mutable::new(ColorValue::OkLab(base_color)),
        name: Mutable::new(format!("Color {}", index + 1)),
    }
}

fn color_editor(color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>) -> impl Widget {
    let color = Arc::new(color);

    card(row((
        card(col((
            color_swatch(color.stream().map(|v| v.as_rgb().with_alpha(1.0)), 200.0),
            color_hex_editor(color.clone()),
        ))),
        col((
            rgb_picker(color.clone()),
            hsl_picker(color.clone()),
            oklab_picker(color),
        )),
    )))
}

const ROUNDING: u32 = 4;

pub fn precise_slider<T>(
    value: impl 'static + Send + Sync + StateDuplex<Item = T>,
    min: T,
    max: T,
) -> LabeledSlider<T>
where
    T: Default + FromStr + ToString + SliderValue,
{
    LabeledSlider::input(value, min, max).with_style(SliderStyle {
        track_size: Unit::px2(360.0, 4.0),
        ..Default::default()
    })
}

fn rgb_picker(color: impl 'static + Send + Sync + StateDuplex<Item = ColorValue>) -> impl Widget {
    let color = Arc::new(
        color
            .map_value(|v| v.as_rgb(), ColorValue::Rgb)
            .map_value(Srgb::<u8>::from_format, |v| v.into_format())
            .memo(Default::default()),
    );

    let r = precise_slider(
        color.clone().project_ref(|v| &v.red, |v| &mut v.red),
        0,
        255,
    );
    let g = precise_slider(
        color.clone().project_ref(|v| &v.green, |v| &mut v.green),
        0,
        255,
    );
    let b = precise_slider(
        color.clone().project_ref(|v| &v.blue, |v| &mut v.blue),
        0,
        255,
    );

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
        .project_ref(|v| &v.hue, |v| &mut v.hue)
        .map_value(|v| v.into_positive_degrees(), RgbHue::from_degrees)
        .memo(Default::default());

    let h = precise_slider(hue, 0.0, 360.0).precision(1);
    let s = precise_slider(
        color
            .clone()
            .project_ref(|v| &v.saturation, |v| &mut v.saturation),
        0.0,
        1.0,
    )
    .precision(ROUNDING);

    let l = precise_slider(
        color
            .clone()
            .project_ref(|v| &v.lightness, |v| &mut v.lightness),
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
        .project_ref(|v| &v.hue, |v| &mut v.hue)
        .map_value(|v| v.into_positive_degrees(), OklabHue::from_degrees)
        .memo(Default::default());

    let h = precise_slider(hue, 0.0, 360.0).precision(1);
    let c = precise_slider(
        color.clone().project_ref(|v| &v.chroma, |v| &mut v.chroma),
        0.0,
        0.37,
    )
    .precision(3);

    let l = precise_slider(color.clone().project_ref(|v| &v.l, |v| &mut v.l), 0.0, 1.0)
        .precision(ROUNDING);

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

fn local_dir() -> PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::current_dir().unwrap()
    }
    #[cfg(target_arch = "wasm32")]
    {
        PathBuf::from(".")
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

    let savefile = Mutable::new(None);

    async fn select_file(default_file: Option<String>) -> Option<FileHandle> {
        let dialog = AsyncFileDialog::new().set_directory(local_dir());

        let dialog = if let Some(file) = default_file {
            dialog.set_file_name(&file)
        } else {
            dialog
        };

        dialog.pick_file().await
    }

    async fn get_savefile(savefile: Mutable<Option<PathBuf>>) -> Option<FileHandle> {
        if let Some(file) = savefile.get_cloned() {
            return Some(file.into());
        } else {
            let file = select_file(Some("colors.json".to_string())).await?;
            savefile.set(Some(file.path().to_path_buf()));

            Some(file)
        }
    }

    let save = {
        to_owned!(palettes, result_tx, savefile);
        move |scope: &ScopeRef| {
            to_owned!(result_tx, savefile);
            let data = serde_json::to_string_pretty(&palettes).unwrap();

            let fut = async move {
                let selected_file = get_savefile(savefile.clone())
                    .await
                    .context("No file specified")?;

                selected_file
                    .write(data.as_bytes())
                    .await
                    .context("Failed to write to save file")?;

                anyhow::Ok(())
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
        to_owned!(palettes, result_tx, savefile);
        move |scope: &ScopeRef| {
            to_owned!(palettes, result_tx, savefile);
            let fut = async move {
                let file = select_file(Some("colors.json".to_string()))
                    .await
                    .context("No file specified")?;

                savefile.set(Some(file.path().to_path_buf()));

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

    #[derive(Copy, Clone, Default, PartialEq, PartialOrd)]
    enum ExportFormat {
        #[default]
        DesignTokens,
        Tailwind,
        Rust,
    }

    impl Widget for ExportFormat {
        fn mount(self, scope: &mut Scope<'_>) {
            label(match self {
                ExportFormat::Tailwind => "Tailwind",
                ExportFormat::DesignTokens => "Design Tokens",
                ExportFormat::Rust => "Rust",
            })
            .mount(scope);
        }
    }

    let formatter = |palettes: &PaletteCollection, format: ExportFormat| match format {
        ExportFormat::Tailwind => {
            serde_json::to_string_pretty(&TailwindExport::from_palettes(&palettes)).unwrap()
        }
        ExportFormat::DesignTokens => {
            serde_json::to_string_pretty(&DesignTokenExport::from_palettes(&palettes)).unwrap()
        }
        ExportFormat::Rust => export_hex_list(palettes),
    };

    let export_format = Mutable::new(ExportFormat::default());

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
        raised_card(
            row((
                Button::new(icon_header(LUCIDE_SAVE, "Save")).on_click(save),
                Button::new(icon_header(LUCIDE_FOLDER_OPEN, "Open")).on_click(load),
                StreamWidget::new(savefile.stream().map(|v| {
                    v.as_ref()
                        .and_then(|v| v.file_name())
                        .map(|v| label(v.display().to_string()))
                })),
            ))
            .center(),
        ),
        raised_card(
            row((
                label("Export Format"),
                Dropdown::new(
                    export_format,
                    [
                        ExportFormat::Tailwind,
                        ExportFormat::DesignTokens,
                        ExportFormat::Rust,
                    ],
                ),
                Button::new(icon_header(LUCIDE_DOWNLOAD, "Export")).on_click(export),
                Button::new(icon_header(LUCIDE_COPY, "Copy")).on_click(export_clipboard),
            ))
            .center(),
        ),
        StreamWidget::new(result_rx.into_stream()),
    ))
    .center()
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

            let base_color = v.base_color.get().as_oklab();
            let shade_settings = palettes.shade_settings.get();
            (0..SHADE_COUNT).map(move |i| {
                let tint_name = TINTS[i];

                let color =
                    shade_settings.tint_from_base(base_color, i as f32 / (SHADE_COUNT - 1) as f32);
                let hex = color_hex(color);

                format!("pub const {name}_{tint_name}: Srgba = srgba!(\"{hex}\");")
            })
        })
        .join("\n")
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct TailwindExport {
    palettes: IndexMap<String, Vec<String>>,
}

impl TailwindExport {
    pub fn from_palettes(palettes: &PaletteCollection) -> TailwindExport {
        let mut used: HashMap<_, usize> = HashMap::new();
        TailwindExport {
            palettes: palettes
                .palettes
                .iter()
                .map(|palette| {
                    let mut name = palette.name.get_cloned().to_kebab_case();
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

                    let shades = palette
                        .all_shades(&*palettes.shade_settings.lock_ref())
                        .map(move |color| color_hex(color))
                        .collect_vec();

                    (name, shades)
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct DesignTokenExport {
    palettes: IndexMap<String, IndexMap<String, String>>,
}

impl DesignTokenExport {
    pub fn from_palettes(palettes: &PaletteCollection) -> Self {
        let mut used: HashMap<_, usize> = HashMap::new();
        Self {
            palettes: palettes
                .palettes
                .iter()
                .map(|palette| {
                    let mut name = palette.name.get_cloned().to_kebab_case();
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

                    let shades = palette
                        .all_shades(&*palettes.shade_settings.lock_ref())
                        .enumerate()
                        .map(move |(i, color)| (TINTS[i].to_string(), color_hex(color)))
                        .collect();

                    (name, shades)
                })
                .collect(),
        }
    }
}
