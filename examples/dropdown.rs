use std::time::Duration;

use futures_signals::signal::Mutable;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    layout::Align,
    style::{base_colors::*, SizeExt, StylesheetOptions},
    time::sleep,
    unit::Unit,
    widget::{
        bold, card, col, interactive::dropdown::Dropdown, label, row, Button, Collapsible,
        Rectangle, SuspenseWidget, Throbber,
    },
    Widget,
};
use violet_lucide::icons::{
    LUCIDE_BACKPACK, LUCIDE_BOX, LUCIDE_BRIEFCASE_BUSINESS, LUCIDE_DROPLETS, LUCIDE_HAMMER,
    LUCIDE_HEADPHONES, LUCIDE_LEAF, LUCIDE_WRENCH,
};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    let selection = Mutable::new(None);
    card(Dropdown::new(
        selection,
        [
            row((bold(LUCIDE_BOX).with_color(OCEAN_400), label("Box"))),
            row((
                bold(LUCIDE_DROPLETS).with_color(AMETHYST_400),
                label("Liquid"),
            )),
            row((bold(LUCIDE_HAMMER).with_color(AMBER_400), label("Tools"))),
            row((bold(LUCIDE_BACKPACK).with_color(RUBY_400), label("Items"))),
            row((
                bold(LUCIDE_HEADPHONES).with_color(EMERALD_400),
                label("Music"),
            )),
            row((
                bold(LUCIDE_BRIEFCASE_BUSINESS).with_color(CITRUS_400),
                label("Business"),
            )),
            row((bold(LUCIDE_WRENCH).with_color(OCEAN_400), label("Settings"))),
            row((bold(LUCIDE_LEAF).with_color(FOREST_400), label("Nature"))),
        ],
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
