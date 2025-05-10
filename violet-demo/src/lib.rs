#![allow(unexpected_cfgs)]

use futures::StreamExt;
use glam::Vec2;
use violet::{
    core::{
        state::{StateExt, StateStream},
        style::{
            element_hover, element_interactive, element_pressed, surface_hover, surface_pressed,
            surface_primary, SizeExt, StyleExt,
        },
        widget::{card, col, panel, row, ButtonStyle, ColorPair, Radio, StreamWidget, WidgetExt},
        Edges, Widget,
    },
    futures_signals::signal::Mutable,
    palette::Srgba,
    wgpu::{renderer::MainRendererConfig, AppBuilder},
};
use wasm_bindgen_futures::wasm_bindgen;

pub mod bridge_of_death;
pub mod colorpicker;
pub mod drag;
pub mod widgets;

#[cfg(target_arch = "wasm32")]
fn setup() {
    use tracing_subscriber::{
        filter::LevelFilter, fmt::format::Pretty, layer::SubscriberExt, util::SubscriberInitExt,
        Layer,
    };
    use tracing_web::{performance_layer, MakeWebConsoleWriter};

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
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum DemoState {
    Widgets,
    BridgeOfDeath,
    PaletteEditor,
}

pub fn multi_app() -> impl Widget {
    let state = Mutable::new(DemoState::Widgets);

    let transparent = Srgba::new(0.0, 0.0, 0.0, 0.0);

    let radio_label = |label: &str, value: DemoState| {
        Radio::label(
            label,
            state.clone().map_value(move |s| s == value, move |_| value),
        )
        .with_margin(Edges::ZERO)
        .with_style(ButtonStyle {
            normal: ColorPair::new(transparent, element_interactive()),
            pressed: ColorPair::new(surface_pressed(), element_pressed()),
            hover: ColorPair::new(surface_hover(), element_hover()),
        })
    };

    let selection = col((
        radio_label("Widgets", DemoState::Widgets),
        radio_label("The Bridge of Death", DemoState::BridgeOfDeath),
        radio_label("Palette Editor", DemoState::PaletteEditor),
    ))
    .with_stretch(true)
    .with_maximize(Vec2::Y);

    panel(row((
        card(selection),
        StreamWidget::new(state.stream().map(|v| match v {
            DemoState::Widgets => widgets::main_app().boxed(),
            DemoState::BridgeOfDeath => bridge_of_death::app().boxed(),
            DemoState::PaletteEditor => colorpicker::main_app().boxed(),
        })),
    )))
    .with_background(surface_primary())
}

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run() {
    setup();

    AppBuilder::new()
        .with_title("Demo")
        .with_font(violet::lucide::font_source())
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(multi_app())
        .unwrap();
}
