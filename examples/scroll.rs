use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{BVec2, Vec2};
use itertools::Itertools;
use palette::{Hsv, IntoColor, Oklcha, Srgba};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet::core::{
    style::SizeExt,
    unit::Unit,
    widget::{col, Rectangle},
    Widget,
};
use violet_core::{
    state::{State, StateStream},
    style::{secondary_surface, spacing_medium, Background},
    utils::zip_latest,
    widget::{label, Button, Checkbox, ScrollArea, SliderWithLabel, StreamWidget, WidgetExt},
};
use violet_wgpu::{renderer::MainRendererConfig, AppBuilder};

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

    AppBuilder::new()
        .with_renderer_config(MainRendererConfig { debug_mode: true })
        .run(app())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorSpace {
    Oklcha,
    Hsv,
}

fn app() -> impl Widget {
    let color_space = Mutable::new(ColorSpace::Oklcha);
    let segments = Mutable::new(128);
    col((
        Checkbox::label(
            "Oklch",
            color_space.clone().map_value(
                |v| v == ColorSpace::Oklcha,
                |v| {
                    if v {
                        ColorSpace::Oklcha
                    } else {
                        ColorSpace::Hsv
                    }
                },
            ),
        ),
        SliderWithLabel::new(segments.clone(), 8, 360),
        ScrollArea::new(
            BVec2::TRUE,
            StreamWidget(zip_latest(color_space.stream(), segments.stream()).map(
                |(color_space, count)| {
                    col((0..count)
                        .map(|v| {
                            let hue = v as f32 * 360.0 / count as f32;

                            let color: Srgba = match color_space {
                                ColorSpace::Oklcha => Oklcha::new(0.5, 0.37, hue, 1.0).into_color(),
                                ColorSpace::Hsv => Hsv::new(hue, 1.0, 1.0).into_color(),
                            };

                            Rectangle::new(color)
                                .with_min_size(Unit::px2(600.0, 20.0))
                                .with_maximize(Vec2::X)
                                .boxed()
                        })
                        .chain([label("An adventurous journey").boxed()])
                        .collect_vec())
                },
            )),
        ),
        Button::label("Button"),
    ))
    .with_padding(spacing_medium())
    .with_background(Background::new(secondary_surface()))
    // .with_padding(spacing_medium())
}
