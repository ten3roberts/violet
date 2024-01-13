use flax::components::name;
use futures::StreamExt;
use futures_signals::signal::Mutable;
use glam::{vec2, Vec2};
use image::{DynamicImage, ImageError};
use itertools::Itertools;
use palette::{Hsla, Hsva, IntoColor, Srgba};
use std::{path::PathBuf, time::Duration};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet::{
    assets::AssetKey,
    components::{
        self, color, draw_shape, font_family, font_size, layout, shape_rectangle, size, text,
        Edges, FontFamily,
    },
    layout::{CrossAlign, Direction, FlowLayout, Layout, StackLayout},
    style::StyleExt,
    time::interval,
    unit::Unit,
    wgpu::font::FontFromFile,
    widget::WidgetExt,
    widgets::{Button, Rectangle},
    App, Scope, StreamEffect, Widget, WidgetCollection,
};

struct MainApp;

macro_rules! srgba {
    ($color:literal) => {{
        let [r, g, b] = color_hex::color_from_hex!($color);

        Srgba::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }};
}

const MARGIN: Edges = Edges::even(15.0);

pub const EERIE_BLACK: Srgba = srgba!("#222525");
pub const EERIE_BLACK_300: Srgba = srgba!("#151616");
pub const PLATINUM: Srgba = srgba!("#dddddf");
pub const VIOLET: Srgba = srgba!("#8000ff");
pub const TEAL: Srgba = srgba!("#247b7b");
pub const EMERALD: Srgba = srgba!("#50c878");
pub const BRONZE: Srgba = srgba!("#cd7f32");
pub const CHILI_RED: Srgba = srgba!("#d34131");

struct Sized<W> {
    min_size: Unit<Vec2>,
    size: Unit<Vec2>,
    widget: W,
}

impl<W> Sized<W> {
    pub fn new(widget: W) -> Self {
        Self {
            min_size: Unit::ZERO,
            size: Unit::ZERO,
            widget,
        }
    }

    /// Sets the preferred size of a widget
    pub fn with_size(mut self, size: Unit<Vec2>) -> Self {
        self.size = size;
        self
    }

    /// Sets the minimum size of a widget
    pub fn with_min_size(mut self, size: Unit<Vec2>) -> Self {
        self.min_size = size;
        self
    }
}

impl<W> Widget for Sized<W>
where
    W: Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);

        scope.set(components::size(), self.size);
        scope.set(components::min_size(), self.min_size);
    }
}

struct Positioned<W> {
    offset: Unit<Vec2>,
    anchor: Unit<Vec2>,
    widget: W,
}

impl<W> Positioned<W> {
    pub fn new(widget: W) -> Self {
        Self {
            offset: Unit::ZERO,
            anchor: Unit::ZERO,
            widget,
        }
    }

    /// Sets the anchor point of the widget
    pub fn with_anchor(mut self, anchor: Unit<Vec2>) -> Self {
        self.anchor = anchor;
        self
    }

    /// Offsets the widget relative to its original position
    pub fn with_offset(mut self, offset: Unit<Vec2>) -> Self {
        self.offset = offset;
        self
    }
}

impl<W> Widget for Positioned<W>
where
    W: Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);

        scope.set(components::anchor(), self.anchor);
        scope.set(components::offset(), self.offset);
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
struct ImageFromPath {
    path: PathBuf,
}

impl AssetKey for ImageFromPath {
    type Output = DynamicImage;
    type Error = ImageError;

    fn load(self, _: &violet::assets::AssetCache) -> Result<Self::Output, ImageError> {
        image::open(self.path)
    }
}

struct Image<P> {
    path: P,
}

impl<P: Into<PathBuf>> Widget for Image<P> {
    fn mount(self, scope: &mut Scope) {
        let image = scope
            .assets_mut()
            .try_load(&ImageFromPath {
                path: self.path.into(),
            })
            .unwrap();

        scope
            .set(name(), "Image".into())
            .set(draw_shape(shape_rectangle()), ())
            .set(components::image(), image);
    }
}

struct Ticker;

impl Widget for Ticker {
    fn mount(self, scope: &mut Scope) {
        scope.spawn(StreamEffect::new(
            interval(Duration::from_millis(50)).enumerate(),
            move |scope: &mut Scope, (i, _)| {
                scope.set(text(), format!("Tick: {:#?}", i % 64));
            },
        ));

        let font = FontFromFile {
            path: PathBuf::from("assets/fonts/Inter/static/Inter-Regular.ttf"),
        };

        scope
            .set(name(), "Counter".into())
            .set(font_size(), 16.0)
            .set(font_family(), "Inter/static/Inter-Regular.ttf".into())
            .set(text(), "".into());
    }
}

