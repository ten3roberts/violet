use flax::{Schedule, World};
use glam::{vec2, Vec2};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

use crate::{
    components::{self, rect, Rect},
    executor::Executor,
    systems::layout_system,
    wgpu::{graphics::Gpu, window_renderer::WindowRenderer},
    Frame, Widget,
};

pub struct Canvas<W> {
    size: Vec2,
    root: W,
}

impl<W: Widget> Widget for Canvas<W> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        scope.set(
            rect(),
            Rect {
                min: Vec2::ZERO,
                max: self.size,
            },
        );
        scope.attach(self.root);
    }
}

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
        let window_size = window.inner_size();
        let window_size = vec2(window_size.width as f32, window_size.height as f32);

        // Mount the root widget
        let root = frame.new_root(Canvas {
            size: window_size,
            root,
        });

        // TODO: Make this a proper effect
        let (gpu, surface) = futures::executor::block_on(Gpu::with_surface(window));

        let mut window_renderer = WindowRenderer::new(&gpu, surface);

        let mut schedule = Schedule::new().with_system(layout_system());

        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                ex.tick(&mut frame);

                schedule.execute_seq(&mut frame.world).unwrap();
                window_renderer.update(&mut frame, root);

                if let Err(err) = window_renderer.draw(&gpu) {
                    tracing::error!("Failed to draw to window: {err:?}");
                    *ctl = ControlFlow::Exit
                }
            }
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::Resized(size) => {
                    frame
                        .world_mut()
                        .set(
                            root,
                            components::rect(),
                            Rect {
                                min: vec2(0.0, 0.0),
                                max: vec2(size.width as f32, size.height as f32),
                            },
                        )
                        .unwrap();

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
