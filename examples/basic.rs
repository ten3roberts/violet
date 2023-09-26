use anyhow::Context;
use flax::name;
use futures::StreamExt;
use futures_signals::signal::{Mutable, SignalExt};
use glam::{vec2, Vec2};
use image::DynamicImage;
use palette::{
    rgb::{Rgb, Rgba},
    Hsla, IntoColor, Srgb, Srgba, WithAlpha,
};
use serde::{de::Visitor, Deserialize};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};
use tracing_subscriber::EnvFilter;
use violet::{
    assets::{fs::BytesFromFile, AssetKey},
    components::{self, color, filled_rect, font_size, layout, size, text, Edges},
    input::{on_focus, on_mouse_input},
    layout::{CrossAlign, Direction, Layout},
    shapes::FilledRect,
    style::StyleExt,
    time::interval,
    unit::Unit,
    wgpu::{components::font_from_file, font::FontFromFile},
    widget::SignalWidget,
    App, Frame, Scope, StreamEffect, Widget, WidgetCollection,
};
use winit::event::ElementState;

struct MainApp;

const MARGIN: Edges = Edges::even(10.0);

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

struct Rectangle {
    color: Srgba,
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "Rectangle".into())
            .set(
                filled_rect(),
                FilledRect {
                    color: self.color,
                    fill_image: None,
                },
            )
            .set(color(), self.color);
    }
}

pub struct Button {
    normal_color: Srgba,
    pressed_color: Srgba,

    on_click: Box<dyn Send + Sync + FnMut(&Frame, winit::event::MouseButton)>,
}

impl Widget for Button {
    fn mount(mut self, scope: &mut Scope<'_>) {
        scope
            .set(
                filled_rect(),
                FilledRect {
                    color: self.normal_color,
                    fill_image: None,
                },
            )
            .set(color(), self.normal_color)
            .set(
                on_focus(),
                Box::new(move |_, entity, focus| {
                    entity.update_dedup(
                        color(),
                        if focus {
                            self.pressed_color
                        } else {
                            self.normal_color
                        },
                    );
                }),
            )
            .set(
                on_mouse_input(),
                Box::new(move |frame, _, state, button| {
                    if state == ElementState::Released {
                        (self.on_click)(frame, button);
                    }
                }),
            );
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
struct ImageFromPath {
    path: PathBuf,
}

impl AssetKey for ImageFromPath {
    type Output = DynamicImage;

    fn load(&self, _: &violet::assets::AssetCache) -> Self::Output {
        image::open(&self.path)
            .context("Failed to load image")
            .unwrap()
    }
}

struct Image<P> {
    path: P,
}

impl<P: Into<PathBuf>> Widget for Image<P> {
    fn mount(self, scope: &mut Scope) {
        let image = scope.assets_mut().load(&ImageFromPath {
            path: self.path.into(),
        });

        scope.set(name(), "Image".into()).set(
            filled_rect(),
            FilledRect {
                color: Srgba::new(1.0, 1.0, 1.0, 1.0),
                fill_image: Some(image),
            },
        );
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
            path: BytesFromFile(PathBuf::from("assets/fonts/Inter/static/Inter-Regular.ttf")),
        };

        scope
            .set(name(), "Counter".into())
            .set(font_size(), 16.0)
            .set(font_from_file(), font)
            .set(text(), "".into());
    }
}

struct Counter {}

impl Widget for Counter {
    fn mount(self, scope: &mut Scope<'_>) {
        let count = Mutable::new(0);

        List::new((
            Sized::new(
                Rectangle {
                    color: Hsla::new(0.0, 0.5, 0.5, 1.0).into_color(),
                }
                .with_margin(Edges::even(50.0)),
            )
            .with_size(Unit::px(vec2(100.0, 100.0))),
            // SignalWidget::new(
            //     count
            //         .signal()
            //         .map(|count| Text::new(format!("Count: {}", count))),
            // )
            // .with_margin(MARGIN),
            Sized::new(Button {
                normal_color: Hsla::new(200.0, 0.5, 0.5, 1.0).into_color(),
                pressed_color: Hsla::new(200.0, 0.5, 0.2, 1.0).into_color(),
                on_click: Box::new(move |_, _| {
                    *count.lock_mut() += 1;
                }),
            })
            .with_min_size(Unit::px(vec2(100.0, 50.0)))
            .with_size(Unit::px(vec2(100.0, 50.0))),
        ))
        .mount(scope);
    }
}

struct Text {
    text: String,
}

impl Text {
    fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for Text {
    fn mount(self, scope: &mut Scope) {
        let font = FontFromFile {
            path: BytesFromFile(PathBuf::from("assets/fonts/Inter/static/Inter-Regular.ttf")),
        };

        scope
            .set(font_size(), 16.0)
            .set(font_from_file(), font)
            .set(text(), self.text);
    }
}

struct ShowWorld;

impl Widget for ShowWorld {
    fn mount(self, scope: &mut Scope) {
        scope.spawn(StreamEffect::new(
            interval(Duration::from_millis(200)).enumerate(),
            move |scope: &mut Scope, (_, _)| {
                let frame = scope.frame();

                scope.set(text(), format!("{:#?}", frame.world()));
            },
        ));

        let font = FontFromFile {
            path: BytesFromFile(PathBuf::from("assets/fonts/Inter/static/Inter-Regular.ttf")),
        };

        scope
            .set(name(), "Inter Font".into())
            .set(font_size(), 10.0)
            .set(font_from_file(), font)
            .set(text(), "".into());
    }
}

#[derive(Default)]
struct List<W> {
    items: W,
    layout: Layout,
    background_color: Option<Srgba>,
}

impl<W: WidgetCollection> List<W> {
    fn new(items: W) -> Self {
        Self {
            items,
            layout: Layout::default(),
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
}

impl<W: WidgetCollection> Widget for List<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        scope
            .set_opt(
                filled_rect(),
                self.background_color.map(|bg| FilledRect {
                    // color: Hsla::new(180.0, 0.048, 0.243, 1.0).into_color(),
                    // color: Hsla::new(190.0, 0.048, 0.143, 1.0).into_color(),
                    color: bg,
                    fill_image: None,
                }),
            )
            .set(layout(), self.layout)
            .set_opt(color(), self.background_color);

        self.items.attach(scope);
    }
}

struct HelloWorld {}

impl Widget for HelloWorld {
    fn mount(self, scope: &mut Scope<'_>) {
        let font = FontFromFile {
            path: BytesFromFile("assets/fonts/Inter/static/Inter-Bold.ttf".into()),
        };

        scope
            .set(name(), "Inter Font".into())
            .set(font_size(), 24.0)
            .set(font_from_file(), font)
            .set(text(), "Hello, World!".into());
    }
}

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "MainApp".into())
            .set(size(), Unit::rel(vec2(1.0, 1.0)));

