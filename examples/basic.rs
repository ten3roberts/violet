use flax::{child_of, name};
use futures::StreamExt;
use glam::{vec2, Vec2};
use palette::{
    named::{self, LIGHTSLATEGRAY, PURPLE},
    rgb::Rgba,
    Hsla, IntoColor, Srgba, WithAlpha,
};
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use violet::{
    components::{constraints, layout, local_position, padding, position, rect, shape, Padding},
    layout::Layout,
    shapes::{FilledRect, Shape},
    time::interval,
    App, Constraints, Frame, Scope, StreamEffect, Widget,
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
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "Rectangle".into())
            .set(shape(), Shape::FilledRect(FilledRect { color: self.color }))
            .set_default(position())
            .set_default(local_position())
            .set_default(rect());
    }
}

struct List {}

impl Widget for List {
    fn mount(self, scope: &mut Scope<'_>) {
        scope
            .set_default(rect())
            .set(
                shape(),
                Shape::FilledRect(FilledRect {
                    // color: Hsla::new(180.0, 0.048, 0.243, 1.0).into_color(),
                    color: Hsla::new(190.0, 0.048, 0.143, 1.0).into_color(),
                }),
            )
            .set(layout(), Layout {})
            .set_default(constraints())
            .set_default(position())
            .set_default(local_position())
            .set(padding(), Padding::even(0.0));

        scope.attach(
            Constrained::new(Rectangle {
                color: Hsla::new(0.0, 0.5, 0.5, 1.0).into_color(),
            })
            .relative_size(vec2(0.5, 0.0))
            .absolute_size(vec2(0.0, 100.0)),
        );

        scope.attach(
            Constrained::new(Rectangle {
                color: Hsla::new(30.0, 0.5, 0.5, 1.0).into_color(),
            })
            .absolute_size(vec2(100.0, 50.0)),
        );

        scope.attach(
            Constrained::new(Rectangle {
                color: Hsla::new(60.0, 0.5, 0.5, 1.0).into_color(),
            })
            .relative_size(vec2(0.2, 0.0))
            .absolute_size(vec2(0.0, 60.0)),
        );
        scope.attach(
            Constrained::new(Rectangle {
                color: Hsla::new(90.0, 0.5, 0.5, 1.0).into_color(),
            })
            .absolute_size(vec2(50.0, 100.0)),
        );
    }
}

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope) {
        let id = scope.id();

        scope.spawn_unscoped(StreamEffect::new(
            interval(Duration::from_secs(5)),
            move |frame: &mut Frame, deadline| {
                tracing::info!(
                    ?deadline,
                    "{id}: {:#?}",
                    frame.world().format_hierarchy(child_of, id)
                )
            },
        ));

        scope
            .set(name(), "MainApp".into())
            // .set(
            //     shape(),
            //     Shape::FilledRect(FilledRect {
            //         color: named::BLUE.into_format().with_alpha(1.0),
            //     }),
            // )
            .set_default(rect())
            .set_default(position())
            .set_default(local_position())
            .set(padding(), Padding::even(5.0))
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
            })
            .absolute_size(vec2(100.0, 0.0))
            .relative_size(vec2(0.0, 1.0))
            .relative_offset(vec2(1.0, 0.0))
            .anchor(vec2(1.0, 0.0)),
        );

        scope.attach(List {});
    }
}

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    App::new().run(MainApp)
}
