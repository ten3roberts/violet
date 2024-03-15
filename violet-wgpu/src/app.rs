use futures::channel::oneshot;
use std::sync::Arc;
use web_time::{Duration, Instant};

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
    to_owned, Frame, FutureEffect, Rect, Scope, Widget,
};

use crate::{
    graphics::Gpu,
    renderer::{RendererConfig, WindowRenderer},
    systems::{register_text_buffers, update_text_buffers},
    text::TextSystem,
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

pub struct App {
    renderer_config: RendererConfig,
}

impl App {
    pub fn new() -> Self {
        Self {
            renderer_config: Default::default(),
        }
    }

    /// Set the renderer config
    pub fn with_renderer_config(mut self, renderer_config: RendererConfig) -> Self {
        self.renderer_config = renderer_config;
        self
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
        let (renderer_tx, mut renderer_rx) = oneshot::channel();

        let text_system = Arc::new(Mutex::new(TextSystem::new_with_defaults()));
        let (layout_changes_tx, layout_changes_rx) = flume::unbounded();

        frame.spawn(FutureEffect::new(Gpu::with_surface(window.clone()), {
            to_owned![text_system];
            move |frame: &mut Frame, (gpu, surface)| {
                renderer_tx
                    .send(WindowRenderer::new(
                        frame,
                        gpu,
                        root,
                        text_system.clone(),
                        surface,
                        layout_changes_rx.clone(),
                        self.renderer_config,
                    ))
                    .ok();
            }
        }));

        let mut renderer = None;

        let mut schedule = Schedule::new()
            .with_system(templating_system(root, layout_changes_tx))
            .flush()
            .with_system(hydrate_text())
            .flush()
            .with_system(register_text_buffers(text_system.clone()))
            .flush()
            .with_system(update_text_buffers(text_system.clone()))
            .with_system(invalidate_cached_layout_system(&mut frame.world))
            .with_system(layout_system(root))
            .with_system(transform_system());

        let start_time = Instant::now();
        let mut cur_time = start_time;

        #[cfg(not(target_arch = "wasm32"))]
        let _puffin_server = setup_puffin();

        let mut minimized = true;

        event_loop.run(move |event, ctl| match event {
            Event::AboutToWait => {
                puffin::profile_scope!("AboutToWait");

                if let Some(mut window_renderer) = renderer_rx.try_recv().ok().flatten() {
                    window_renderer.resize(window_size);
                    renderer = Some(window_renderer);
                }

                if minimized {
                    return;
                }

                let new_time = Instant::now();

                let frame_time = new_time.duration_since(cur_time);

                cur_time = new_time;

                // tracing::info!(?dt, fps = 1.0 / delta_time);

                stats.record_frame(frame_time);

                {
                    puffin::profile_scope!("Tick");
                    ex.tick(&mut frame);
                }

                update_animations(&mut frame, cur_time - start_time);

                {
                    puffin::profile_scope!("Schedule");
                    schedule.execute_seq(&mut frame.world).unwrap();
                }

                if let Some(renderer) = &mut renderer {
                    puffin::profile_scope!("Draw");
                    if let Err(err) = renderer.draw(&mut frame) {
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
                    if let Some(renderer) = &mut renderer {
                        if let Err(err) = renderer.draw(&mut frame) {
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
                WindowEvent::KeyboardInput { event, .. } => {
                    puffin::profile_scope!("KeyboardInput", format!("{event:?}"));
                    input_state.on_keyboard_input(&mut frame, event)
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

                    if let Some(renderer) = &mut renderer {
                        renderer.resize(size);
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

#[cfg(not(target_arch = "wasm32"))]
fn setup_puffin() -> Option<puffin_http::Server> {
    let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
    let server = match puffin_http::Server::new(&server_addr) {
        Ok(server) => server,
        Err(err) => {
            tracing::warn!("Failed to start puffin server: {err}");
            return None;
        }
    };

    tracing::info!("Puffin running at {server_addr}");
    puffin::set_scopes_on(true);
    Some(server)
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
