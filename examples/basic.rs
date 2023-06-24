use anyhow::Context;
use flax::name;
use futures::StreamExt;
use glam::{vec2, Vec2};
use image::DynamicImage;
use palette::{Hsla, IntoColor, Srgba};
use std::{path::PathBuf, time::Duration};
use tracing_subscriber::EnvFilter;
use violet::{
    assets::{fs::BytesFromFile, AssetKey},
    components::{
        self, color, filled_rect, layout, local_position, margin, padding, rect, screen_position,
        size, text, Edges,
    },
    layout::{CrossAlign, Direction, Layout},
    shapes::FilledRect,
    time::interval,
    unit::Unit,
    wgpu::{
        components::{font_from_file, model_matrix},
        font::{FontAtlas, FontFromBytes, FontFromFile},
    },
    App, Scope, StreamEffect, Widget, WidgetCollection,
};

struct MainApp;

struct Counter;
impl Widget for Counter {
    fn mount(self, scope: &mut Scope) {
        scope.spawn(StreamEffect::new(
            interval(Duration::from_millis(200)).enumerate(),
            move |scope: &mut Scope, (i, _)| {
                scope.set(name(), format!("Counter: {:#?}", i));
            },
        ));
    }
}

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
    margin: Edges,
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "Rectangle".into())
            .set(margin(), self.margin)
            .set_default(screen_position())
            .set_default(local_position())
            .set_default(model_matrix())
            .set(
                filled_rect(),
                FilledRect {
                    color: self.color,
                    fill_image: None,
                },
            )
            .set(color(), self.color)
            .set_default(rect());
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

        scope
            .set(name(), "Image".into())
            .set_default(screen_position())
            .set_default(local_position())
            .set(
                filled_rect(),
                FilledRect {
                    color: Srgba::new(1.0, 1.0, 1.0, 1.0),
                    fill_image: Some(image),
                },
            )
            .set_default(model_matrix())
            .set_default(rect());
    }
}

#[derive(Default)]
struct List<W> {
    items: W,
    layout: Layout,
    background_color: Option<Srgba>,

    padding: Edges,
    margin: Edges,
}

impl<W: WidgetCollection> List<W> {
    fn new(items: W) -> Self {
        Self {
            items,
            layout: Layout::default(),
            background_color: None,
            padding: Edges::default(),
            margin: Edges::default(),
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

    pub fn with_padding(mut self, padding: Edges) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_margin(mut self, margin: Edges) -> Self {
        self.margin = margin;
        self
    }
}

impl<W: WidgetCollection> Widget for List<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        scope
            .set_default(rect())
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
            .set_default(screen_position())
            .set_default(local_position())
            .set_default(model_matrix())
            .set_opt(color(), self.background_color)
            .set(padding(), self.padding)
            .set(margin(), self.margin);

        self.items.attach(scope)
    }
}

struct FontAtlasView {}

impl Widget for FontAtlasView {
    fn mount(self, scope: &mut Scope<'_>) {
        let font = FontFromFile {
            path: BytesFromFile(PathBuf::from("assets/fonts/Inter/static/Inter-Regular.ttf")),
        };

        scope
            .set(name(), "Inter Font".into())
            .set_default(screen_position())
            .set_default(local_position())
            .set(font_from_file(), font)
            .set(text(), "Hello, World!".into())
            .set_default(model_matrix())
            .set_default(rect());
    }
}

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "MainApp".into())
            // .set(
            //     shape(),
            //     Shape::FilledRect(FilledRect {
            //         color: named::BLUE.into_format().with_alpha(1.0),
            //     }),
            // )
            .set_default(rect())
            .set_default(screen_position())
            .set_default(local_position())
            .set(padding(), Edges::even(5.0))
            .set(padding(), Edges::even(5.0))
            .set(size(), Unit::rel(vec2(1.0, 1.0)));

        // scope.attach(Counter);
        // scope.attach(Rectangle {
        //     color: palette::named::BLUEVIOLET.into_format().with_alpha(1.0),
        // });

        scope.attach(
            Positioned::new(
                Sized::new(Rectangle {
                    color: Hsla::new(270.0, 0.5, 0.5, 1.0).into_color(),
                    margin: Default::default(),
                })
                .with_size(Unit::px(vec2(100.0, 0.0)) + Unit::rel(vec2(0.0, 1.0))),
            )
            .with_offset(Unit::rel(vec2(1.0, 0.0)))
            // TODO: parent anchor
            .with_anchor(Unit::rel(vec2(1.0, 0.0))),
        );

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
            Positioned::new(Sized::new(FontAtlasView {}).with_size(Unit::px(vec2(400.0, 200.0))))
                .with_offset(Unit::rel(Vec2::Y))
                .with_anchor(Unit::rel(Vec2::Y)),
        );

        let list1 = List::new((
            Sized::new(Rectangle {
                color: Hsla::new(0.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(10.0),
            })
            .with_min_size(Unit::px(vec2(100.0, 100.0)))
            .with_size(Unit::px(vec2(0.0, 100.0)) + Unit::rel(vec2(0.5, 0.0))),
            Sized::new(Rectangle {
                color: Hsla::new(30.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(10.0),
            })
            .with_size(Unit::px(vec2(100.0, 50.0))),
            Sized::new(Rectangle {
                color: Hsla::new(60.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(25.0),
            })
            .with_size(Unit::px(vec2(0.0, 60.0)) + Unit::rel(vec2(0.2, 0.0))),
            Sized::new(Rectangle {
                color: Hsla::new(90.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::new(10.0, 25.0, 10.0, 25.0),
            })
            .with_min_size(Unit::px(vec2(50.0, 100.0)))
            .with_size(Unit::px(vec2(50.0, 0.0)) + Unit::rel(vec2(0.0, 0.2))),
        ))
        .with_background_color(Hsla::new(190.0, 0.048, 0.143, 1.0).into_color())
        .with_padding(Edges::even(10.0))
        .with_margin(Edges::even(10.0));

        let list3 = List::new((
            Sized::new(Rectangle {
                color: Hsla::new(180.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::default(),
            })
            .with_size(Unit::px(vec2(80.0, 20.0))),
            Sized::new(Rectangle {
                color: Hsla::new(270.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::default(),
            })
            .with_size(Unit::px(vec2(100.0, 20.0))),
        ))
        .with_direction(Direction::Vertical)
        .with_cross_align(CrossAlign::End);

        let list2 = List::new((
            (Sized::new(Rectangle {
                color: Hsla::new(30.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(5.0),
            })
            .with_size(Unit::px(vec2(100.0, 50.0)))),
            List::new([list3]).with_padding(Edges::even(10.0)),
            Sized::new(Rectangle {
                color: Hsla::new(60.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(5.0),
            })
            .with_min_size(Unit::px(vec2(20.0, 60.0)))
            .with_size(Unit::px(vec2(200.0, 60.0))),
            Sized::new(Rectangle {
                color: Hsla::new(90.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(5.0),
            })
            .with_size(Unit::px(vec2(50.0, 50.0))),
        ))
        .with_cross_align(CrossAlign::Center)
        .with_background_color(Hsla::new(190.0, 0.048, 0.143, 1.0).into_color())
        .with_padding(Edges::even(10.0))
        .with_margin(Edges::even(10.0));

        scope.attach(
            List::new((list1, list2))
                .with_cross_align(CrossAlign::Stretch)
                .with_direction(Direction::Vertical)
                .with_background_color(Hsla::new(190.0, 0.048, 0.1, 1.0).into_color())
                .with_padding(Edges::even(10.0)),
        );
    }
}

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    App::new().run(MainApp)
}
