use flax::{components::name, FetchExt, Query};
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
    style::Background,
    widget::{BoxSized, ContainerStyle},
    Edges,
};

struct MainApp;

macro_rules! srgba {
    ($color:literal) => {{
        let [r, g, b] = color_hex::color_from_hex!($color);

        Srgba::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }};
}

const MARGIN: Edges = Edges::even(10.0);
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

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "MainApp".into())
            .set(size(), Unit::rel(vec2(1.0, 1.0)));

        List::new((
            LayoutFlexTest,
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
                        let size = Vec2::splat(128.0 / i as f32);
                        Stack::new(
                            BoxSized::new(Image::new("./assets/images/statue.jpg"))
                                .with_min_size(Unit::px(size))
                                .with_aspect_ratio(1.0),
                        )
                        .with_style(ContainerStyle {
                            margin: MARGIN,
                            ..Default::default()
                        })
                    })
                    .collect_vec(),
            )
            .with_name("Images"),
            Stack::new((Text::rich([
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
            .with_font_size(18.0),))
            .with_style(ContainerStyle {
                background: Some(Background::new(EERIE_BLACK)),
                padding: MARGIN,
                margin: MARGIN,
            }),
            Stack::new(
                Text::rich([
                    TextSegment::new("The quick brown fox 🦊 jumps over the lazy dog 🐕")
                        .with_style(Style::Italic),
                ])
                .with_wrap(Wrap::Word)
                // .with_family("Inter")
                .with_font_size(18.0),
            )
            .with_style(ContainerStyle {
                background: Some(Background::new(EERIE_BLACK)),
                padding: MARGIN,
                margin: MARGIN,
            }),
            Stack::new((
                BoxSized::new(Rectangle::new(CHILI_RED))
                    .with_min_size(Unit::px(vec2(100.0, 30.0)))
                    .with_size(Unit::px(vec2(50.0, 30.0))),
                BoxSized::new(Rectangle::new(TEAL))
                    .with_min_size(Unit::px(vec2(200.0, 10.0)))
                    .with_size(Unit::px(vec2(50.0, 10.0))),
                Text::new("This is some text").with_font_size(16.0),
            ))
            .with_vertical_alignment(Alignment::Center)
            .with_horizontal_alignment(Alignment::Center)
            .with_background(Background::new(EERIE_BLACK_300))
            .with_padding(MARGIN)
            .with_margin(MARGIN),
        ))
        .with_style(ContainerStyle {
            background: Some(Background::new(EERIE_BLACK_600)),
            ..Default::default()
        })
        .contain_margins(true)
        .with_direction(Direction::Vertical)
        .mount(scope);
    }
}

struct DisplayWorld;

impl Widget for DisplayWorld {
    fn mount(self, scope: &mut Scope<'_>) {
        scope.spawn_effect(StreamEffect::new(
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
            // .with_margin(MARGIN)
            .mount(scope);
    }
}

struct StackTest {}

impl Widget for StackTest {
    fn mount(self, scope: &mut Scope<'_>) {
        Stack::new((Text::new("This is an overlaid text").with_color(EMERALD),))
            .with_style(ContainerStyle {
                background: Some(Background::new(EERIE_BLACK_300)),
                padding: MARGIN,
                margin: MARGIN,
            })
            .mount(scope)
    }
}

struct LayoutFlexTest;

impl Widget for LayoutFlexTest {
    fn mount(self, scope: &mut Scope<'_>) {
        List::new(
            (0..8)
                .map(|i| {
                    let size = vec2(100.0, 20.0);

                    Stack::new(
                        BoxSized::new(Rectangle::new(
                            Hsva::new(i as f32 * 30.0, 1.0, 1.0, 1.0).into_color(),
                        ))
                        .with_min_size(Unit::px(size))
                        .with_size(Unit::px(size * vec2(i as f32, 1.0))),
                    )
                    .with_style(ContainerStyle {
                        margin: MARGIN,
                        ..Default::default()
                    })
                })
                .collect_vec(),
        )
        .mount(scope)
    }
}

struct LayoutTest {
    contain_margins: bool,
}

impl Widget for LayoutTest {
    fn mount(self, scope: &mut Scope<'_>) {
        let click_count = Mutable::new(0);

        let row_1 = List::new((
            Button::new(List::new(
                Stack::new(
                    Text::rich([
                        TextSegment::new("This is "),
                        TextSegment::new("sparta")
                            .with_style(Style::Italic)
                            .with_color(BRONZE),
                    ])
                    .with_font_size(16.0)
                    .with_wrap(Wrap::None),
                )
                .with_style(ContainerStyle {
                    background: Some(Background::new(EERIE_BLACK)),
                    padding: MARGIN_SM,
                    margin: MARGIN_SM,
                }),
            ))
            .on_press({
                let click_count = click_count.clone();
                move |_, _| {
                    *click_count.lock_mut() += 1;
                }
            }),
            // row_2,
            StackTest {},
            // Button::new(Text::new("Nope, don't you dare").with_color(CHILI_RED)).on_press({
            //     let click_count = click_count.clone();
            //     move |_, _| {
            //         *click_count.lock_mut() -= 1;
            //     }
            // }),
            // Text::new("Inline text, wrapping to fit"),
            // BoxSized::new(Rectangle::new(EMERALD))
            //     .with_margin(MARGIN)
            //     .with_size(Unit::px(vec2(10.0, 80.0))),
            // Signal(
            //     click_count
            //         .signal()
            //         .map(|v| Text::new(format!("Clicked {} times", v))),
            // ),
        ))
        .contain_margins(self.contain_margins)
        .with_cross_align(Alignment::Center)
        .with_style(ContainerStyle {
            background: Some(Background::new(EERIE_BLACK)),
            padding: MARGIN,
            margin: MARGIN,
        });
        // row_1.mount(scope);

        List::new((row_1,))
            .contain_margins(self.contain_margins)
            .with_style(ContainerStyle {
                background: Some(Background::new(EERIE_BLACK_300)),
                padding: MARGIN,
                margin: MARGIN,
            })
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

    violet_wgpu::App::new().run(MainApp)
}
