use flax::{child_of, name};
use futures::StreamExt;
use glam::{vec2, Vec2};
use palette::{
    named::{LIGHTSLATEGRAY, PURPLE},
    Srgba, WithAlpha,
};
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use violet::{
    components::{constraints, padding, rect, shape, Padding},
    shapes::{FilledRect, Shape},
    systems::Constraints,
    time::interval,
    App, Frame, Scope, StreamEffect, Widget,
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
    color: Srgba<u8>,
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "Rectangle".into())
            .set(shape(), Shape::FilledRect(FilledRect { color: self.color }))
            .set_default(rect());
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
            .set(
                shape(),
                Shape::FilledRect(FilledRect {
                    color: palette::named::WHITESMOKE.into_format().with_alpha(255),
                }),
            )
            .set_default(rect())
            .set(padding(), Padding::even(10.0))
            .set(
                constraints(),
                Constraints {
                    rel_size: vec2(1.0, 1.0),
                    ..Default::default()
                },
            );

        scope.attach(Counter);
        // scope.attach(Rectangle {
        //     color: palette::named::BLUEVIOLET.into_format().with_alpha(255),
        // });

        scope.attach(
            Constrained::new(Rectangle {
                color: PURPLE.into_format().with_alpha(255),
            })
            .absolute_size(vec2(100.0, 0.0))
            .relative_size(vec2(0.0, 1.0))
            .relative_offset(vec2(1.0, 0.0))
            .anchor(vec2(1.0, 0.0)),
        );

        scope.attach(
            Constrained::new(Rectangle {
                color: palette::named::TEAL.into_format().with_alpha(255),
            })
            .absolute_size(vec2(100.0, 100.0)),
        );

        scope.attach(
            Constrained::new(Rectangle {
                color: palette::named::TEAL.into_format().with_alpha(255),
            })
            .absolute_size(vec2(100.0, 100.0))
            .absolute_offset(vec2(110.0, 0.0)),
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
