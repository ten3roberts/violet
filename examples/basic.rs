use std::time::{Duration, Instant};
use flax::{child_of, name, Entity};
use futures::StreamExt;
use glam::vec2;
use tracing_subscriber::EnvFilter;
use violet::{
    components::{children, position, shape},
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
                    size: vec2(100.0, 100.0),
                }),
            )
            .set(position(), vec2(0.0, 0.0))
            .attach(Counter);
    }
}

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    App::new().run(MainApp)
}
