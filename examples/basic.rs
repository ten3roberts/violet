use flax::{name, Entity};
use violet::{App, Frame, Widget};

struct MainApp;

impl Widget for MainApp {
    fn mount(self, frame: &mut Frame) -> Entity {
        Entity::builder()
            .set(name(), "MainApp".into())
            .spawn(&mut frame.world)
    }
}

pub fn main() -> anyhow::Result<()> {
    App::new().run(MainApp)
}
