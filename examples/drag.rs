use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::widget::{card, col};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

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
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(col(card(violet_demo::drag::app())).with_contain_margins(true))
}
