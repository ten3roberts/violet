use flax::{components::name, FetchExt, Query};
use futures_signals::signal::Mutable;
use glam::{vec2, Vec2};
use itertools::Itertools;
use palette::{Hsva, IntoColor};
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
    style::{
        colors::{DARK_CYAN_DEFAULT, JADE_DEFAULT, LION_DEFAULT},
        danger_item, primary_background, secondary_background, spacing_medium, spacing_small,
        Background, SizeExt, ValueOrRef,
    },
    widget::ContainerStyle,
};

struct MainApp;

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
                            Image::new("./assets/images/statue.jpg")
                                .with_min_size(Unit::px(size))
                                .with_aspect_ratio(1.0),
                        )
                        .with_margin(spacing_medium())
                    })
                    .collect_vec(),
            )
            .with_name("Images"),
            Stack::new((Text::rich([
                TextSegment::new("Violet"),
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
                    .with_color(DARK_CYAN_DEFAULT),
                TextSegment::new("\n\nfn main() { \n    println!(")
                    .with_family(FontFamily::named("JetBrainsMono Nerd Font")),
                TextSegment::new("\"Hello, world!\"")
                    .with_family(FontFamily::named("JetBrainsMono Nerd Font"))
                    .with_color(LION_DEFAULT)
                    .with_style(Style::Italic),
                TextSegment::new("); \n}")
                    .with_family(FontFamily::named("JetBrainsMono Nerd Font")),
            ])
            .with_font_size(18.0),))
            .with_margin(spacing_small())
            .with_margin(spacing_small())
            .with_background(Background::new(primary_background())),
            Stack::new(
                Text::rich([
                    TextSegment::new("The quick brown fox 🦊 jumps over the lazy dog 🐕")
                        .with_style(Style::Italic),
                ])
                .with_wrap(Wrap::Word)
                // .with_family("Inter")
                .with_font_size(18.0),
            )
            .with_margin(spacing_small())
            .with_padding(spacing_small())
            .with_background(Background::new(primary_background())),
            Stack::new((
                Rectangle::new(danger_item())
                    .with_min_size(Unit::px(vec2(100.0, 30.0)))
                    .with_size(Unit::px(vec2(50.0, 30.0))),
                Rectangle::new(danger_item())
                    .with_min_size(Unit::px(vec2(200.0, 10.0)))
                    .with_size(Unit::px(vec2(50.0, 10.0))),
                Text::new("This is some text").with_font_size(16.0),
            ))
            .with_vertical_alignment(Alignment::Center)
            .with_horizontal_alignment(Alignment::Center)
            .with_background(Background::new(secondary_background()))
            .with_padding(spacing_small())
            .with_margin(spacing_small()),
        ))
        .with_background(Background::new(secondary_background()))
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
        Stack::new((Text::new("This is an overlaid text").with_color(JADE_DEFAULT),))
            .with_style(ContainerStyle {
                background: Some(Background::new(secondary_background())),
            })
            .with_margin(spacing_small())
            .with_padding(spacing_small())
            .mount(scope)
    }
}

struct LayoutFlexTest;

impl Widget for LayoutFlexTest {
    fn mount(self, scope: &mut Scope<'_>) {
        List::new(
            (0..8)
                .map(|i| {
                    let size = vec2(50.0, 20.0);

                    Stack::new(
                        Rectangle::new(ValueOrRef::value(
                            Hsva::new(i as f32 * 30.0, 1.0, 1.0, 1.0).into_color(),
                        ))
                        .with_min_size(Unit::px(size))
                        .with_maximize(Vec2::X * i as f32),
                    )
                    .with_margin(spacing_small())
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
                            .with_color(LION_DEFAULT),
                    ])
                    .with_font_size(16.0)
                    .with_wrap(Wrap::None),
                )
                .with_margin(spacing_small())
                .with_style(ContainerStyle {
                    background: Some(Background::new(primary_background())),
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
        .with_margin(spacing_small())
        .with_padding(spacing_small())
        .with_style(ContainerStyle {
            background: Some(Background::new(primary_background())),
        });
        // row_1.mount(scope);

        List::new((row_1,))
            .contain_margins(self.contain_margins)
            .with_margin(spacing_small())
            .with_padding(spacing_small())
            .with_style(ContainerStyle {
                background: Some(Background::new(secondary_background())),
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
