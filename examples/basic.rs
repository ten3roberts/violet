use flax::{components::name, FetchExt, Query};
use futures_signals::signal::{Mutable, SignalExt};
use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{Hsva, IntoColor, Srgba};
use std::time::Duration;
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};
use tracing_tree::HierarchicalLayer;
use violet::{
    components::{self, layout, rect, size, text, Edges},
    layout::{CrossAlign, Direction},
    style::StyleExt,
    text::{FontFamily, Style, TextSegment, Weight, Wrap},
    time::interval,
    unit::Unit,
    widget::{
        Button, ContainerExt, Image, List, Rectangle, Signal, Stack, StreamWidget, Text, WidgetExt,
    },
    App, Scope, StreamEffect, Widget,
};

struct MainApp;

macro_rules! srgba {
    ($color:literal) => {{
        let [r, g, b] = color_hex::color_from_hex!($color);

        Srgba::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }};
}

const MARGIN: Edges = Edges::even(15.0);
const MARGIN_SM: Edges = Edges::even(5.0);

pub const EERIE_BLACK: Srgba = srgba!("#222525");
pub const EERIE_BLACK_300: Srgba = srgba!("#151616");
pub const EERIE_BLACK_400: Srgba = srgba!("#1b1e1e");
pub const EERIE_BLACK_600: Srgba = srgba!("#4c5353");
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

// impl<K> Asset<DynamicImage> for K
// where
//     K: AssetKey<Bytes>,
// {
//     type Error = K::Error;

//     fn load(self, _: &violet::assets::AssetCache) -> Result<DynamicImage, ImageError> {
//         image::load_from_memory(&self.0)
//     }
// }

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "MainApp".into())
            .set(size(), Unit::rel(vec2(1.0, 1.0)));

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
                depth: 2,
            }
            .with_name("LayoutText 3"),
            LayoutTest {
                contain_margins: false,
                depth: 2,
            }
            .with_name("LayoutText 2"),
            List::new(
                (1..=4)
                    .map(|i| {
                        let size = Vec2::splat(128.0 / i as f32);
                        Image::new("./assets/images/statue.jpg")
                            .with_min_size(Unit::px(size))
                            .with_size(Unit::px(size))
                            .with_margin(MARGIN)
                    })
                    .collect_vec(),
            )
            .with_name("Images"),
            Stack::new((
                Rectangle::new(EMERALD).with_size(Unit::px(vec2(100.0, 20.0))),
                Text::rich([
                    TextSegment::new("Violet").with_color(VIOLET),
                    TextSegment::new(" now has support for "),
                    TextSegment::new("rich ").with_style(Style::Italic),
                    TextSegment::new("text. I wanted to "),
                    TextSegment::new("emphasize").with_style(Style::Italic),
                    TextSegment::new(" that, "),
                    TextSegment::new("(and put something in bold)")
                        .with_family("Inter")
                        .with_weight(Weight::BOLD),
                    TextSegment::new(", and").with_style(Style::Italic),
                    TextSegment::new(" also show off the different font loadings: \n"),
                    TextSegment::new("Monospace:")
                        .with_family(FontFamily::named("JetBrainsMono Nerd Font"))
                        .with_color(TEAL),
                    TextSegment::new("\n\nfn main() { \n    println!(")
                        .with_family(FontFamily::named("JetBrainsMono Nerd Font")),
                    TextSegment::new("\"Hello, world!\"")
                        .with_family(FontFamily::named("JetBrainsMono Nerd Font"))
                        .with_color(BRONZE)
                        .with_style(Style::Italic),
                    TextSegment::new("); \n}")
                        .with_family(FontFamily::named("JetBrainsMono Nerd Font")),
                ])
                .with_font_size(28.0)
                .with_margin(MARGIN),
            ))
            .with_vertical_alignment(CrossAlign::End)
            .with_background(Rectangle::new(EERIE_BLACK))
            .with_padding(MARGIN)
            .with_margin(MARGIN),
            Stack::new((Text::rich([TextSegment::new(
                "The quick brown fox 🦊 jumps over the lazy dog 🐕",
            )
            .with_style(cosmic_text::Style::Italic)])
            .with_wrap(Wrap::Glyph)
            // .with_family("Inter")
            .with_font_size(32.0)
            .with_margin(MARGIN),))
            .with_background(Rectangle::new(EERIE_BLACK))
            .with_padding(MARGIN)
            .with_margin(MARGIN),
            Stack::new((
                Rectangle::new(CHILI_RED)
                    .with_min_size(Unit::px(vec2(100.0, 30.0)))
                    .with_size(Unit::px(vec2(100.0, 30.0))),
                Rectangle::new(TEAL)
                    .with_min_size(Unit::px(vec2(200.0, 10.0)))
                    .with_size(Unit::px(vec2(100.0, 10.0)))
                    .with_margin(MARGIN),
                Text::new("This is some text")
                    .with_font_size(16.0)
                    .with_margin(MARGIN),
                // Rectangle::n#ew(EERIE_BLACK).with_size(Unit::rel(vec2(1.0, 1.0))),
            ))
            .with_background(Rectangle::new(EERIE_BLACK))
            .with_margin(MARGIN),
            // Text::new("Foo"),
            // DisplayWorld,
            // Rectangle::new(CHILI_RED).with_size(Unit::px(vec2(100.0, 10.0))),
        ))
        .with_background(Rectangle::new(EERIE_BLACK_600))
        .contain_margins(true)
        .with_direction(Direction::Vertical)
        .mount(scope);
    }
}

