use core::f32;

use glam::Vec2;
use palette::Srgba;
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;
use violet_core::{
    layout::FlowLayout,
    style::{
        base_colors::*, element_secondary, spacing_medium, spacing_small, surface_primary,
        surface_secondary, SizeExt, StylesheetOptions,
    },
    unit::{Unit, Zero},
    widget::{bold, card, col, label, pill, raised_card, row, Button, List, Rectangle, Stack},
    Edges, Widget, WidgetCollection,
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
        .run(layout_using_flow())
}

fn card_layout() -> impl Widget {
    row((
        col((
            card(()).with_maximize(Vec2::ONE),
            card(())
                .with_min_size(Unit::px2(260.0, 40.0))
                .with_maximize(Vec2::X),
        )),
        card(())
            .with_max_size(Unit::px2(40.0, f32::MAX))
            .with_min_size(Unit::px2(40.0, 0.0))
            .with_maximize(Vec2::Y), // window(
                                     //     LUCIDE_SATELLITE,
                                     //     "Right Panel",
                                     //     col(
                                     //         // Slider::new(Mutable::new(50), 0, 100),
                                     //         // Slider::new(Mutable::new(20), 0, 100),
                                     //         // col((
                                     //         //         row((Stack::new(label("Value")).with_maximize(Vec2::X), TextInput::new(Mutable::new("Editable".to_string())).input_box())),
                                     //         //         )),
                                     //         raised_card(label("lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").with_wrap(cosmic_text::Wrap::WordOrGlyph))
                                     //     ),
                                     // )
                                     // .with_max_size(Unit::px2(400.0, f32::MAX))
                                     // .with_maximize(Vec2::Y),
    ))
    .with_background(surface_primary())
    .with_contain_margins(true)
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
                    btn(LUCIDE_MAXIMIZE).success(),
                    btn(LUCIDE_MINUS).warning(),
                    btn(LUCIDE_X).danger(),
                ))),
            ))
            .center(),
            content,
        )), // .with_stretch(true),
    )
    .with_background(surface_secondary())
}

fn btn(_label: impl Into<String>) -> Button<impl Widget> {
    Button::new(
        label(_label),
        // Rectangle::new(element_secondary())
        //     .with_min_size(Unit::px2(10.0, 10.0))
        //     .with_size(Unit::px2(10.0, 10.0)),
    )
}
