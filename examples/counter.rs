use flax::components::name;
use futures_signals::signal::{Mutable, SignalExt};
use glam::vec2;
use palette::Srgba;
use tracing_subscriber::{prelude::*, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet::core::{
    components::size,
    layout::Align,
    unit::Unit,
    widget::{Button, List, SignalWidget, Text},
    Scope, Widget,
};
use violet_core::{
    style::{element_accent, Background},
    widget::pill,
};

macro_rules! srgba {
    ($color:literal) => {{
        let [r, g, b] = color_hex::color_from_hex!($color);

        Srgba::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }};
}

pub const EERIE_BLACK: Srgba = srgba!("#222525");
pub const EERIE_BLACK_300: Srgba = srgba!("#151616");
pub const EERIE_BLACK_400: Srgba = srgba!("#1b1e1e");
pub const EERIE_BLACK_600: Srgba = srgba!("#4c5353");
pub const PLATINUM: Srgba = srgba!("#dddddf");
pub const VIOLET: Srgba = srgba!("#8000ff");
pub const TEAL: Srgba = srgba!("#247b7b");
pub const EMERALD: Srgba = srgba!("#50c878");
pub const BRONZE: Srgba = srgba!("#cd7f32");
pub const CHILI_RED: Srgba = srgba!("#d34131");

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "MainApp".into())
            .set(size(), Unit::rel(vec2(1.0, 1.0)));

        let counter = Mutable::new(0);

        List::new((
            pill(SignalWidget(
                counter
                    .signal()
                    .map(|v| Text::new(format!("Count: {v:>4}"))),
            )),
            Button::new(Text::new("Increment")).on_click(move |_| *counter.lock_mut() += 1),
            pill(Text::new(
                "Please click the button to increment the counter",
            )),
        ))
        .with_background(Background::new(element_accent()))
        .with_cross_align(Align::Center)
        .mount(scope);
    }
}

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

    violet_wgpu::AppBuilder::new().run(MainApp)
}
