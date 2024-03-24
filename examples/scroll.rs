use flax::{components::name, FetchExt, Query};
use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{FromColor, Hsva, IntoColor, Oklch, Oklcha, Srgba};
use std::time::Duration;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet::core::{
    components::{self, rect, size, text},
    layout::{Alignment, Direction},
    style::StyleExt,
    text::{FontFamily, Style, TextSegment, Weight, Wrap},
    time::interval,
    unit::Unit,
    widget::{Button, Image, List, Rectangle, Stack, Text, WidgetExt},
    Scope, StreamEffect, Widget,
};
use violet_core::{
    state::{State, StateStream},
    style::{
        colors::{AMBER_500, EMERALD_500, TEAL_500},
        danger_background, danger_item, primary_background, secondary_background, spacing_medium,
        spacing_small, Background, SizeExt, ValueOrRef,
    },
    widget::{
        card, col, label, pill, row, ContainerStyle, SliderWithLabel, StreamWidget, TextInput,
    },
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
