use core::f32;

use glam::Vec2;
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{
        base_colors::*, spacing_medium, surface_primary, surface_secondary, SizeExt,
        StylesheetOptions,
    },
    unit::Unit,
    widget::{col, row, Rectangle, Stack},
    Widget, WidgetCollection,
};
use violet_wgpu::renderer::MainRendererConfig;

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

    violet_wgpu::AppBuilder::new()
        .with_font(violet_lucide::font_source())
        .with_stylesheet(
            StylesheetOptions::new()
                .with_icons(violet_lucide::icon_set())
                .build(),
        )
        .with_renderer_config(MainRendererConfig { debug_mode: false })
        .run(layout_using_flow())
}

fn layout_using_flow() -> impl Widget {
    fn container(content: impl WidgetCollection) -> Stack<impl WidgetCollection> {
        Stack::new(content).with_background(surface_secondary())
        // .with_margin(spacing_small())
    }
    row((
        col((
            container(())
                .with_margin(spacing_medium())
                .with_background(STONE_700)
                .with_maximize(Vec2::ONE)
                // .with_min_size(Unit::px2(0.0, 200.0))
                .with_size(Unit::px2(600.0, 400.0)),
            Stack::new(Rectangle::new(SAPPHIRE_500).with_min_size(Unit::px2(260.0, 40.0))),
            container(())
                .with_margin(spacing_medium())
                .with_background(RUBY_800)
                .with_maximize(Vec2::ONE),
            // .with_size(Unit::px2(1.0, 1.0)),
        )),
        container(())
            .with_margin(spacing_medium())
            .with_background(STONE_900)
            .with_max_size(Unit::px2(40.0, f32::MAX))
            .with_min_size(Unit::px2(40.0, 0.0))
            .with_maximize(Vec2::Y),
    ))
    .with_background(surface_primary())
    .with_contain_margins(true)
}
