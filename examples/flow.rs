use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet::core::{
    style::{Background, SizeExt},
    widget::{col, row, Button},
    Edges, Widget,
};
use violet_core::style::{surface_primary, surface_secondary};
use violet_wgpu::renderer::MainRendererConfig;

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

    violet_wgpu::AppBuilder::new()
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}

fn app() -> impl Widget {
    col(col((
        // row((Button::label("Row 1"), Button::label("Row 2"))).contain_margins(true),
        row((
            Button::label("Button").with_margin(Edges::even(32.0)),
            Button::label("Button"),
        )),
        Button::label("Button"),
        row((Button::label("Button"), Button::label("Button"))),
        row((Button::label("Longer Button"), Button::label("Button"))),
    ))
    .with_background(Background::new(surface_secondary())))
    .with_contain_margins(true)
    .with_background(Background::new(surface_primary()))
    // .with_padding(spacing_medium())
}
