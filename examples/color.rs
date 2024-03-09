use futures_signals::signal::{Mutable, SignalExt};
use glam::Vec3;
use palette::{IntoColor, Oklch};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    project::{MappedDuplex, MappedState},
    style::SizeExt,
    unit::Unit,
    widget::{card, column, row, InputField, Rectangle, SignalWidget, SliderWithInput, Text},
    Edges, Scope, Widget,
};
use violet_wgpu::renderer::RendererConfig;

pub fn main() -> anyhow::Result<()> {
    registry()
        .with(
            HierarchicalLayer::default()
                .with_deferred_spans(true)
                .with_span_retrace(true)
                .with_indent_lines(true)
                .with_indent_amount(4),
        )
        .with(EnvFilter::from_default_env())
        .init();

    violet_wgpu::App::new()
        .with_renderer_config(RendererConfig { debug_mode: false })
        .run(MainApp)
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
