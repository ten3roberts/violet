use std::sync::Arc;

use futures::StreamExt;
use glam::{Vec2, Vec3};
use itertools::Itertools;
use tracing_subscriber::{
    filter::LevelFilter, fmt::format::Pretty, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};
use tracing_web::{performance_layer, MakeWebConsoleWriter};
use violet::{
    core::{
        layout::Alignment,
        state::{DynStateDuplex, State, StateMut, StateStream, StateStreamRef},
        style::{primary_background, Background, SizeExt, ValueOrRef},
        to_owned,
        unit::Unit,
        utils::zip_latest_ref,
        widget::{
            card, column, label, pill, row, Button, Checkbox, Rectangle, SliderWithLabel, Stack,
            StreamWidget, Text, TextInput, WidgetExt,
        },
        Edges, Scope, Widget,
    },
    futures_signals::signal::Mutable,
    glam::vec2,
    palette::{FromColor, IntoColor, OklabHue, Oklch, Srgb},
    wgpu::renderer::RendererConfig,
};
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
fn setup() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(MakeWebConsoleWriter::new())
        .with_filter(LevelFilter::INFO);

    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();

    console_error_panic_hook::set_once();
}

#[cfg(not(target_arch = "wasm32"))]
fn setup() {
    tracing_subscriber::registry()
        .with(
            tracing_tree::HierarchicalLayer::default()
                .with_deferred_spans(true)
                .with_span_retrace(true)
                .with_indent_lines(true)
                .with_indent_amount(4),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[wasm_bindgen]
pub fn run() {
    setup();

    violet::wgpu::App::new()
        .with_renderer_config(RendererConfig { debug_mode: true })
        .run(MainApp)
        .unwrap();
}

struct MainApp;

const DEFAULT_FALLOFF: f32 = 15.0;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let palette_item = Mutable::new(
            (0..8)
                .map(|i| {
                    Mutable::new(PaletteColor {
                        color: Oklch::new(0.5, 0.27, (i as f32 * 60.0) % 360.0),
                        falloff: DEFAULT_FALLOFF,
                    })
                })
                .collect(),
        );

        column((Palettes::new(palette_item),))
            .with_size(Unit::rel2(1.0, 1.0))
            .with_background(Background::new(primary_background()))
            .contain_margins(true)
            .mount(scope);
    }
}

struct Tints {
    base: Oklch,
    falloff: f32,
}

impl Tints {
    fn new(base: Oklch, falloff: f32) -> Self {
        Self { base, falloff }
    }
}

impl Widget for Tints {
    fn mount(self, scope: &mut Scope<'_>) {
        row((1..=9)
            .map(|i| {
                let f = (i as f32) / 10.0;
                let chroma = self.base.chroma * (1.0 / (1.0 + self.falloff * (f - 0.5).powi(2)));

                // let color = self.base.lighten(f);
                let color = Oklch {
                    chroma,
                    l: f,
                    ..self.base
                };

                Stack::new(column((Rectangle::new(ValueOrRef::value(
                    color.into_color(),
                ))
                .with_size(Unit::px2(80.0, 60.0)),)))
                .with_margin(Edges::even(4.0))
                .with_name("Tint")
            })
            .collect_vec())
        .mount(scope)
    }
}

pub fn color_hex(color: impl IntoColor<Srgb>) -> String {
    let hex: Srgb<u8> = color.into_color().into_format();
    format!("#{:0>2x}{:0>2x}{:0>2x}", hex.red, hex.green, hex.blue)
}

pub struct Palettes {
    items: Mutable<Vec<Mutable<PaletteColor>>>,
}

impl Palettes {
    pub fn new(items: Mutable<Vec<Mutable<PaletteColor>>>) -> Self {
        Self { items }
    }
}

impl Widget for Palettes {
    fn mount(self, scope: &mut Scope<'_>) {
        let items = self.items.clone();

        let discard = move |i| {
            let items = items.clone();
            Button::new(Text::new("-"))
                .on_press({
                    move |_, _| {
                        items.lock_mut().remove(i);
                    }
                })
                .danger()
        };

        let current_choice = Mutable::new(Some(0));

        let editor = zip_latest_ref(
            self.items.stream(),
            current_choice.stream(),
            |items, i: &Option<usize>| {
                i.and_then(|i| items.get(i).cloned())
                    .map(PaletteEditor::new)
            },
        );

        let palettes = StreamWidget(self.items.stream_ref({
            to_owned![current_choice];
            move |items| {
                let items = items
                    .iter()
                    .enumerate()
                    .map({
                        to_owned![current_choice];
                        let discard = &discard;
                        move |(i, item)| {
                            let checkbox = Checkbox::new(
                                current_choice
                                    .clone()
                                    .map(move |v| v == Some(i), move |state| state.then_some(i)),
                            );

                            card(row((checkbox, discard(i), StreamWidget(item.stream()))))
                        }
                    })
                    .collect_vec();

                column(items)
            }
        }));

        let items = self.items.clone();

        column((
            StreamWidget(editor),
            palettes,
            Button::label("+").on_press(move |_, _| {
                items.write_mut(|v| {
                    v.push(Mutable::new(PaletteColor {
                        color: Oklch::new(0.5, 0.27, (v.len() as f32 * 60.0) % 360.0),
                        falloff: DEFAULT_FALLOFF,
                    }));
                    current_choice.set(Some(v.len() - 1));
                })
            }),
        ))
        .mount(scope)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PaletteColor {
    color: Oklch,
    falloff: f32,
}

impl Widget for PaletteColor {
    fn mount(self, scope: &mut Scope<'_>) {
        Stack::new((
            row((Tints::new(self.color, self.falloff),)),
            pill(label(color_hex(self.color))),
        ))
        .with_vertical_alignment(Alignment::End)
        .with_horizontal_alignment(Alignment::Center)
        .mount(scope)
    }
}

pub struct PaletteEditor {
    color: Mutable<PaletteColor>,
}

impl PaletteEditor {
    pub fn new(color: Mutable<PaletteColor>) -> Self {
        Self { color }
    }
}

impl Widget for PaletteEditor {
    fn mount(self, scope: &mut Scope<'_>) {
        let color = Arc::new(self.color.clone().map_ref(|v| &v.color, |v| &mut v.color));
        let falloff = self.color.map_ref(|v| &v.falloff, |v| &mut v.falloff);

        let lightness = color.clone().map_ref(|v| &v.l, |v| &mut v.l);
        let chroma = color.clone().map_ref(|v| &v.chroma, |v| &mut v.chroma);
        let hue = color
            .clone()
            .map_ref(|v| &v.hue, |v| &mut v.hue)
            .map(|v| v.into_positive_degrees(), OklabHue::new);

        let color_rect = color.stream().map(|v| {
            Rectangle::new(ValueOrRef::value(v.into_color()))
                .with_size(Unit::new(vec2(0.0, 100.0), vec2(1.0, 0.0)))
                .with_name("ColorPreview")
        });

        card(column((
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
        .mount(scope)
    }
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
