use flax::{components::name, FetchExt, Query};
use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{Hsva, IntoColor, Srgba};
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
    let name = Mutable::new("".to_string());
    let quest = Mutable::new("".to_string());
    let color = Mutable::new(Srgba::new(0.0, 0.61, 0.388, 1.0));

    // Map a `Mutable<Srgba>` into a `StateDuplex<f32>` for each field
    let r = color.clone().map_ref(|v| &v.red, |v| &mut v.red);
    let g = color.clone().map_ref(|v| &v.green, |v| &mut v.green);
    let b = color.clone().map_ref(|v| &v.blue, |v| &mut v.blue);

    let speed = Mutable::new(None as Option<f32>);

    col((
        card(row((label("What is your name?"), TextInput::new(name)))),
        card(row((label("What is your quest?"), TextInput::new(quest)))),
        card(col((
            label("What is your favorite colour?"),
            SliderWithLabel::new(r, 0.0, 1.0).round(0.01),
            SliderWithLabel::new(g, 0.0, 1.0).round(0.01),
            SliderWithLabel::new(b, 0.0, 1.0).round(0.01),
            StreamWidget(color.stream().map(|v| {
                Rectangle::new(v)
                    .with_maximize(Vec2::X)
                    .with_min_size(Unit::px2(100.0, 100.0))
            })),
        ))),
        card(row((
            label("What is the airspeed velocity of an unladen swallow?"),
            // Fallibly parse and fill in the None at the same time using the `State` trait
            // combinators
            TextInput::new(speed.clone().prevent_feedback().filter_map(
                |v| v.map(|v| v.to_string()),
                |v| Some(v.parse::<f32>().ok()),
            )),
            StreamWidget(speed.stream().map(|v| {
                match v {
                    Some(v) => pill(Text::new(format!("{v} m/s"))),
                    None => pill(Text::new("×".to_string()))
                        .with_background(Background::new(danger_background())),
                }
            })),
        ))),
    ))
    .contain_margins(true)
}
