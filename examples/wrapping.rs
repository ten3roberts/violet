use palette::Srgba;
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{colors::EMERALD_400, primary_surface, spacing_medium, spacing_small, SizeExt},
    unit::Unit,
    widget::{col, row, Rectangle, Stack, Text},
    Widget,
};
use violet_wgpu::renderer::MainRendererConfig;

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

    violet_wgpu::AppBuilder::new()
        .with_renderer_config(MainRendererConfig { debug_mode: true })
        .run(main_app())
}

fn block(text: impl Into<String>, color: Rectangle) -> impl Widget {
    Stack::new((color, Text::new(text.into())))
}

fn main_app() -> impl Widget {
    let test_1 = row((
        Rectangle::new(EMERALD_400)
            .with_size(Unit::px2(200.0, 20.0))
            .with_margin(spacing_medium()),
        Rectangle::new(EMERALD_400)
            .with_min_size(Unit::px2(200.0, 20.0))
            .with_size(Unit::px2(200.0, 20.0))
            .with_margin(spacing_medium()),
    ));

    col(test_1)
        .with_background(primary_surface())
        .with_contain_margins(true)
}
