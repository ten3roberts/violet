use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{base_colors::EMERALD_400, spacing_medium, surface_primary, SizeExt},
    unit::Unit,
    widget::{col, row, Rectangle},
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
        .with_background(surface_primary())
        .with_contain_margins(true)
}