struct DisplayWorld;

impl Widget for DisplayWorld {
    fn mount(self, scope: &mut Scope<'_>) {
        scope.spawn(StreamEffect::new(
            interval(Duration::from_secs(1)),
            |scope: &mut Scope<'_>, _| {
                let world = &scope.frame().world;
                let s = Query::new((components::color(), rect().opt()))
                    .borrow(world)
                    .iter()
                    .map(|v| format!("{v:?}"))
                    .join("\n");

                scope.set(
                    text(),
                    vec![TextSegment::new(s).with_family(FontFamily::Monospace)],
                );
            },
        ));

        Text::new("")
            .with_font_size(12.0)
            .with_margin(MARGIN)
            .mount(scope);
    }
}

struct StackTest {}

impl Widget for StackTest {
    fn mount(self, scope: &mut Scope<'_>) {
        // Text::new("This is an overlaid text")
        //     .with_color(EMERALD)
        //     .mount(scope)
        Stack::new((Text::new("This is an overlaid text").with_color(EMERALD),))
            .with_background(Rectangle::new(EERIE_BLACK_300))
            .with_padding(MARGIN)
            .with_margin(MARGIN)
            .mount(scope)
    }
}

struct LayoutTest {
    contain_margins: bool,
    depth: usize,
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
        .with_background(Rectangle::new(EERIE_BLACK_300))
        .with_margin(MARGIN);

        let click_count = Mutable::new(0);

        let row_1 = List::new((
            Button::new(List::new((
                Stack::new(
                    Text::rich([
                        TextSegment::new("This is "),
                        TextSegment::new("sparta")
                            .with_style(Style::Italic)
                            .with_color(BRONZE),
                    ])
                    .with_font_size(32.0)
                    .with_wrap(Wrap::None),
                )
                .with_background(Rectangle::new(EERIE_BLACK))
                .with_padding(MARGIN_SM),
                if self.depth > 0 {
                    Some(Self {
                        contain_margins: true,
                        depth: self.depth - 1,
                    })
                } else {
                    None
                },
            )))
            .on_press({
                let click_count = click_count.clone();
                move |_, _| {
                    *click_count.lock_mut() += 1;
                }
            })
            .with_background(Image::new("./assets/images/statue.jpg"))
            .with_background_color(Srgba::new(1.0, 1.0, 1.0, 1.0))
            .with_pressed_color(EERIE_BLACK)
            .with_padding(MARGIN)
            .with_margin(MARGIN)
            .with_size(Unit::px(vec2(800.0, 50.0))),
            row_2,
            StackTest {},
            Button::new(Text::new("Nope, don't you dare").with_color(CHILI_RED))
                .on_press({
                    let click_count = click_count.clone();
                    move |_, _| {
                        *click_count.lock_mut() -= 1;
                    }
                })
                .with_padding(MARGIN_SM)
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(200.0, 50.0))),
            Text::new("Inline text, wrapping to fit").with_margin(MARGIN),
            Rectangle::new(EMERALD)
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(10.0, 80.0))),
            Signal(
                click_count
                    .signal()
                    .map(|v| Text::new(format!("Clicked {} times", v))),
            )
            .with_margin(MARGIN),
        ))
        .contain_margins(self.contain_margins)
        .with_cross_align(CrossAlign::Center)
        .with_background(Rectangle::new(EERIE_BLACK))
        .with_margin(MARGIN);

        // row_1.mount(scope);

        List::new((row_1,))
            .contain_margins(self.contain_margins)
            .with_background(Rectangle::new(EERIE_BLACK_300))
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
