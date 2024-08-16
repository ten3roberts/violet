use glam::{vec2, Vec2};
use itertools::Itertools;
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;

use violet::core::{style::Background, unit::Unit, widget::Rectangle, Scope, Widget};
use violet_core::{
    style::{accent_item, primary_background, spacing_small, SizeExt},
    widget::{centered, col, row, Image, Stack},
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
        .with_renderer_config(MainRendererConfig { debug_mode: true })
        .run(MainApp)
}

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        Stack::new(
            col((
                // row((
                //     label("This text can wrap to save horizontal space"),
                //     card((
                //         Rectangle::new(JADE_DEFAULT).with_size(Unit::px2(100.0, 40.0)),
                //         label("Jade"),
                //     )),
                //     label("This text can wrap to save horizontal space"),
                // )),
                // row((
                //     column((
                //         Rectangle::new(JADE_DEFAULT).with_size(Unit::px2(900.0, 40.0)),
                //         Rectangle::new(JADE_400)
                //             .with_size(Unit::px2(900.0, 40.0))
                //             .with_min_size(Unit::px2(400.0, 40.0)),
                //     )),
                //     Rectangle::new(LION_DEFAULT).with_size(Unit::px2(900.0, 40.0)),
                // )),
                // Rectangle::new(REDWOOD_DEFAULT)
                //     .with_min_size(Unit::px2(100.0, 100.0))
                //     .with_size(Unit::px2(0.0, 100.0) + Unit::rel2(1.0, 0.0)),
                // .with_margin(spacing_medium()),
                row((0..4)
                    .map(|_| Box::new(Stack::new(Item)) as Box<dyn Widget>)
                    .chain([Box::new(
                        centered((Rectangle::new(accent_item())
                            .with_maximize(vec2(1.0, 0.0))
                            .with_size(Unit::px2(0.0, 50.0))
                            .with_max_size(Unit::px2(1000.0, 100.0)),))
                        .with_maximize(Vec2::ONE),
                    ) as Box<dyn Widget>])
                    .collect_vec())
                .with_padding(spacing_small()),
            ))
            // .with_padding(spacing_medium())
            .contain_margins(true),
        )
        .with_background(Background::new(primary_background()))
        .mount(scope)
    }
}

#[derive(Debug, Clone)]
struct Item;

impl Widget for Item {
    fn mount(self, scope: &mut Scope<'_>) {
        Image::new("./assets/images/statue.jpg")
            .with_size(Unit::px2(100.0, 100.0))
            // .with_aspect_ratio(1.0)
            .with_margin(spacing_small())
            .mount(scope)
    }
}
