use std::time::Duration;

use flax::{child_of, name, Entity};
use futures::StreamExt;
use violet::{time::interval, App, Frame, StreamEffect, Widget};

struct MainApp;

struct Counter;
impl Widget for Counter {
    fn mount(self, frame: &mut Frame) -> Entity {
        let id = Entity::builder().spawn(&mut frame.world);

        frame.spawner.spawn(StreamEffect::new(
            interval(Duration::from_millis(200)).enumerate(),
            move |frame: &mut Frame, (i, _)| {
                frame
                    .world
                    .set(id, name(), format!("Counter: {:#?}", i))
                    .unwrap();
            },
        ));

        id
    }
}

impl Widget for MainApp {
    fn mount(self, frame: &mut Frame) -> Entity {
        frame.spawner.spawn(StreamEffect::new(
            interval(Duration::from_secs(1)),
            |frame: &mut Frame, deadline| tracing::info!(?deadline, "World: {:#?}", frame.world),
        ));

        let _counter = Counter.mount(frame);

        Entity::builder()
            .set(name(), "MainApp".into())
            .spawn(&mut frame.world)
    }
}

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    App::new().run(MainApp)
}
