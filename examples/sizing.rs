use std::usize;

use futures_signals::{
    map_ref,
    signal::{self, Mutable, SignalExt},
    signal_map::MutableSignalMap,
};

use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{num::Round, Hsva, IntoColor, Srgba};
use tracing_subscriber::{layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use tracing_tree::HierarchicalLayer;

use futures::stream::StreamExt;
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
    widget::{BoxSized, Button, ButtonStyle, ContainerStyle, Positioned, SliderWithLabel},
    WidgetCollection,
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
            Signal::new(size.signal().map(|size| StackSizing { size })),
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
        row((
            card(column((
                label("Unconstrained list"),
                row((
                    SizedBox::new(JADE_DEFAULT, Unit::px(self.size)),
                    SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
                ))
                .with_background(bg),
            ))),
            card(column((
                label("Constrained list with min size"),
                row((
                    SizedBox::new(JADE_DEFAULT, Unit::px(self.size)),
                    SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
                ))
                .with_background(bg)
                .with_min_size(Unit::px2(100.0, 100.0)),
            ))),
            card(column((
                label("Constrained list with max size"),
                row((
                    SizedBox::new(JADE_DEFAULT, Unit::px(self.size)),
                    SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
                ))
                .with_background(bg)
                .with_max_size(Unit::px2(100.0, 100.0)),
            ))),
        ))
        .mount(scope)
    }
}

struct StackSizing {
    size: Vec2,
}

impl Widget for StackSizing {
    fn mount(self, scope: &mut Scope<'_>) {
        let bg = Background::new(JADE_100);

        row((
            card(column((
                label("Unconstrained list"),
                centered((
                    SizedBox::new(JADE_DEFAULT, Unit::px(self.size)),
                    SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
                ))
                .with_background(bg),
            ))),
            card(column((
                label("Constrained list with min size"),
                centered((
                    SizedBox::new(JADE_DEFAULT, Unit::px(self.size)),
                    SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
                ))
                .with_background(bg)
                .with_min_size(Unit::px2(100.0, 100.0)),
            ))),
            card(column((
                label("Constrained list with max size"),
                centered((
                    SizedBox::new(JADE_DEFAULT, Unit::px(self.size)),
                    SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
                ))
                .with_background(bg)
                .with_max_size(Unit::px2(100.0, 100.0)),
            ))),
        ))
        .mount(scope)
    }
}
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
        Stack::new((
            Rectangle::new(self.color).with_size(self.size),
            column((
                Text::new(format!("{}", self.size.px)),
                Text::new(format!("{}", self.size.rel)),
            )),
        ))
        .mount(scope)
    }
}
