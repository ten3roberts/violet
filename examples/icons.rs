use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::base_colors::*,
    widget::{card, col, icon, row},
    Widget,
};
use violet_lucide::LucideIcon;
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    col(card(row((
        icon(LucideIcon::CircleChevronRight).with_color(EMERALD_400),
        icon(LucideIcon::Folder).with_color(SAPPHIRE_400),
        icon(LucideIcon::AlignCenter).with_color(AMETHYST_400),
        icon(LucideIcon::Terminal).with_color(SAPPHIRE_600),
        icon(LucideIcon::Move).with_color(SAPPHIRE_400),
        icon(LucideIcon::RotateCw).with_color(EMERALD_400),
        icon(LucideIcon::Expand).with_color(RUBY_400),
    ))))
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
        .with_icon_pack(violet_lucide::pack())
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}
