use futures_signals::signal::Mutable;
use glam::Vec2;
use palette::Srgba;
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    style::{base_colors::*, surface_primary, surface_secondary, SizeExt, StylesheetOptions},
    unit::Unit,
    widget::{bold, card, col, label, pill, raised_card, row, Rectangle, Slider},
    Edges, Widget,
};
use violet_lucide::icons::*;
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
        .run(main_ui())
}

fn main_ui() -> impl Widget {
    row((
            col((
                    window(LUCIDE_BRUSH, "Center Panel", card(()).with_maximize(Vec2::ONE)),
                    window(LUCIDE_LIST_TREE, "Bottom Panel", card(()).with_maximize(Vec2::X)),
            )),
            window(
                LUCIDE_SATELLITE,
                "Right Panel",
                col(
                    // Slider::new(Mutable::new(50), 0, 100),
                    // Slider::new(Mutable::new(20), 0, 100),
                    // col((
                    //         row((Stack::new(label("Value")).with_maximize(Vec2::X), TextInput::new(Mutable::new("Editable".to_string())).input_box())),
                    //         )),
                    raised_card(label("lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").with_wrap(cosmic_text::Wrap::WordOrGlyph))
                ),
            )
            .with_max_size(Unit::px2(400.0, f32::MAX))
            .with_maximize(Vec2::Y),
    ))
    .with_background(surface_primary())
    .with_contain_margins(true)
}

fn window(
    icon: impl Into<String>,
    title: impl Into<String>,
    content: impl Widget,
) -> impl Widget + SizeExt {
    card(
        col((
            row((
                label(icon).with_margin(Edges::right(8.0)),
                bold(title),
                Rectangle::new(Srgba::new(0.0, 0.0, 0.0, 0.0)).with_maximize(Vec2::X),
                pill(row((
                    label(LUCIDE_MAXIMIZE).with_color(EMERALD_400),
                    label(LUCIDE_MINUS).with_color(AMBER_400),
                    label(LUCIDE_X).with_color(RUBY_400),
                ))),
            ))
            .center(),
            content,
        )), // .with_stretch(true),
    )
    .with_background(surface_secondary())
}
