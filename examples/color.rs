use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::Widget;
use violet_wgpu::renderer::RendererConfig;

pub fn main() -> anyhow::Result<()> {
    registry()
        .with(
            HierarchicalLayer::default()
                .with_deferred_spans(true)
                .with_span_retrace(true)
                .with_indent_lines(true)
                .with_indent_amount(4),
        )
        .with(EnvFilter::from_default_env())
        .init();

    violet_wgpu::App::new()
        .with_renderer_config(RendererConfig { debug_mode: false })
        .run(MainApp)
}

struct MainApp;

impl Widget for MainApp {
    fn mount(scope: &mut Scope<'_>) {}
}