        // scope.attach(Counter);
        // scope.attach(Rectangle {
        //     color: palette::named::BLUEVIOLET.into_format().with_alpha(1.0),
        // });

        // scope.attach(
        //     Positioned::new(
        //         Sized::new(Rectangle {
        //             color: Hsla::new(270.0, 0.5, 0.5, 1.0).into_color(),
        //         })
        //         .with_size(Unit::px(vec2(100.0, 0.0)) + Unit::rel(vec2(0.0, 1.0))),
        //     )
        //     .with_offset(Unit::rel(vec2(1.0, 0.0)))
        //     // TODO: parent anchor
        //     .with_anchor(Unit::rel(vec2(1.0, 0.0))),
        // );

        // scope.spawn(FutureEffect::new(
        //     sleep(Duration::from_secs(2)),
        //     move |scope: &mut Scope, _| {
        //         scope.attach(
        //             Positioned::new(
        //                 Sized::new(Image {
        //                     path: "./assets/images/uv.png",
        //                 })
        //                 .with_size(Unit::px(vec2(400.0, 400.0))),
        //             )
        //             .with_offset(Unit::rel(Vec2::Y))
        //             .with_anchor(Unit::rel(Vec2::Y)),
        //         );
        //     },
        // ));

        scope.attach(
            Positioned::new(Sized::new(HelloWorld {}).with_size(Unit::px(vec2(400.0, 200.0))))
                .with_offset(Unit::rel(Vec2::Y))
                .with_anchor(Unit::rel(Vec2::Y)),
        );

        let list1 = List::new((
            Sized::new(Button {
                normal_color: Hsla::new(0.0, 0.5, 0.5, 1.0).into_color(),
                pressed_color: Hsla::new(0.0, 0.5, 0.2, 1.0).into_color(),
                on_click: Box::new(|_, _| {
                    tracing::info!("Clicked!");
                }),
            })
            .with_min_size(Unit::px(vec2(100.0, 100.0)))
            .with_size(Unit::px(vec2(0.0, 100.0)) + Unit::rel(vec2(0.5, 0.0)))
            .with_margin(MARGIN),
            Counter {}.with_margin(MARGIN),
            Sized::new(Rectangle {
                color: Hsla::new(30.0, 0.5, 0.5, 1.0).into_color(),
            })
            .with_size(Unit::px(vec2(100.0, 50.0)))
            .with_margin(MARGIN),
            Sized::new(Rectangle {
                color: Hsla::new(60.0, 0.5, 0.5, 1.0).into_color(),
            })
            .with_size(Unit::px(vec2(0.0, 60.0)) + Unit::rel(vec2(0.2, 0.0)))
            .with_margin(MARGIN),
            Sized::new(Rectangle {
                color: Hsla::new(90.0, 0.5, 0.5, 1.0).into_color(),
            })
            .with_min_size(Unit::px(vec2(50.0, 100.0)))
            .with_size(Unit::px(vec2(50.0, 0.0)) + Unit::rel(vec2(0.0, 0.2)))
            .with_margin(MARGIN),
        ))
        .with_background_color(Hsla::new(190.0, 0.048, 0.143, 1.0).into_color());

