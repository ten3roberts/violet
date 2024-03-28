use futures::StreamExt;
use glam::Vec2;
use violet::{
    core::{
        state::{State, StateStream},
        style::{accent_surface, SizeExt},
        widget::{col, row, Radio, StreamWidget, WidgetExt},
        Widget,
    },
    futures_signals::signal::Mutable,
    wgpu::{app::App, renderer::RendererConfig},
};
use wasm_bindgen_futures::wasm_bindgen;

pub mod bridge_of_death;
mod palettes;

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

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run() {
    setup();

    App::builder()
        .with_title("Palette Editor")
        .with_renderer_config(RendererConfig { debug_mode: false })
        .run(app())
        .unwrap();
}

#[derive(Debug, Clone, PartialEq)]
enum DemoState {
    Basic,
    PaletteEditor,
}

fn app() -> impl Widget {
    let state = Mutable::new(DemoState::Basic);
    col((
        (row((
            Radio::label(
                "The Bridge of Death",
                state
                    .clone()
                    .map(|s| s == DemoState::Basic, move |_| DemoState::Basic),
            ),
            Radio::label(
                "Palette Editor",
                state.clone().map(
                    |s| s == DemoState::PaletteEditor,
                    move |_| DemoState::PaletteEditor,
                ),
            ),
        ))
        .with_background(accent_surface()))
        .with_maximize(Vec2::X),
        StreamWidget(state.stream().map(|v| match v {
            DemoState::Basic => bridge_of_death::app().boxed(),
            DemoState::PaletteEditor => palettes::App.boxed(),
        })),
    ))
}
