use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet::core::{
    style::{spacing_medium, Background, SizeExt},
    unit::Unit,
    widget::{Positioned, Rectangle, Stack},
    Widget,
};
use violet_core::style::{base_colors::EMERALD_500, surface_secondary};

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

    violet_wgpu::AppBuilder::new().run(app())
}

fn app() -> impl Widget {
    Stack::new(
        Positioned::new(
            Rectangle::new(EMERALD_500)
                .with_min_size(Unit::px2(100.0, 100.0))
                .with_margin(spacing_medium()),
        )
        .with_offset(Unit::px2(10.0, 10.0)),
    )
    .with_padding(spacing_medium())
    .with_background(Background::new(surface_secondary()))
}
