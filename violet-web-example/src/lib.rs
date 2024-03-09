use glam::Vec2;
use tracing_subscriber::{
    filter::LevelFilter, fmt::format::Pretty, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};
use tracing_web::{performance_layer, MakeWebConsoleWriter};
use violet::{
    core::{
        components,
        layout::{Alignment, Direction},
        style::{
            colors::{
                EERIE_BLACK_400, EERIE_BLACK_DEFAULT, JADE_200, JADE_DEFAULT, LION_DEFAULT,
                REDWOOD_DEFAULT,
            },
            Background, SizeExt,
        },
        text::Wrap,
        unit::Unit,
        widget::{List, Rectangle, SignalWidget, SliderWithLabel, Stack, Text, WidgetExt},
        Edges, Scope, Widget, WidgetCollection,
    },
    flax::components::name,
    futures_signals::signal::{Mutable, SignalExt},
    glam::vec2,
    palette::Srgba,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn run() {
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

    violet::wgpu::App::new().run(MainApp).unwrap();
}

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let color = Mutable::new(Vec3::new(0.0, 0.0, 0.0));

        let lightness = MappedState::new(color.clone(), |v| &v.x, |v| &mut v.x);
        let chroma = MappedState::new(color.clone(), |v| &v.y, |v| &mut v.y);
        let hue = MappedState::new(color.clone(), |v| &v.z, |v| &mut v.z);

        let color_rect = color.signal().map(|v| {
            let color = Oklch::new(v.x, v.y, v.z).into_color();
            Rectangle::new(color).with_min_size(Unit::px2(200.0, 100.0))
        });

        column((
            row((
                Text::new("Lightness"),
                SliderWithInput::new(lightness, 0.0, 1.0),
            )),
            row((Text::new("Chroma"), SliderWithInput::new(chroma, 0.0, 0.37))),
            row((Text::new("Hue"), SliderWithInput::new(hue, 0.0, 360.0))),
            SignalWidget(color.signal().map(|v| Text::new(format!("{}", v)))),
            card(SignalWidget(color_rect)),
        ))
        .with_margin(Edges::even(4.0))
        .mount(scope);
    }
}
