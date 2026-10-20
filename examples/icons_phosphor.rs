use itertools::Itertools;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{base_colors::*, SizeExt},
    widget::{card, col, icon, interactive::tooltip::Tooltip, row, ScrollArea},
    Widget,
};
use violet_phosphor::PhosphorIcon;
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

/// Icons per grid row.
const COLUMNS: usize = 32;

fn app() -> impl Widget {
    let rows = PhosphorIcon::ALL
        .iter()
        .chunks(COLUMNS)
        .into_iter()
        .map(|chunk| {
            row(chunk
                .map(|&glyph| Tooltip::label(icon(glyph).with_color(EMERALD_400), glyph.name()))
                .collect_vec())
        })
        .collect_vec();

    col(card(ScrollArea::new([false, true], col(rows))))
        .with_maximize(glam::Vec2::ONE)
        .with_contain_margins(true)
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
        .with_icon_pack(violet_phosphor::pack())
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}
