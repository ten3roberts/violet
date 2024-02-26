use std::{
    time::{Duration, Instant},
    usize,
};

use flax::components::name;
use futures_signals::{
    map_ref,
    signal::{self, Mutable, SignalExt},
    signal_map::MutableSignalMap,
};

use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{num::Round, Hsva, IntoColor, Srgba, WithAlpha};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;

use futures::stream::{StreamExt, StreamFuture};
use violet::core::{
    components::{self, screen_rect, Edges, Rect},
    editor::{self, EditAction, EditorAction, TextEditor},
    input::{focusable, on_char_typed, on_keyboard_input, on_mouse_input},
    layout::{Alignment, Direction},
    style::StyleExt,
    text::{LayoutGlyphs, TextSegment},
    to_owned,
    unit::Unit,
    widget::{List, NoOp, Rectangle, Signal, Stack, Text, WidgetExt},
    Scope, Widget,
};
use violet_core::{
    components::size,
    input::{focus_sticky, ElementState, VirtualKeyCode},
    style::{
        self,
        colors::{
            DARK_CYAN_DEFAULT, EERIE_BLACK_300, EERIE_BLACK_400, EERIE_BLACK_600,
            EERIE_BLACK_DEFAULT, JADE_100, JADE_400, JADE_DEFAULT, LION_DEFAULT, PLATINUM_DEFAULT,
            REDWOOD_DEFAULT, ULTRA_VIOLET_DEFAULT,
        },
        Background,
    },
    time::{interval, Interval},
    widget::{BoxSized, Button, ButtonStyle, ContainerStyle, Positioned, SliderWithLabel},
    StreamEffect, WidgetCollection,
};

const MARGIN: Edges = Edges::even(8.0);
const MARGIN_SM: Edges = Edges::even(4.0);

fn label(text: impl Into<String>) -> Stack<Text> {
    Stack::new(Text::new(text.into()))
        .with_padding(MARGIN_SM)
        .with_margin(MARGIN_SM)
        .with_background(Background::new(EERIE_BLACK_400))
}

fn row<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Horizontal)
}

fn column<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Vertical)
}

fn centered<W>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_horizontal_alignment(Alignment::Center)
        .with_vertical_alignment(Alignment::Center)
}

fn card<W>(widget: W) -> Stack<W> {
    Stack::new(widget)
        .with_background(Background::new(EERIE_BLACK_400))
        .with_padding(MARGIN)
        .with_margin(MARGIN)
}

fn pill(widget: impl Widget) -> impl Widget {
    Stack::new(widget).with_style(ContainerStyle {
        background: Some(Background::new(EERIE_BLACK_300)),
        padding: MARGIN,
        margin: MARGIN,
    })
}

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

    violet_wgpu::App::new().run(MainApp)
}

struct Vec2Editor {
    value: Mutable<Vec2>,
    x_label: String,
    y_label: String,
}

impl Vec2Editor {
    fn new(value: Mutable<Vec2>, x_label: impl Into<String>, y_label: impl Into<String>) -> Self {
        Self {
            value,
            x_label: x_label.into(),
            y_label: y_label.into(),
        }
    }
}

impl Widget for Vec2Editor {
    fn mount(self, scope: &mut Scope<'_>) {
        let value = self.value;

        column((
            row((
                label(self.x_label),
                SliderWithLabel::new_with_transform(
                    value.clone(),
                    0.0,
                    200.0,
                    |v| v.x,
                    |v, x| v.x = x.round(),
                ),
            )),
            row((
                label(self.y_label),
                SliderWithLabel::new_with_transform(
                    value.clone(),
                    0.0,
                    200.0,
                    |v| v.y,
                    |v, y| v.y = y.round(),
                ),
            )),
        ))
        .mount(scope)
    }
}
struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        let size = Mutable::new(vec2(100.0, 100.0));

        column((
            card(column((
                Vec2Editor::new(size.clone(), "width", "height"),
                Signal::new(size.signal().map(|size| label(format!("Rectangle size: {size}")))),
            ))),
            row((label("This is a row of longer text that is wrapped. When the text wraps it will take up more vertical space in the layout, and will as such increase the overall height"), label(":P"))),
            Signal::new(size.signal().map(|size| FlowSizing { size })),
            // AnimatedSize,
        ))
        .contain_margins(true)
        .with_background(Background::new(EERIE_BLACK_DEFAULT))
        .mount(scope)
    }
}

struct FlowSizing {
    size: Vec2,
}

impl Widget for FlowSizing {
    fn mount(self, scope: &mut Scope<'_>) {
        let bg = Background::new(JADE_100);

        let content = (
            SizedBox::new(JADE_DEFAULT, Unit::px(self.size)).with_name("JADE"),
            SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)).with_name("REDWOOD"),
            AnimatedSize,
        );

        column((
            row((
                card(column((
                    label("Unconstrained list"),
                    row(content.clone()).with_background(bg),
                ))),
                card(column((
                    label("Constrained list with min size"),
                    row(content.clone())
                        .with_background(bg)
                        .with_min_size(Unit::px2(100.0, 100.0)),
                ))),
                card(column((
                    label("Constrained list with max size"),
                    row(content.clone())
                        .with_background(bg)
                        .with_max_size(Unit::px2(100.0, 100.0)),
                ))),
            )),
            row((
                card(column((
                    label("Unconstrained list"),
                    centered(content.clone()).with_background(bg),
                ))),
                card(column((
                    label("Constrained list with min size"),
                    centered(content.clone())
                        .with_background(bg)
                        .with_min_size(Unit::px2(100.0, 100.0)),
                ))),
                card(column((
                    label("Constrained list with max size"),
                    centered(content.clone())
                        .with_background(bg)
                        .with_max_size(Unit::px2(100.0, 100.0)),
                ))),
            )),
        ))
        .mount(scope)
    }
}

#[derive(Debug, Clone)]
struct SizedBox {
    color: Srgba,
    size: Unit<Vec2>,
}

impl SizedBox {
    fn new(color: Srgba, size: Unit<Vec2>) -> Self {
        Self { color, size }
    }
}

impl Widget for SizedBox {
    fn mount(self, scope: &mut Scope<'_>) {
        // Stack::new((
        Rectangle::new(self.color)
            .with_size(self.size)
            //     column((
            //         Text::new(format!("{}", self.size.px)),
            //         Text::new(format!("{}", self.size.rel)),
            //     )),
            // ))
            .mount(scope)
    }
}

#[derive(Debug, Clone)]
pub struct AnimatedSize;

impl Widget for AnimatedSize {
    fn mount(self, scope: &mut Scope<'_>) {
        scope.set(name(), "AnimatedBox".into());
        let start = Instant::now();
        scope.spawn_effect(StreamEffect::new(
            interval(Duration::from_millis(100)),
            move |scope: &mut Scope<'_>, deadline: Instant| {
                let t = (deadline - start).as_secs_f32();

                let size = vec2(t.sin() * 50.0, (t * 2.5).cos() * 50.0) + vec2(100.0, 100.0);

                scope.set(components::size(), Unit::px(size));
            },
        ));

        Rectangle::new(LION_DEFAULT).mount(scope)
    }
}
