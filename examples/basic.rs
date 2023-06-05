use std::time::Duration;

use flax::{child_of, name, Entity};
use futures::StreamExt;
use violet::{components::children, time::interval, App, Frame, Scope, StreamEffect, Widget};

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
        scope.spawn(StreamEffect::new(
            interval(Duration::from_secs(1)),
            |scope: &mut Scope, deadline| {
                tracing::info!(?deadline, "Entity: {:#?}", scope.entity())
            },
        ));

        scope.set(name(), "MainApp".into()).attach(Counter);
    }
}

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    App::new().run(MainApp)
}
