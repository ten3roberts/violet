use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::base_colors::*,
    widget::{card, col, label, row},
    Widget,
};
use violet_lucide::icons::*;
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    col(card(row((
        label(LUCIDE_CIRCLE_CHEVRON_RIGHT).with_color(EMERALD_400),
        label(LUCIDE_FOLDER).with_color(SAPPHIRE_400),
        label(LUCIDE_ALIGN_CENTER).with_color(AMETHYST_400),
        label(LUCIDE_TERMINAL).with_color(SAPPHIRE_600),
        label(LUCIDE_MOVE).with_color(SAPPHIRE_400),
        label(LUCIDE_ROTATE_CW).with_color(EMERALD_400),
        label(LUCIDE_EXPAND).with_color(RUBY_400),
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
        .with_font(violet_lucide::font_source())
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}
