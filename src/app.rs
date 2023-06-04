use flax::World;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopBuilder,
    window::WindowBuilder,
};

use crate::{executor::Executor, Frame, Widget};

pub struct App {}

impl App {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(self, root: impl Widget) -> anyhow::Result<()> {
        let mut ex = Executor::new();

        let spawner = ex.spawner();

        let mut frame = Frame {
            world: World::new(),
            spawner,
        };

        let event_loop = EventLoopBuilder::new().build();

        let window = WindowBuilder::new().build(&event_loop)?;

        // Mount the root widget
        root.mount(&mut frame);

        event_loop.run(move |event, _, ctl| {
            let _window = &window;

            match event {
                Event::MainEventsCleared => {
                    ex.tick(&mut frame);
                }
                Event::WindowEvent { window_id, event } => match event {
                    WindowEvent::CloseRequested => {
                        *ctl = winit::event_loop::ControlFlow::Exit;
                    }
                    event => {
                        tracing::debug!(?event, ?window_id, "Window event")
                    }
                },
                event => {
                    tracing::debug!(?event, "Event")
                }
            }
        })
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
