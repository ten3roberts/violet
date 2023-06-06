use flax::World;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

use crate::{
    executor::Executor,
    wgpu::{graphics::Gpu, renderer::WindowRenderer},
    Frame, Widget,
};

pub struct App {}

impl App {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(self, root: impl Widget) -> anyhow::Result<()> {
        let mut ex = Executor::new();

        let spawner = ex.spawner();

        let world = World::new();

        let mut frame = Frame { world, spawner };

        let event_loop = EventLoopBuilder::new().build();

        let window = WindowBuilder::new().build(&event_loop)?;

        // Mount the root widget
        let root = frame.new_root(root);

        // TODO: Make this a proper effect
        let (gpu, surface) = futures::executor::block_on(Gpu::with_surface(window));

        let mut window_renderer = WindowRenderer::new(&gpu, surface);

        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                ex.tick(&mut frame);

                window_renderer.update(&mut frame, root);

                if let Err(err) = window_renderer.draw(&gpu) {
                    tracing::error!("Failed to draw to window: {err:?}");
                    *ctl = ControlFlow::Exit
                }
            }
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::Resized(size) => {
                    window_renderer.resize(&gpu, size);
                }
                WindowEvent::CloseRequested => {
                    *ctl = ControlFlow::Exit;
                }
                event => {
                    tracing::trace!(?event, ?window_id, "Window event")
                }
            },
            event => {
                tracing::trace!(?event, "Event")
            }
        })
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
