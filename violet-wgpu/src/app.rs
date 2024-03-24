use futures::channel::oneshot;
use std::sync::Arc;
use web_time::{Duration, Instant};

use flax::{components::name, entity_ids, Entity, Query, Schedule, World};
use glam::{vec2, Vec2};
use parking_lot::Mutex;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget},
    window::WindowBuilder,
};

use violet_core::{
    animation::update_animations,
    assets::AssetCache,
    components::{self, local_position, rect},
    executor::Executor,
    input::InputState,
    io::{self, Clipboard},
    style::{primary_background, setup_stylesheet, stylesheet, Background},
    systems::{
        hydrate_text, invalidate_cached_layout_system, layout_system, templating_system,
        transform_system,
    },
    to_owned,
    widget::col,
    Frame, FutureEffect, Rect, Scope, Widget,
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
            .set(stylesheet(self.stylesheet), ());

        col(self.root)
            .contain_margins(true)
            .with_background(Background::new(primary_background()))
            .mount(scope);
    }
}

pub struct AppBuilder {
    renderer_config: RendererConfig,
    title: String,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            renderer_config: Default::default(),
            title: "Violet".to_string(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the renderer config
    pub fn with_renderer_config(mut self, renderer_config: RendererConfig) -> Self {
        self.renderer_config = renderer_config;
        self
    }

    pub fn run(self, root: impl Widget) -> anyhow::Result<()> {
        let executor = Executor::new();

        let spawner = executor.spawner();

        let mut frame = Frame::new(spawner, AssetCache::new(), World::new());

        let event_loop = EventLoopBuilder::new().build()?;

        #[allow(unused_mut)]
        let mut builder = WindowBuilder::new().with_title(self.title);

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
        }

        let window = builder.build(&event_loop)?;

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().expect("Missing window canvas");
            let (w, h) = (canvas.client_width(), canvas.client_height());

            canvas.set_width(w.try_into().unwrap());
            canvas.set_height(h.try_into().unwrap());
            window.request_inner_size(winit::dpi::PhysicalSize::new(w, h));
        }

        let mut input_state = InputState::new(Vec2::ZERO);

        let stylesheet = setup_stylesheet().spawn(frame.world_mut());

        let clipboard = frame.store_mut().insert(Arc::new(Clipboard::new()));
        frame.set_atom(io::clipboard(), clipboard);

        // Mount the root widget
        let root = frame.new_root(Canvas {
            stylesheet,
            size: vec2(0.0, 0.0),
            root,
        });

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

        let schedule = Schedule::new()
            .with_system(templating_system(layout_changes_tx))
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

        #[cfg(not(target_arch = "wasm32"))]
        let _puffin_server = setup_puffin();

        let mut instance = App {
            frame,
            renderer: None,
            root,
            scale_factor: window.scale_factor(),
            stats: AppStats::new(60),
            current_time: start_time,
            start_time,
            executor,
            schedule,
            window_size: window.inner_size(),
        };

        let on_event = move |event, ctl: &EventLoopWindowTarget<()>| match event {
            Event::AboutToWait => {
                puffin::profile_scope!("AboutToWait");

                if let Some(mut window_renderer) = renderer_rx.try_recv().ok().flatten() {
                    window_renderer.resize(instance.window_size, instance.scale_factor);
                    instance.renderer = Some(window_renderer);
                }

                instance.update();

                if !instance.is_minimized() {
                    let archetypes = instance.frame.world.archetype_info();
                    let pruned = instance.frame.world.prune_archetypes();
                    let entity_count = Query::new(entity_ids())
                        .borrow(&instance.frame.world)
                        .iter()
                        .count();
                    tracing::debug!(archetype_count = archetypes.len(), entity_count, pruned);
                    // let report = instance.?stats.report();

                    // window.set_title(&format!(
                    //     "Violet - {:>4.1?} {:>4.1?} {:>4.1?}",
                    //     report.min_frame_time, report.average_frame_time, report.max_frame_time,
                    // ));
                }

                ctl.set_control_flow(ControlFlow::Poll);
                window.request_redraw();
                puffin::GlobalProfiler::lock().new_frame();
            }
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::RedrawRequested => {
                    puffin::profile_scope!("RedrawRequested");
                    if let Err(err) = instance.draw() {
                        tracing::error!("Failed to draw to window: {err:?}");
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    puffin::profile_scope!("MouseInput");
                    input_state.on_mouse_input(&mut instance.frame, state, button);
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    puffin::profile_scope!("ModifiersChanged");
                    input_state.on_modifiers_change(modifiers.state());
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    puffin::profile_scope!("KeyboardInput", format!("{event:?}"));
                    input_state.on_keyboard_input(&mut instance.frame, event)
                }
                WindowEvent::CursorMoved { position, .. } => {
                    puffin::profile_scope!("CursorMoved");
                    input_state.on_cursor_move(
                        &mut instance.frame,
                        vec2(position.x as f32, position.y as f32),
                    )
                }
                WindowEvent::ScaleFactorChanged {
                    scale_factor: s, ..
                } => {
                    tracing::info!("Scale factor changed to {s}");
                    instance.scale_factor = s;

                    let size = instance.window_size;
                    instance.on_resize(size);
                }
                WindowEvent::Resized(size) => {
                    instance.on_resize(size);
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
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            event_loop.run(on_event)?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn(on_event);
        }

        Ok(())
    }
}

/// A running application instance of violet
pub struct App {
    frame: Frame,
    renderer: Option<WindowRenderer>,
    root: Entity,
    scale_factor: f64,
    stats: AppStats,
    current_time: Instant,
    start_time: Instant,
    executor: Executor,
    schedule: Schedule,
    window_size: PhysicalSize<u32>,
}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::new()
    }

    pub fn on_resize(&mut self, size: PhysicalSize<u32>) {
        self.window_size = size;

        tracing::info!(?size, self.scale_factor, "Resizing window");

        let logical_size: LogicalSize<f32> = size.to_logical(self.scale_factor);

        self.frame
            .world_mut()
            .set(
                self.root,
                components::rect(),
                Rect::from_size(vec2(logical_size.width, logical_size.height)),
            )
            .unwrap();

        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size, self.scale_factor);
        }
    }

    pub fn update(&mut self) {
        if self.is_minimized() {
            return;
        }

        let new_time = Instant::now();

        let frame_time = new_time.duration_since(self.current_time);

        self.current_time = new_time;
        self.stats.record_frame(frame_time);

        self.executor.tick(&mut self.frame);

        update_animations(&mut self.frame, self.current_time - self.start_time);

        {
            self.schedule.execute_seq(&mut self.frame.world).unwrap();
        }
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        puffin::profile_function!();
        if let Some(renderer) = &mut self.renderer {
            puffin::profile_scope!("Draw");
            renderer.draw(&mut self.frame)?;
        }

        Ok(())
    }
    pub fn is_minimized(&self) -> bool {
        self.window_size.width == 0 || self.window_size.height == 0
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

impl Default for AppBuilder {
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
        if self.frames.len() >= self.max_frames {
            self.frames.remove(0);
        }
        self.frames.push(AppFrame { frame_time });
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
