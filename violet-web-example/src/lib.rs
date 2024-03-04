use glam::Vec2;
use tracing_subscriber::{
    filter::LevelFilter, fmt::format::Pretty, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};
use tracing_web::{performance_layer, MakeWebConsoleWriter};
use violet::{
    core::{
        components,
        layout::{Alignment, Direction},
        style::{
            colors::{
                EERIE_BLACK_400, EERIE_BLACK_DEFAULT, JADE_200, JADE_DEFAULT, LION_DEFAULT,
                REDWOOD_DEFAULT,
            },
            Background,
        },
        text::Wrap,
        unit::Unit,
        widget::{List, Rectangle, SignalWidget, SliderWithLabel, Stack, Text, WidgetExt},
        Edges, Scope, Widget, WidgetCollection,
    },
    flax::components::name,
    futures_signals::signal::{Mutable, SignalExt},
    glam::vec2,
    palette::Srgba,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn run() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .without_time()
        .with_writer(MakeWebConsoleWriter::new())
        .with_filter(LevelFilter::INFO);

    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();

    console_error_panic_hook::set_once();

    violet::wgpu::App::new().run(MainApp).unwrap();
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
                SignalWidget::new(size.signal().map(|size| label(format!("Rectangle size: {size}")))),
            ))),
            row((label("This is a row of longer text that is wrapped. When the text wraps it will take up more vertical space in the layout, and will as such increase the overall height"), card(Text::new(":P").with_wrap(Wrap::None)))),
            SignalWidget::new(size.signal().map(|size| FlowSizing { size })),
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
        let bg = Background::new(JADE_200);

        let content = (
            SizedBox::new(JADE_DEFAULT, Unit::px(self.size)).with_name("EMERALD"),
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
        scope.set(
            components::on_animation_frame(),
            Box::new(move |_, entity, t| {
                let t = t.as_secs_f32();

                let size = vec2(t.sin() * 50.0, (t * 2.5).cos() * 50.0) + vec2(100.0, 100.0);
                entity.update_dedup(components::size(), Unit::px(size));
            }),
        );

        Rectangle::new(LION_DEFAULT).mount(scope)
    }
}
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

const MARGIN: Edges = Edges::even(8.0);
const MARGIN_SM: Edges = Edges::even(4.0);
