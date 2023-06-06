use flax::{child_of, name, Entity};
use futures::StreamExt;
use glam::{vec2, Vec2};
use palette::{Srgba, WithAlpha};
use std::time::{Duration, Instant};
use tracing_subscriber::EnvFilter;
use violet::{
    components::{
        absolute_offset, absolute_size, children, position, relative_offset, shape, size,
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

struct Rectangle {
    size: Vec2,
    center: Vec2,
    color: Srgba<u8>,
}

impl Widget for Rectangle {
    fn mount(self, scope: &mut Scope) {
        scope
            .set(name(), "Rectangle".into())
            .set(
                shape(),
                Shape::Rect(Rect {
                    size: self.size,
                    color: self.color,
                }),
            )
            .set(position(), Default::default())
            .set(size(), Default::default())
            .set(absolute_offset(), self.center);
    }
}

impl Widget for MainApp {
    fn mount(self, scope: &mut Scope) {
        let id = scope.id();

        scope.spawn_unscoped(StreamEffect::new(
            interval(Duration::from_secs(1)),
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
                    size: vec2(200.0, 200.0),
                    color: palette::named::WHITESMOKE.into_format().with_alpha(255),
                }),
            )
            .set(size(), vec2(200.0, 200.0))
            .set(position(), vec2(0.0, 0.0))
            .set(absolute_size(), vec2(200.0, 200.0))
            .set(absolute_offset(), vec2(-120.0, -120.0))
            .set(relative_offset(), vec2(0.5, 0.5));

        scope.attach(Counter);
        scope.attach(Rectangle {
            size: vec2(100.0, 100.0),
            center: vec2(0.0, 0.0),
            color: palette::named::BLUEVIOLET.into_format().with_alpha(255),
        });

        scope.attach(Rectangle {
            size: vec2(50.0, 50.0),
            center: vec2(0.0, -120.0),
            color: palette::named::TEAL.into_format().with_alpha(255),
        });
    }
}

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .init();

    App::new().run(MainApp)
}
