use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{base_colors::*, SizeExt, StylesheetOptions},
    unit::Unit,
    widget::{card, col, row, Rectangle, ScrollArea},
    Widget,
};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

fn app() -> impl Widget {
    const WIDTH: f32 = 100.0;
    card(row((
        col((
            Rectangle::new(EMERALD_500)
                .with_size(Unit::px2(WIDTH, 200.0))
                .with_min_size(Unit::px2(WIDTH, 200.0)),
            ScrollArea::vertical(
                Rectangle::new(EMERALD_400)
                    .with_size(Unit::px2(WIDTH, 200.0))
                    .with_min_size(Unit::px2(WIDTH, 200.0)),
            ),
            Rectangle::new(EMERALD_500)
                .with_size(Unit::px2(WIDTH, 200.0))
                .with_min_size(Unit::px2(WIDTH, 200.0)),
        )),
        // col((
        //     Rectangle::new(TEAL_500)
        //         .with_size(Unit::px2(WIDTH, 200.0))
        //         .with_min_size(Unit::px2(WIDTH, 200.0)),
        //     ScrollArea::vertical(
        //         Rectangle::new(TEAL_400)
        //             .with_size(Unit::px2(WIDTH, 200.0))
        //             .with_min_size(Unit::px2(WIDTH, 200.0)),
        //     ),
        //     Rectangle::new(TEAL_500)
        //         .with_size(Unit::px2(WIDTH, 200.0))
        //         .with_min_size(Unit::px2(WIDTH, 100.0)),
        // )),
    )))
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
