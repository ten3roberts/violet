use std::usize;

use futures_signals::{
    map_ref,
    signal::{self, Mutable, SignalExt},
    signal_map::MutableSignalMap,
};

use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{Hsva, IntoColor, Srgba};
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
            EERIE_BLACK_DEFAULT, JADE_DEFAULT, LION_DEFAULT, PLATINUM_DEFAULT, REDWOOD_DEFAULT,
            ULTRA_VIOLET_DEFAULT,
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
}

fn row<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Horizontal)
}

fn column<W: WidgetCollection>(widgets: W) -> List<W> {
    List::new(widgets).with_direction(Direction::Vertical)
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

struct MainApp;

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope<'_>) {
        column((
            row(Text::new("This is a row of longer text that is wrapped. When the text wraps it will take up more vertical space in the layout, and will as such increase the overall height")),
            row((
                SizedBox::new(JADE_DEFAULT, Unit::px2(100.0, 40.0)),
                SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
            ))
            .with_background(Background::new(EERIE_BLACK_300)),
            row((
                SizedBox::new(JADE_DEFAULT, Unit::px2(100.0, 40.0)),
                SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
            ))
            .with_background(Background::new(EERIE_BLACK_300))
            .with_min_size(Unit::px2(200.0, 200.0)),
            row((
                SizedBox::new(JADE_DEFAULT, Unit::px2(100.0, 40.0)),
                SizedBox::new(REDWOOD_DEFAULT, Unit::px2(50.0, 40.0)),
            ))
            .with_background(Background::new(EERIE_BLACK_300))
            .with_max_size(Unit::px2(100.0, 200.0)),
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
