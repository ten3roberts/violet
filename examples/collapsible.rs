use std::time::Duration;

use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    layout::Align,
    style::{base_colors::*, SizeExt, StylesheetOptions},
    time::sleep,
    unit::Unit,
    widget::{bold, card, col, label, Button, Collapsible, Rectangle, SuspenseWidget, Throbber},
    Widget,
};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    card(
        Collapsible::deferred(label("Collapse"), || {
            SuspenseWidget::new(
                col((Throbber::new(64.0), bold("Loading"))).with_cross_align(Align::Center),
                async move {
                    sleep(Duration::from_secs(5)).await;
                    col((
                        Button::label("Press Me"),
                        Rectangle::new(RUBY_400).with_exact_size(Unit::px2(400.0, 40.0)),
                        Rectangle::new(RUBY_500).with_exact_size(Unit::px2(400.0, 40.0)),
                        Rectangle::new(RUBY_600).with_exact_size(Unit::px2(400.0, 40.0)),
                    ))
                },
            )
        })
        .collapsed(true),
    )
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
