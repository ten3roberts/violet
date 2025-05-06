use std::sync::Arc;

use cosmic_text::fontdb::Source;
use flax::component;
use futures_signals::signal::Mutable;
use itertools::Itertools;
use palette::{Srgba, WithAlpha};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{base_colors::*, spacing_small, SizeExt},
    unit::Unit,
    widget::{
        card, col, drop_target, interactive::tooltip::Tooltip, label, pill, row, Draggable,
        Rectangle, SignalWidget,
    },
    Scope, Widget,
};
use violet_lucide::{
    lucide_icons::{
        ICON_ALIGN_CENTER, ICON_CIRCLE_CHEVRON_RIGHT, ICON_EXPAND, ICON_FOLDER, ICON_MOVE,
        ICON_ROTATE_CW, ICON_TERMINAL,
    },
    LucideIcon,
};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    col(card(col((
        LucideIcon::new(ICON_CIRCLE_CHEVRON_RIGHT).with_color(EMERALD_400),
        LucideIcon::new(ICON_FOLDER).with_color(OCEAN_400),
        LucideIcon::new(ICON_ALIGN_CENTER).with_color(AMETHYST_400),
        LucideIcon::new(ICON_TERMINAL).with_color(OCEAN_600),
        LucideIcon::new(ICON_MOVE).with_color(OCEAN_400),
        LucideIcon::new(ICON_ROTATE_CW).with_color(EMERALD_400),
        LucideIcon::new(ICON_EXPAND).with_color(RUBY_400),
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
        .with_font(Source::Binary(Arc::new(include_bytes!(
            "../violet-lucide/bin/lucide/lucide.ttf"
        ))))
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(app())
}
