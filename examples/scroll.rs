use glam::Vec2;
use itertools::Itertools;
use palette::{FromColor, Oklcha, Srgba};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet::core::{
    style::{spacing_small, SizeExt},
    unit::Unit,
    widget::{col, Rectangle},
    Widget,
};

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

    violet_wgpu::AppBuilder::new().run(app())
}

fn app() -> impl Widget {
    col((0..32)
        .map(|v| {
            Rectangle::new(Srgba::from_color(Oklcha::new(
                0.5,
                0.37,
                v as f32 * 5.0,
                1.0,
            )))
            .with_margin(spacing_small())
            .with_min_size(Unit::px2(100.0, 50.0))
            .with_maximize(Vec2::X)
        })
        .collect_vec())
}