struct Counter {}

impl Widget for Counter {
    fn mount(self, scope: &mut Scope<'_>) {
        let count = Mutable::new(0);

        List::new((
            Sized::new(
                Rectangle::new(Hsla::new(0.0, 0.5, 0.5, 1.0).into_color())
                    .with_margin(Edges::even(50.0)),
            )
            .with_size(Unit::px(vec2(100.0, 100.0))),
            // SignalWidget::new(
            //     count
            //         .signal()
            //         .map(|count| Text::new(format!("Count: {}", count))),
            // )
            // .with_margin(MARGIN),
            Sized::new(Button::new(
                Hsla::new(200.0, 0.5, 0.5, 1.0).into_color(),
                Hsla::new(200.0, 0.5, 0.2, 1.0).into_color(),
                Box::new(move |_, _| {
                    *count.lock_mut() += 1;
                }),
            ))
            .with_min_size(Unit::px(vec2(100.0, 50.0)))
            .with_size(Unit::px(vec2(100.0, 50.0))),
        ))
        .mount(scope);
    }
}

struct Text {
    color: Option<Srgba>,
    text: String,
    font: FontFamily,
    font_size: f32,
}

impl Text {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: None,
            font_size: 16.0,
            font: "Inter/static/Inter-Regular.ttf".into(),
        }
    }

    /// Set the font
    pub fn with_font(mut self, font: impl Into<FontFamily>) -> Self {
        self.font = font.into();
        self
    }

    /// Set the font_size
    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Set the text color
    pub fn with_color(mut self, color: Srgba) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for Text {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(font_size(), self.font_size)
            .set(font_family(), self.font)
            .set(text(), self.text)
            .set_opt(color(), self.color);
    }
}

#[derive(Default)]
struct List<W> {
    items: W,
    layout: FlowLayout,
    background_color: Option<Srgba>,
}

impl<W: WidgetCollection> List<W> {
    fn new(items: W) -> Self {
        Self {
            items,
            layout: FlowLayout::default(),
            background_color: None,
        }
    }

    /// Set the List's direction
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.layout.direction = direction;
        self
    }

    /// Set the List's cross axis alignment
    pub fn with_cross_align(mut self, cross_align: CrossAlign) -> Self {
        self.layout.cross_align = cross_align;
        self
    }

    /// Set the List's background color
    pub fn with_background_color(mut self, background_color: Srgba) -> Self {
        self.background_color = Some(background_color);
        self
    }

    pub fn contain_margins(mut self, enable: bool) -> Self {
        self.layout.contain_margins = enable;
        self
    }

    fn with_stretch(mut self, enable: bool) -> Self {
        self.layout.stretch = enable;
        self
    }
}

