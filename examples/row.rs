use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;

use violet::core::{
    style::{
        colors::{EERIE_BLACK_DEFAULT, REDWOOD_DEFAULT},
        Background,
    },
    unit::Unit,
    widget::Rectangle,
    Scope, Widget,
};
use violet_core::{
    style::{
        colors::{JADE_400, JADE_DEFAULT, LION_DEFAULT},
        spacing_medium, SizeExt,
    },
    widget::{column, row, Stack},
};
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
        .with_renderer_config(RendererConfig { debug_mode: true })
        .run(MainApp)
}

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        Stack::new(
            column((
                row((
                    column((
                        Rectangle::new(JADE_DEFAULT).with_size(Unit::px2(900.0, 40.0)),
                        Rectangle::new(JADE_400).with_size(Unit::px2(900.0, 40.0)),
                    )),
                    Rectangle::new(LION_DEFAULT).with_size(Unit::px2(900.0, 40.0)),
                )),
                Rectangle::new(REDWOOD_DEFAULT)
                    .with_min_size(Unit::px2(100.0, 100.0))
                    .with_size(Unit::px2(0.0, 100.0) + Unit::rel2(1.0, 0.0)),
                // .with_margin(spacing_medium()),
            ))
            // .with_padding(spacing_medium())
            .contain_margins(true),
        )
        .with_background(Background::new(EERIE_BLACK_DEFAULT))
        .mount(scope)
    }
}
