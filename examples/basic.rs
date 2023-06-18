use anyhow::Context;
use flax::name;
use futures::StreamExt;
use glam::{vec2, Vec2};
use image::DynamicImage;
use palette::{Hsla, IntoColor, Srgba};
use std::{path::PathBuf, time::Duration};
use tracing_subscriber::EnvFilter;
use violet::{
    assets::AssetKey,
    components::{
        constraints, layout, local_position, margin, padding, rect, screen_position, shape, Edges,
    },
    layout::{CrossAlign, Direction, Layout},
    shapes::{FilledRect, Shape},
    time::{interval, sleep},
    App, Constraints, FutureEffect, Scope, StreamEffect, Widget, WidgetCollection,
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

struct Constrained<W> {
    constraints: Constraints,
    widget: W,
}

impl<W> Constrained<W> {
    pub fn new(widget: W) -> Self {
        Self {
            widget,
            constraints: Constraints::default(),
        }
    }

    fn absolute_size(mut self, abs_size: Vec2) -> Self {
        self.constraints.abs_size = abs_size;
        self
    }

    fn absolute_offset(mut self, abs_offset: Vec2) -> Self {
        self.constraints.abs_offset = abs_offset;
        self
    }

    fn relative_size(mut self, rel_size: Vec2) -> Self {
        self.constraints.rel_size = rel_size;
        self
    }

    fn relative_offset(mut self, rel_offset: Vec2) -> Self {
        self.constraints.rel_offset = rel_offset;
        self
    }

    fn anchor(mut self, anchor: Vec2) -> Self {
        self.constraints.anchor = anchor;
        self
    }
}

impl<W> Widget for Constrained<W>
where
    W: Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);

        scope.set(constraints(), self.constraints);
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
            .set(
                shape(),
                Shape::FilledRect(FilledRect {
                    color: self.color,
                    fill_image: None,
                }),
            )
            .set(margin(), self.margin)
            .set_default(screen_position())
            .set_default(local_position())
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
        let image = scope.assets_mut().load(ImageFromPath {
            path: self.path.into(),
        });

        scope
            .set(name(), "Image".into())
            .set_default(screen_position())
            .set_default(local_position())
            .set(
                shape(),
                Shape::FilledRect(FilledRect {
                    color: Srgba::new(1.0, 1.0, 1.0, 1.0),
                    fill_image: Some(image),
                }),
            )
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
                shape(),
                self.background_color.map(|bg| {
                    Shape::FilledRect(FilledRect {
                        // color: Hsla::new(180.0, 0.048, 0.243, 1.0).into_color(),
                        // color: Hsla::new(190.0, 0.048, 0.143, 1.0).into_color(),
                        color: bg,
                        fill_image: None,
                    })
                }),
            )
            .set(layout(), self.layout)
            .set_default(constraints())
            .set_default(screen_position())
            .set_default(local_position())
            .set(padding(), self.padding)
            .set(margin(), self.margin);

        self.items.attach(scope)
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
            .set(
                constraints(),
                Constraints {
                    rel_size: vec2(1.0, 1.0),
                    ..Default::default()
                },
            );

        // scope.attach(Counter);
        // scope.attach(Rectangle {
        //     color: palette::named::BLUEVIOLET.into_format().with_alpha(1.0),
        // });

        scope.attach(
            Constrained::new(Rectangle {
                color: Hsla::new(270.0, 0.5, 0.5, 1.0).into_color(),
                margin: Default::default(),
            })
            .absolute_size(vec2(100.0, 0.0))
            .relative_size(vec2(0.0, 1.0))
            .relative_offset(vec2(1.0, 0.0))
            .anchor(vec2(1.0, 0.0)),
        );

        scope.spawn(FutureEffect::new(
            sleep(Duration::from_secs(2)),
            move |scope: &mut Scope, _| {
                scope.attach(
                    Constrained::new(Image {
                        path: "./assets/images/uv.png",
                    })
                    .absolute_size(vec2(400.0, 400.0))
                    .relative_offset(vec2(0.0, 1.0))
                    .anchor(vec2(0.0, 1.0)),
                );
            },
        ));
        let list1 = List::new((
            (Constrained::new(Rectangle {
                color: Hsla::new(0.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(10.0),
            })
            .relative_size(vec2(0.2, 0.0))
            .absolute_size(vec2(0.0, 100.0))),
            (Constrained::new(Rectangle {
                color: Hsla::new(30.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(10.0),
            })
            .absolute_size(vec2(100.0, 50.0))),
            (Constrained::new(Rectangle {
                color: Hsla::new(60.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(25.0),
            })
            .relative_size(vec2(0.2, 0.0))
            .absolute_size(vec2(0.0, 60.0))),
            (Constrained::new(Rectangle {
                color: Hsla::new(90.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::new(10.0, 25.0, 10.0, 25.0),
            })
            .relative_size(vec2(0.0, 0.2))
            .absolute_size(vec2(50.0, 0.0))),
        ))
        .with_background_color(Hsla::new(190.0, 0.048, 0.143, 1.0).into_color())
        .with_padding(Edges::even(10.0))
        .with_margin(Edges::even(10.0));

        let list3 = List::new((
            Constrained::new(Rectangle {
                color: Hsla::new(180.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::default(),
            })
            .absolute_size(vec2(80.0, 20.0)),
            Constrained::new(Rectangle {
                color: Hsla::new(270.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::default(),
            })
            .absolute_size(vec2(100.0, 20.0)),
        ))
        .with_direction(Direction::Vertical);

        let list2 = List::new((
            List::new([list3]).with_padding(Edges::even(10.0)),
            (Constrained::new(Rectangle {
                color: Hsla::new(30.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(5.0),
            })
            .absolute_size(vec2(100.0, 50.0))),
            (Constrained::new(Rectangle {
                color: Hsla::new(60.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(5.0),
            })
            .absolute_size(vec2(50.0, 60.0))),
            (Constrained::new(Rectangle {
                color: Hsla::new(90.0, 0.5, 0.5, 1.0).into_color(),
                margin: Edges::even(5.0),
            })
            .absolute_size(vec2(50.0, 50.0))),
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