        let list3 = List::new((
            Sized::new(Button {
                normal_color: Hsla::new(180.0, 0.5, 0.5, 1.0).into_color(),
                pressed_color: Hsla::new(180.0, 0.5, 0.2, 1.0).into_color(),
                on_click: Box::new(|_, _| {}),
            })
            .with_size(Unit::px(vec2(80.0, 20.0)))
            .with_margin(Edges::even(2.0)),
            Sized::new(Button {
                normal_color: Hsla::new(270.0, 0.5, 0.5, 1.0).into_color(),
                pressed_color: Hsla::new(270.0, 0.5, 0.2, 1.0).into_color(),
                on_click: Box::new(|_, _| {}),
            })
            .with_size(Unit::px(vec2(100.0, 20.0)))
            .with_margin(Edges::even(2.0)),
            Sized::new(Button {
                normal_color: Hsla::new(30.0, 0.5, 0.5, 1.0).into_color(),
                pressed_color: Hsla::new(30.0, 0.5, 0.2, 1.0).into_color(),
                on_click: Box::new(|_, _| {}),
            })
            .with_size(Unit::px(vec2(120.0, 10.0)))
            .with_margin(Edges::even(2.0)),
        ))
        .with_direction(Direction::Vertical)
        .with_cross_align(CrossAlign::End);

        let list2 = List::new((
            Ticker.with_margin(MARGIN),
            // (Sized::new(Rectangle {
            //     color: Hsla::new(30.0, 0.5, 0.5, 1.0).into_color(),
            // })
            // .with_size(Unit::px(vec2(100.0, 50.0)))),
            List::new([list3])
                .with_background_color(Hsla::new(190.0, 0.048, 0.2, 1.0).into_color())
                .with_padding(MARGIN),
            Text {
                text: "Hello There!".into(),
            }
            .with_margin(MARGIN),
            Text {
                text: "General Kenobi".into(),
            }
            .with_margin(MARGIN),
            Sized::new(Rectangle {
                color: Hsla::new(60.0, 0.5, 0.5, 1.0).into_color(),
            })
            .with_min_size(Unit::px(vec2(20.0, 60.0)))
            .with_size(Unit::px(vec2(200.0, 60.0)))
            .with_margin(MARGIN),
            Sized::new(Rectangle {
                color: Hsla::new(90.0, 0.5, 0.5, 1.0).into_color(),
            })
            .with_size(Unit::px(vec2(50.0, 50.0)))
            .with_margin(MARGIN),
        ))
        .with_cross_align(CrossAlign::Center)
        .with_background_color(Hsla::new(190.0, 0.048, 0.143, 1.0).into_color());

        // let list3 = List::new((Sized::new(ShowWorld).with_size(Unit::px(vec2(200.0, 0.0))),))
        //     .with_cross_align(CrossAlign::Stretch)
        //     .with_background_color(Hsla::new(190.0, 0.048, 0.143, 1.0).into_color());

        scope.attach(
            List::new((
                LayoutTest {
                    contain_margins: false,
                },
                LayoutTest {
                    contain_margins: false,
                },
                LayoutTest {
                    contain_margins: true,
                },
                LayoutTest {
                    contain_margins: true,
                },
                LayoutTest {
                    contain_margins: false,
                },
                Text::new("Hello, World!"),
            ))
            .contain_margins(true)
            .with_direction(Direction::Vertical)
            .with_padding(Edges::even(0.0)),
            // List::new((
            //     // list3,
            //     List::new((list1, list2))
            //         .with_cross_align(CrossAlign::Stretch)
            //         .with_direction(Direction::Vertical)
            //         .with_background_color(Hsla::new(190.0, 0.048, 0.1, 1.0).into_color()),
            // ))
            // .with_cross_align(CrossAlign::Stretch)
            // .with_direction(Direction::Horizontal)
            // .with_background_color(Hsla::new(190.0, 0.048, 0.1, 1.0).into_color()),
        );
    }
}

#[derive(Debug, serde::Deserialize)]
struct ColorPalette {
    eerie_black: Shades,
    platinum: Shades,
    violet: Shades,
    teal: Shades,
    emerald: Shades,
    bronze: Shades,
    chili_red: Shades,
}

