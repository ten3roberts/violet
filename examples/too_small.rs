use glam::Vec2;
use palette::Srgba;
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{
        base_colors::*, default_corner_radius, surface_primary, surface_secondary, SizeExt,
        StylesheetOptions,
    },
    unit::Unit,
    widget::{bold, card, col, label, pill, raised_card, row, Button, Rectangle},
    Edges, Widget,
};
use violet_lucide::icons::*;
use violet_wgpu::renderer::MainRendererConfig;

pub fn main() -> anyhow::Result<()> {
    registry()
        .with(
            HierarchicalLayer::default()
                // .with_deferred_spans(true)
                .with_span_retrace(true)
                .with_indent_lines(true)
                .with_indent_amount(4),
        )
        .with(EnvFilter::from_default_env())
        .init();

    violet_wgpu::AppBuilder::new()
        .with_font(violet_lucide::font_source())
        .with_stylesheet(
            StylesheetOptions::new()
                .with_icons(violet_lucide::icon_set())
                .build(),
        )
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(main_ui())
}

fn main_ui() -> impl Widget {
    col((
        row(pill(label("XYZ")).with_background(RUBY_400)),
        row(Rectangle::new(RUBY_400)
            .with_size(Unit::px2(40.0, 25.0))
            .with_min_size(Unit::px2(10.0, 25.0))
            .with_corner_radius(Unit::px(16.0))),
        // label("Long Test Text").with_wrap(cosmic_text::Wrap::Word),
        Rectangle::new(SAPPHIRE_400)
            .with_size(Unit::px2(100.0, 25.0))
            .with_min_size(Unit::px2(75.0, 25.0))
            .with_corner_radius(Unit::px(16.0)),
    ))
    .with_padding(Edges::even(40.0))
    .with_background(surface_primary())
    .with_contain_margins(true)
}
