use futures::channel::oneshot;
use std::sync::Arc;
use web_time::{Duration, Instant};

use anyhow::anyhow;
use flax::{components::name, Entity, Schedule, World};
use glam::{vec2, Vec2};
use parking_lot::Mutex;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoopBuilder,
    keyboard::Key,
    window::WindowBuilder,
};

use violet_core::{
    animation::update_animations,
    assets::AssetCache,
    components::{self, local_position, rect, screen_position},
    executor::Executor,
    input::InputState,
    style::{setup_stylesheet, stylesheet},
    systems::{
        hydrate_text, invalidate_cached_layout_system, layout_system, templating_system,
        transform_system,
    },
    Frame, FutureEffect, Rect, Scope, Widget,
};

use crate::{
    graphics::{Gpu, Surface},
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

        let event_loop = EventLoopBuilder::new().build()?;

        #[allow(unused_mut)]
        let mut builder = WindowBuilder::new().with_inner_size(PhysicalSize::new(800, 600));

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;

            use winit::platform::web::WindowBuilderExtWebSys;
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            builder = builder.with_canvas(Some(canvas));
            // use winit::dpi::PhysicalSize;
            // window.request_inner_size(PhysicalSize::new(450, 400));

            // let node = web_sys::window()
            //     .unwrap()
            //     .document()
            //     .unwrap()
            //     .get_element_by_id("canvas-container")
            //     .unwrap();

            // use winit::platform::web::WindowExtWebSys;
            // node.append_child(&web_sys::Element::from(
            //     window.canvas().ok_or_else(|| anyhow!("No canvas"))?,
            // ))
            // .expect("Failed to add child");
        }

        let window = builder.build(&event_loop)?;

        let mut window_size = window.inner_size();

        let mut input_state = InputState::new(Vec2::ZERO);

        let stylesheet = setup_stylesheet().spawn(frame.world_mut());

        // Mount the root widget
        let root = frame.new_root(Canvas {
            stylesheet,
            size: vec2(window_size.width as f32, window_size.height as f32),
            root,
        });

        let mut stats = AppStats::new(60);

        tracing::info!("creating gpu");
        let window = Arc::new(window);

        // TODO: async within violet's executor
        let (gpu_tx, mut gpu_rx) = oneshot::channel();

        let text_system = Arc::new(Mutex::new(TextSystem::new_with_defaults()));
        frame.spawn(FutureEffect::new(
            {
                let window = window.clone();
                async move {
                    violet_core::time::sleep(Duration::from_secs(2)).await;
                    let gpu = Gpu::with_surface(window).await;
                    gpu_tx.send(gpu).ok();
                }
            },
            |_: &mut Frame, _| {},
        ));

        let mut window_renderer = None;

        let mut schedule = Schedule::new()
            .with_system(templating_system(root))
            .flush()
            .with_system(hydrate_text())
            .flush()
            .with_system(register_text_buffers(text_system.clone()))
            .flush()
            .with_system(update_text_buffers(text_system.clone()))
            .with_system(invalidate_cached_layout_system(&mut frame.world))
            .with_system(layout_system())
            .with_system(transform_system());

        let start_time = Instant::now();
        let mut cur_time = start_time;

        // let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
        // let _puffin_server = puffin_http::Server::new(&server_addr).unwrap();
        // eprintln!("Run this to view profiling data:  puffin_viewer {server_addr}");
        // puffin::set_scopes_on(true);
        let mut minimized = true;

        event_loop.run(move |event, ctl| match event {
            Event::AboutToWait => {
                puffin::profile_scope!("AboutToWait");

                if let Some((gpu, surface)) = gpu_rx.try_recv().ok().flatten() {
                    tracing::info!("created gpu");
                    let mut w = WindowRenderer::new(&mut frame, gpu, text_system.clone(), surface);
                    w.resize(window_size);
                    window_renderer = Some(w);
                }

                if minimized {
                    return;
                }

                let new_time = Instant::now();

                let frame_time = new_time.duration_since(cur_time);
                let delta_time = frame_time.as_secs_f32();

                cur_time = new_time;

                // tracing::info!(?dt, fps = 1.0 / delta_time);

                stats.record_frame(frame_time);

                ex.tick(&mut frame);

                update_animations(&mut frame, cur_time - start_time);

                schedule.execute_seq(&mut frame.world).unwrap();

                if let Some(window_renderer) = &mut window_renderer {
                    if let Err(err) = window_renderer.draw(&mut frame) {
                        tracing::error!("Failed to draw to window: {err:?}");
                    }
                }

                let report = stats.report();
                window.set_title(&format!(
                    "Violet - {:>4.1?} {:>4.1?} {:>4.1?}",
                    report.min_frame_time, report.average_frame_time, report.max_frame_time,
                ));
                puffin::GlobalProfiler::lock().new_frame();
            }
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::RedrawRequested => {
                    puffin::profile_scope!("RedrawRequested");
                    if let Some(window_renderer) = &mut window_renderer {
                        if let Err(err) = window_renderer.draw(&mut frame) {
                            tracing::error!("Failed to draw to window: {err:?}");
                        }
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    puffin::profile_scope!("MouseInput");
                    input_state.on_mouse_input(&mut frame, state, button);
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    puffin::profile_scope!("ModifiersChanged");
                    input_state.on_modifiers_change(modifiers.state());
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            text: Some(text),
                            ..
                        },
                    ..
                } => {
                    puffin::profile_scope!("KeyboardInput");

                    input_state.on_char_input(&mut frame, text.as_str());
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state,
                            logical_key: Key::Named(keycode),
                            ..
                        },
                    ..
                } => {
                    puffin::profile_scope!("KeyboardInput");

                    input_state.on_keyboard_input(&mut frame, state, keycode)
                }
                WindowEvent::CursorMoved { position, .. } => {
                    puffin::profile_scope!("CursorMoved");
                    input_state
                        .on_cursor_move(&mut frame, vec2(position.x as f32, position.y as f32))
                }
                WindowEvent::Resized(size) => {
                    puffin::profile_scope!("Resized");
                    minimized = size.width == 0 || size.height == 0;

                    window_size = size;

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

                    if let Some(window_renderer) = &mut window_renderer {
                        window_renderer.resize(size);
                    }
                }
                WindowEvent::CloseRequested => {
                    ctl.exit();
                }
                event => {
                    tracing::trace!(?event, ?window_id, "Window event")
                }
            },
            event => {
                tracing::trace!(?event, "Event")
            }
        })?;

        Ok(())
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