#[derive(Debug)]
struct Shades {
    shade_100: Srgba,
    shade_200: Srgba,
    shade_300: Srgba,
    shade_400: Srgba,
    shade_500: Srgba,
    shade_600: Srgba,
    shade_700: Srgba,
    shade_800: Srgba,
    shade_900: Srgba,
    default: Srgba,
}

impl<'de> Deserialize<'de> for Shades {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Shade(Srgba);

        impl<'de> Deserialize<'de> for Shade {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_str(ShadeVisitor)
            }
        }

        struct ShadeVisitor;
        impl<'de> Visitor<'de> for ShadeVisitor {
            type Value = Shade;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a hex coded rgb color")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let s: Srgb<u8> = v
                    .parse()
                    .map_err(|_| serde::de::Error::custom(format!("Invalid color {v}")))?;

                Ok(Shade(s.into_format().with_alpha(1.0)))
            }
        }

        struct ShadesVisitor;

        impl<'de> Visitor<'de> for ShadesVisitor {
            type Value = Shades;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map of shades")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut shades = [None; 10];
                while let Some(key) = map.next_key()? {
                    let shade = match key {
                        "100" => &mut shades[0],
                        "200" => &mut shades[1],
                        "300" => &mut shades[2],
                        "400" => &mut shades[3],
                        "500" => &mut shades[4],
                        "600" => &mut shades[5],
                        "700" => &mut shades[6],
                        "800" => &mut shades[7],
                        "900" => &mut shades[8],
                        "DEFAULT" => &mut shades[9],
                        v => return Err(serde::de::Error::custom(format!("Invalid shade {v}"))),
                    };

                    if shade.is_some() {
                        return Err(serde::de::Error::custom("Duplicate shade"));
                    }

                    let Shade(value) = map.next_value()?;

                    *shade = Some(value.into_format());
                }

                if let [Some(a), Some(b), Some(c), Some(d), Some(e), Some(f), Some(g), Some(h), Some(i), Some(default)] =
                    shades
                {
                    Ok(Shades {
                        shade_100: a,
                        shade_200: b,
                        shade_300: c,
                        shade_400: d,
                        shade_500: e,
                        shade_600: f,
                        shade_700: g,
                        shade_800: h,
                        shade_900: i,
                        default,
                    })
                } else {
                    Err(serde::de::Error::custom("Missing shades"))
                }
            }
        }

        deserializer.deserialize_map(ShadesVisitor)
    }
}

macro_rules! srgba {
    ($color:literal) => {{
        let [r, g, b] = color_hex::color_from_hex!($color);

        Srgba::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }};
}

struct LayoutTest {
    contain_margins: bool,
}

impl Widget for LayoutTest {
    fn mount(self, scope: &mut Scope<'_>) {
        const EERIE_BLACK: Srgba = srgba!("#222525");
        const EERIE_BLACK_300: Srgba = srgba!("#151616");
        const PLATINUM: Srgba = srgba!("#dddddf");
        const VIOLET: Srgba = srgba!("#8000ff");
        const TEAL: Srgba = srgba!("#247b7b");
        const EMERALD: Srgba = srgba!("#50c878");
        const BRONZE: Srgba = srgba!("#cd7f32");
        const CHILI_RED: Srgba = srgba!("#d34131");

        let row_2 = List::new((
            Rectangle { color: BRONZE }
                .with_margin(MARGIN)
                .with_min_size(Unit::px(vec2(100.0, 50.0)))
                .with_size(Unit::px(vec2(100.0, 50.0))),
            Rectangle { color: EMERALD }
                .with_margin(MARGIN)
                .with_min_size(Unit::px(vec2(20.0, 50.0)))
                .with_size(Unit::px(vec2(20.0, 50.0))),
        ))
        .contain_margins(self.contain_margins)
        .with_background_color(EERIE_BLACK_300)
        .with_margin(MARGIN);

        let row_1 = List::new((
            Rectangle { color: CHILI_RED }
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(200.0, 50.0))),
            row_2,
            Rectangle { color: TEAL }
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(100.0, 50.0))),
            Rectangle { color: EMERALD }
                .with_margin(MARGIN)
                .with_size(Unit::rel(vec2(0.2, 0.1))),
            Rectangle { color: TEAL }
                .with_margin(MARGIN)
                .with_size(Unit::px(vec2(50.0, 50.0))),
        ))
        .contain_margins(self.contain_margins)
        .with_background_color(EERIE_BLACK)
        .with_margin(MARGIN);

        let col_1 = List::new((row_1,))
            .contain_margins(self.contain_margins)
            .with_background_color(EERIE_BLACK_300);

        col_1.mount(scope);
    }
}

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    App::new().run(MainApp)
}
