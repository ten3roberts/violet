use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use flax::{components::name, Entity, Schedule, World};
use glam::{vec2, Vec2};
use parking_lot::Mutex;
use winit::{
    event::{Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

use violet_core::{
    assets::AssetCache,
    components::{self, local_position, rect, screen_position},
    executor::Executor,
    input::InputState,
    style::{setup_stylesheet, stylesheet},
    systems::{
        hydrate_text, invalidate_cached_layout_system, layout_system, templating_system,
        transform_system,
    },
    Frame, Rect, Scope, Widget,
};

use crate::{
    graphics::Gpu,
    systems::{register_text_buffers, update_text_buffers},
    text_renderer::TextSystem,
    window_renderer::WindowRenderer,
};

pub struct Canvas<W> {
    stylesheet: Entity,
    size: Vec2,
    root: W,
}

impl<W: Widget> Widget for Canvas<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        scope
            .set(name(), "Canvas".into())
            .set(stylesheet(self.stylesheet), ())
            .set(
                rect(),
                Rect {
                    min: Vec2::ZERO,
                    max: self.size,
                },
            )
            .set_default(screen_position())
            .set_default(local_position());

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

        let mut frame = Frame::new(spawner, AssetCache::new(), World::new());

        let event_loop = EventLoopBuilder::new().build();

        let window = WindowBuilder::new().build(&event_loop)?;

        let window_size = window.inner_size();
        let window_size = vec2(window_size.width as f32, window_size.height as f32);

        let mut input_state = InputState::new(Vec2::ZERO);

        let stylesheet = setup_stylesheet().spawn(frame.world_mut());

        // Mount the root widget
        let root = frame.new_root(Canvas {
            stylesheet,
            size: window_size,
            root,
        });

        let mut stats = AppStats::new(60);

        // TODO: Make this a proper effect
        let (gpu, surface) = futures::executor::block_on(Gpu::with_surface(window));

        let text_system = Arc::new(Mutex::new(TextSystem::new()));

        let mut window_renderer =
            WindowRenderer::new(gpu, &mut frame, text_system.clone(), surface);

        let mut schedule = Schedule::new()
            .with_system(templating_system(root))
            .flush()
            .with_system(hydrate_text())
            .flush()
            .with_system(register_text_buffers(text_system.clone()))
            .flush()
            .with_system(update_text_buffers(text_system))
            .with_system(invalidate_cached_layout_system(&mut frame.world))
            .with_system(layout_system())
            .with_system(transform_system());

        let mut cur_time = Instant::now();

        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                let new_time = Instant::now();

                let frame_time = new_time.duration_since(cur_time);
                let delta_time = frame_time.as_secs_f32();

                cur_time = new_time;

                frame.delta_time = delta_time;

                // tracing::info!(?dt, fps = 1.0 / delta_time);

                stats.record_frame(frame_time);

                ex.tick(&mut frame);

                schedule.execute_seq(&mut frame.world).unwrap();

                if let Err(err) = window_renderer.draw(&mut frame) {
                    tracing::error!("Failed to draw to window: {err:?}");
                    *ctl = ControlFlow::Exit
                }

                let report = stats.report();
                window_renderer.surface().window().set_title(&format!(
                    "Violet - {:>4.1?} {:>4.1?} {:>4.1?}",
                    report.min_frame_time, report.average_frame_time, report.max_frame_time,
                ));
            }
            Event::RedrawRequested(_) => {
                tracing::info!("Redraw requested");
                if let Err(err) = window_renderer.draw(&mut frame) {
                    tracing::error!("Failed to draw to window: {err:?}");
                    *ctl = ControlFlow::Exit
                }
            }
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::MouseInput { state, button, .. } => {
                    input_state.on_mouse_input(&mut frame, state, button);
                }
                WindowEvent::ReceivedCharacter(c) => {
                    input_state.on_char_input(&mut frame, c);
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    input_state.on_modifiers_change(modifiers);
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(keycode),
                            ..
                        },
                    ..
                } => input_state.on_keyboard_input(&mut frame, state, keycode),
                WindowEvent::CursorMoved { position, .. } => input_state
                    .on_cursor_move(&mut frame, vec2(position.x as f32, position.y as f32)),
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

                    window_renderer.resize(size);
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

struct AppStats {
    frames: Vec<AppFrame>,
    max_frames: usize,
}

impl AppStats {
    pub fn new(max_frames: usize) -> Self {
        Self {
            frames: Vec::with_capacity(max_frames),
            max_frames,
        }
    }

    fn record_frame(&mut self, frame_time: Duration) {
        self.frames.push(AppFrame { frame_time });
        if self.frames.len() > self.max_frames {
            self.frames.remove(0);
        }
    }

    fn report(&self) -> StatsReport {
        let average = self
            .frames
            .iter()
            .map(|f| f.frame_time)
            .sum::<Duration>()
            .div_f32(self.frames.len() as f32);

        let min = self
            .frames
            .iter()
            .map(|f| f.frame_time)
            .min()
            .unwrap_or_default();
        let max = self
            .frames
            .iter()
            .map(|f| f.frame_time)
            .max()
            .unwrap_or_default();

        StatsReport {
            average_frame_time: average,
            min_frame_time: min,
            max_frame_time: max,
        }
    }
}

pub struct StatsReport {
    pub average_frame_time: Duration,
    pub min_frame_time: Duration,
    pub max_frame_time: Duration,
}

struct AppFrame {
    frame_time: Duration,
}
