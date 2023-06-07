use flax::{child_of, name};
use futures::StreamExt;
use glam::{vec2, Vec2};
use palette::{
    named::{LIGHTGRAY, LIGHTSLATEGRAY},
    Srgba, WithAlpha,
};
use std::time::{Duration, Instant};
use tracing_subscriber::EnvFilter;
use violet::{
    components::{
        absolute_offset, absolute_size, origin, position, relative_offset, relative_size, shape,
        size,
    },
    shapes::{Rect, Shape},
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
    abs_size: Option<Vec2>,
    abs_offset: Option<Vec2>,
    rel_offset: Option<Vec2>,
    rel_size: Option<Vec2>,
    origin: Option<Vec2>,
    widget: W,
}

impl<W> Constrained<W> {
    pub fn new(widget: W) -> Self {
        Self {
            widget,
            abs_size: None,
            abs_offset: None,
            rel_offset: None,
            rel_size: None,
            origin: None,
        }
    }
    fn absolute_size(mut self, abs_size: Vec2) -> Self {
        self.abs_size = Some(abs_size);
        self
    }
    fn absolute_offset(mut self, abs_offset: Vec2) -> Self {
        self.abs_offset = Some(abs_offset);
        self
    }
    fn relative_size(mut self, rel_size: Vec2) -> Self {
        self.rel_size = Some(rel_size);
        self
    }
    fn relative_offset(mut self, rel_offset: Vec2) -> Self {
        self.rel_offset = Some(rel_offset);
        self
    }
    fn origin(mut self, origin: Vec2) -> Self {
        self.origin = Some(origin);
        self
    }
}

impl<W> Widget for Constrained<W>
where
    W: Widget,
{
    fn mount(self, scope: &mut Scope<'_>) {
        self.widget.mount(scope);

        scope.set_opt(absolute_size(), self.abs_size);
        scope.set_opt(absolute_offset(), self.abs_offset);
        scope.set_opt(relative_offset(), self.rel_offset);
        scope.set_opt(relative_size(), self.rel_size);
        scope.set_opt(origin(), self.origin);
    }
}

struct Rectangle {
    color: Srgba<u8>,
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "Rectangle".into())
            .set(shape(), Shape::Rect(Rect { color: self.color }))
            .set_default(position())
            .set_default(size());
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

        let start = Instant::now();
        scope.spawn(StreamEffect::new(
            interval(Duration::from_millis(1000 / 60)),
            move |scope: &mut Scope, _| {
                let t = start.elapsed().as_secs_f32();
                scope.set(position(), vec2(t.sin() * 100.0, 0.0));
            },
        ));

        scope
            .set(name(), "MainApp".into())
            .set(
                shape(),
                Shape::Rect(Rect {
                    color: palette::named::WHITESMOKE.into_format().with_alpha(255),
                }),
            )
            .set_default(size())
            .set_default(position())
            .set(absolute_size(), vec2(-20.0, -20.0))
            .set(relative_size(), vec2(1.0, 1.0));

        scope.attach(Counter);
        // scope.attach(Rectangle {
        //     color: palette::named::BLUEVIOLET.into_format().with_alpha(255),
        // });

        scope.attach(
            Constrained::new(Rectangle {
                color: LIGHTSLATEGRAY.into_format().with_alpha(255),
            })
            .absolute_size(vec2(100.0, 0.0))
            .relative_size(vec2(0.0, 0.5))
            .relative_offset(vec2(1.0, 0.0))
            .absolute_offset(vec2(-10.0, 10.0))
            .origin(vec2(1.0, 0.0)),
        );

        scope.attach(
            Constrained::new(Rectangle {
                color: palette::named::TEAL.into_format().with_alpha(255),
            })
            .absolute_size(vec2(100.0, 100.0))
            .absolute_offset(vec2(10.0, 10.0)),
        );
        scope.attach(
            Constrained::new(Rectangle {
                color: palette::named::TEAL.into_format().with_alpha(255),
            })
            .absolute_size(vec2(100.0, 100.0))
            .absolute_offset(vec2(120.0, 10.0)),
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
