use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{base_colors::*, SizeExt, StylesheetOptions},
    unit::Unit,
    widget::{card, col, Button, Collapsible, Rectangle},
    Widget,
};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    card(Collapsible::label(
        "Collapse",
        col((
            Button::label("Press Me"),
            Rectangle::new(RUBY_400).with_exact_size(Unit::px2(400.0, 40.0)),
            Rectangle::new(RUBY_500).with_exact_size(Unit::px2(400.0, 40.0)),
            Rectangle::new(RUBY_600).with_exact_size(Unit::px2(400.0, 40.0)),
        )),
    ))
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

    AppBuilder::new()
        .with_font(violet_lucide::font_source())
        .with_stylesheet(
            StylesheetOptions::new()
                .with_icons(violet_lucide::icon_set())
                .build(),
        )
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}
