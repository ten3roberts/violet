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
        components,
        layout::{Alignment, Direction},
        state::{Map, MapRef, State, StateMut, StateStream, StateStreamRef},
        style::{
            colors::{
                EERIE_BLACK_400, EERIE_BLACK_DEFAULT, JADE_200, JADE_DEFAULT, LION_DEFAULT,
                REDWOOD_DEFAULT,
            },
            danger_item, success_item, Background, SizeExt, StyleExt, ValueOrRef,
        },
        text::Wrap,
        to_owned,
        unit::Unit,
        utils::{zip_latest, zip_latest_ref},
        widget::{
            card, column, label, row, Button, ButtonStyle, Checkbox, List, Rectangle, SignalWidget,
            SliderWithLabel, Stack, StreamWidget, Text, WidgetExt,
        },
        Edges, Scope, Widget, WidgetCollection,
    },
    flax::components::name,
    futures_signals::signal::{Mutable, SignalExt},
    glam::vec2,
    palette::{FromColor, IntoColor, Oklch, Srgb, Srgba},
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

    violet::wgpu::App::new().run(MainApp).unwrap();
}

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let palette_item = Mutable::new(Vec::new());

        card((Palettes::new(palette_item),))
            .with_margin(Edges::even(4.0))
            .with_size(Unit::rel2(1.0, 1.0))
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

                Stack::new(column((
                    Rectangle::new(ValueOrRef::value(color.into_color()))
                        .with_min_size(Unit::px2(60.0, 60.0)),
                    // Text::new(format!("{:.2}", f)),
                )))
                .with_margin(Edges::even(4.0))
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

        let current_choice = Mutable::new(None as Option<usize>);

        let editor = zip_latest_ref(
            self.items.stream(),
            current_choice.stream(),
            |items, i: &Option<usize>| i.and_then(|i| items.get(i).cloned()).map(OklchEditor::new),
        );

        let palettes = StreamWidget(self.items.stream_ref(move |items| {
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
        }));

        let items = self.items.clone();

        column((
            StreamWidget(editor),
            palettes,
            Button::label("+").on_press(move |_, _| {
                items.write_mut(|v| v.push(Mutable::new(PaletteColor::default())))
            }),
        ))
        .mount(scope)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PaletteColor {
    color: Vec3,
    falloff: f32,
}

impl Default for PaletteColor {
    fn default() -> Self {
        Self {
            color: Vec3::new(0.5, 0.27, 153.0),
            falloff: 10.0,
        }
    }
}

impl Widget for PaletteColor {
    fn mount(self, scope: &mut Scope<'_>) {
        let oklch_color = Oklch::new(self.color.x, self.color.y, self.color.z);
        column((
            row((Tints::new(oklch_color, self.falloff),)),
            label(color_hex(oklch_color)),
        ))
        .with_cross_align(Alignment::Center)
        .mount(scope)
    }
}

pub struct OklchEditor {
    color: Mutable<PaletteColor>,
}

impl OklchEditor {
    pub fn new(color: Mutable<PaletteColor>) -> Self {
        Self { color }
    }
}

impl Widget for OklchEditor {
    fn mount(self, scope: &mut Scope<'_>) {
        let color = Arc::new(self.color.clone().map_ref(|v| &v.color, |v| &mut v.color));
        let falloff = self.color.map_ref(|v| &v.falloff, |v| &mut v.falloff);

        let color_oklch = Map::new(
            color.clone(),
            |v| Oklch::new(v.x, v.y, v.z),
            |v| Vec3::new(v.l, v.chroma, v.hue.into_positive_degrees()),
        );

        let lightness = color.clone().map_ref(|v| &v.x, |v| &mut v.x);
        let chroma = color.clone().map_ref(|v| &v.y, |v| &mut v.y);
        let hue = color.clone().map_ref(|v| &v.z, |v| &mut v.z);

        let color_rect = color.stream().map(|v| {
            let color = Oklch::new(v.x, v.y, v.z).into_color();
            Rectangle::new(ValueOrRef::value(color))
                .with_size(Unit::new(vec2(0.0, 100.0), vec2(1.0, 0.0)))
        });

        column((
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
            StreamWidget(color.stream_ref(|v| {
                let hex: Srgb<u8> = Srgb::from_color(Oklch::new(v.x, v.y, v.z)).into_format();
                Text::new(format!(
                    "#{:0>2x}{:0>2x}{:0>2x}",
                    hex.red, hex.green, hex.blue
                ))
            })),
            StreamWidget(color.stream().map(|v| Text::new(format!("{}", v)))),
            StreamWidget(color_rect),
            row((
                Text::new("Chroma falloff"),
                SliderWithLabel::new(falloff, 0.0, 100.0)
                    .editable(true)
                    .round(1.0),
            )),
        ))
        .mount(scope)
    }
}