impl<W: WidgetCollection> Widget for List<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        if let Some(background_color) = self.background_color {
            scope
                .set(draw_shape(shape_rectangle()), ())
                .set(color(), background_color);
        }
        scope
            .set(layout(), Layout::Flow(self.layout))
            .set_opt(color(), self.background_color);

        self.items.attach(scope);
    }
}

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "MainApp".into())
            .set(size(), Unit::rel(vec2(1.0, 1.0)));

        Stack::new(
            List::new((
                List::new(
                    (0..4)
                        .map(|i| {
                            let size = vec2(50.0, 50.0);

                            Rectangle::new(Hsva::new(i as f32 * 30.0, 1.0, 1.0, 1.0).into_color())
                                .with_min_size(Unit::px(size))
                                .with_size(Unit::px(size * vec2(2.0, 1.0)))
                        })
                        .collect_vec(),
                ),
                LayoutTest {
                    contain_margins: true,
                }
                .with_name("LayoutText 3"),
                LayoutTest {
                    contain_margins: false,
                }
                .with_name("LayoutText 2"),
                List::new(
                    (1..=4)
                        .map(|i| {
                            Image {
                                path: "./assets/images/statue.jpg",
                            }
                            .with_min_size(Unit::px(vec2(256.0 / i as f32, 256.0 / i as f32)))
                            .with_size(Unit::px(vec2(256.0 / i as f32, 256.0 / i as f32)))
                            .with_margin(MARGIN)
                        })
                        .collect_vec(),
                )
                .with_name("Images"),
                Stack::new((Text::new(
                    "The quick brown fox 🦊 jumps over the lazy dog 🐕 fi fO t f-t ===",
                )
                .with_font("Inter/static/Inter-Bold.ttf")
                .with_font_size(32.0)
                .with_margin(MARGIN),))
                .with_background(EERIE_BLACK),
                Stack::new((
                    Text::new(" -> <==========> ======= != <$> ~~>")
                        .with_font("Inter/static/Inter-Bold.ttf")
                        .with_font_size(32.0)
                        .with_margin(MARGIN),
                    Rectangle::new(EERIE_BLACK)
                        .with_size(Unit::rel(vec2(1.0, 0.0)) + Unit::px(vec2(0.0, 50.0))),
                )),
            ))
            .contain_margins(true)
            .with_direction(Direction::Vertical)
            .with_padding(Edges::even(5.0)),
        )
        .mount(scope);

        // scope.attach(LayoutTest {
        //     contain_margins: true,
        // });
        // scope.attach(
        //     List::new((
        //         LayoutTest {
        //             contain_margins: true,
        //         },
        //         LayoutTest {
        //             contain_margins: false,
        //         },
        //         List::new(
        //             (1..=4)
        //                 .map(|i| {
        //                     Image {
        //                         path: "./assets/images/statue.jpg",
        //                     }
        //                     .with_min_size(Unit::px(vec2(256.0 / i as f32, 256.0 / i as f32)))
        //                     .with_size(Unit::px(vec2(256.0 / i as f32, 256.0 / i as f32)))
        //                     .with_margin(MARGIN)
        //                 })
        //                 .collect_vec(),
        //         ),
        //         Stack {
        //             items: (
        //                 Text::new("Hello, World!")
        //                     .with_font("assets/fonts/Inter/static/Inter-Bold.ttf")
        //                     .with_font_size(32.0)
        //                     .with_margin(MARGIN),
        //                 Rectangle::new(EERIE_BLACK)
        //                     .with_size(Unit::rel(vec2(1.0, 0.0)) + Unit::px(vec2(0.0, 50.0))),
        //             ),
        //         },
        //     ))
        //     .contain_margins(true)
        //     .with_direction(Direction::Vertical)
        //     .with_padding(Edges::even(5.0)),
        // );
    }
}

struct Stack<W> {
    items: W,
    background: Option<Srgba>,
}

impl<W> Stack<W> {
    fn new(items: W) -> Self {
        Self {
            items,
            background: None,
        }
    }

    fn with_background(mut self, background: Srgba) -> Self {
        self.background = Some(background);
        self
    }
}

impl<W> Widget for Stack<W>
where
    W: WidgetCollection,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.items.attach(scope);

        if let Some(background) = self.background {
            scope
                .set(draw_shape(shape_rectangle()), ())
                .set(color(), background);
        }

        scope.set(
            layout(),
            Layout::Stack(StackLayout {
                horizontal_alignment: CrossAlign::Center,
                vertical_alignment: CrossAlign::Start,
            }),
        );
    }
}

struct StackTest {}

impl Widget for StackTest {
    fn mount(self, scope: &mut Scope<'_>) {
        // Text::new("This is an overlaid text")
        //     .with_color(EMERALD)
        //     .mount(scope)
        scope.set(layout(), Layout::Stack(Default::default()));
        scope.attach(Text::new("This is an overlaid text").with_color(EMERALD));

        Rectangle::new(EERIE_BLACK_300)
            .with_margin(Edges::even(10.0))
            .with_padding(Edges::even(5.0))
            .mount(scope);
    }
}

struct LayoutTest {
    contain_margins: bool,
}

impl Widget for LayoutTest {
    fn mount(self, scope: &mut Scope<'_>) {
        let row_2 = List::new((
            Rectangle::new(BRONZE)
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(100.0, 20.0))),
            Rectangle::new(EMERALD)
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(20.0, 20.0))),
        ))
        .with_direction(Direction::Vertical)
        .contain_margins(self.contain_margins)
        .with_background_color(EERIE_BLACK_300)
        .with_margin(MARGIN);

        let row_1 = List::new((
            Button::new(CHILI_RED, BRONZE, Box::new(|_, _| {}))
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(800.0, 50.0))),
            row_2,
            StackTest {},
            Button::new(CHILI_RED, BRONZE, Box::new(|_, _| {}))
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(200.0, 50.0))),
            Text::new("Inline text, wrapping to fit").with_margin(MARGIN),
            Rectangle::new(EMERALD)
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(10.0, 80.0))),
        ))
        .contain_margins(self.contain_margins)
        .with_cross_align(CrossAlign::Center)
        .with_background_color(EERIE_BLACK)
        .with_margin(MARGIN);

        // row_1.mount(scope);

        List::new((row_1,))
            .contain_margins(self.contain_margins)
            .with_background_color(EERIE_BLACK_300)
            .mount(scope);
    }
}

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

    App::new().run(MainApp)
}
