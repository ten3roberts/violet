use futures::channel::oneshot;
use parking_lot::Mutex;
use std::sync::Arc;
use web_time::{Duration, Instant};

use flax::{components::name, entity_ids, Entity, Query, Schedule, World};
use glam::{vec2, Vec2};
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget},
    window::WindowBuilder,
};

use violet_core::{
    animation::update_animations,
    assets::AssetCache,
    components::{self, max_size, size},
    executor::Executor,
    input::InputState,
    io::{self, Clipboard},
    layout::cache::LayoutUpdateEvent,
    style::{primary_surface, setup_stylesheet, stylesheet, Background, SizeExt},
    systems::{
        hydrate_text, invalidate_cached_layout_system, layout_system, templating_system,
        transform_system,
    },
    unit::Unit,
    widget::{col, WidgetExt},
    Frame, FutureEffect, Rect, Scope, Widget,
};

use crate::{
    graphics::Gpu,
    renderer::{MainRendererConfig, WindowRenderer},
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
            .set(max_size(), Unit::px(self.size))
            .set(size(), Unit::px(self.size));

        scope.attach(
            col(self.root)
                .contain_margins(true)
                .with_maximize(Vec2::ONE)
                .with_background(Background::new(primary_surface()))
                .with_name("CanvasColumn"),
        );
    }
}

pub struct AppBuilder {
    renderer_config: MainRendererConfig,
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
    pub fn with_renderer_config(mut self, renderer_config: MainRendererConfig) -> Self {
        self.renderer_config = renderer_config;
        self
    }

    pub fn run(self, root: impl Widget) -> anyhow::Result<()> {
        let event_loop = EventLoopBuilder::new().build()?;

        #[allow(unused_mut)]
        let mut builder = WindowBuilder::new().with_title(self.title.clone());

        let mut instance = AppInstance::new(root);

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
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().expect("Missing window canvas");
            let sf = window.scale_factor() as f32;
            let on_resize = move || {
                let (w, h) = (
                    (canvas.client_width() as f32 * sf) as u32,
                    (canvas.client_height() as f32 * sf) as u32,
                );

                canvas.set_width(w as _);
                canvas.set_height(h as _);
            };

            on_resize();

            let window = web_sys::window().unwrap();
            window
                .add_event_listener_with_callback(
                    "resize",
                    wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::EventTarget)>::new(
                        move |_: web_sys::EventTarget| {
                            on_resize();
                        },
                    )
                    .into_js_value()
                    .unchecked_ref(),
                )
                .expect("Failed to add resize listener");
        }

        tracing::info!("creating gpu");
        let window = Arc::new(window);

        let (renderer_tx, mut renderer_rx) = oneshot::channel();

        instance
            .frame
            .spawn(FutureEffect::new(Gpu::with_surface(window.clone()), {
                let text_system = instance.text_system.to_owned();
                let layout_changes_rx = instance.layout_changes_rx.to_owned();
                move |frame: &mut Frame, (gpu, surface)| {
                    renderer_tx
                        .send(WindowRenderer::new(
                            frame,
                            gpu,
                            instance.root,
                            text_system.clone(),
                            surface,
                            layout_changes_rx.clone(),
                            self.renderer_config,
                        ))
                        .ok();
                }
            }));

        #[cfg(not(target_arch = "wasm32"))]
        let _puffin_server = setup_puffin();

        let on_event = move |event, ctl: &EventLoopWindowTarget<()>| match event {
            Event::AboutToWait => {
                puffin::profile_scope!("AboutToWait");

                if let Some(mut renderer) = renderer_rx.try_recv().ok().flatten() {
                    renderer.resize(instance.window_size, instance.scale_factor);
                    instance.renderer = Some(renderer);
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
                    instance
                        .input_state
                        .on_mouse_input(&mut instance.frame, state, button);
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    puffin::profile_scope!("ModifiersChanged");
                    instance.input_state.on_modifiers_change(modifiers.state());
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    puffin::profile_scope!("KeyboardInput", format!("{event:?}"));
                    instance
                        .input_state
                        .on_keyboard_input(&mut instance.frame, event)
                }
                WindowEvent::CursorMoved { position, .. } => {
                    puffin::profile_scope!("CursorMoved");
                    instance.input_state.on_cursor_move(
                        &mut instance.frame,
                        vec2(position.x as f32, position.y as f32),
                    )
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    puffin::profile_scope!("MouseWheel");
                    match delta {
                        winit::event::MouseScrollDelta::LineDelta(x, y) => {
                            const LINE_SIZE: f32 = 16.0;
                            instance
                                .input_state
                                .on_scroll(&mut instance.frame, vec2(x * LINE_SIZE, y * LINE_SIZE))
                        }
                        winit::event::MouseScrollDelta::PixelDelta(pos) => {
                            let pos = pos.to_logical::<f32>(instance.scale_factor);
                            instance
                                .input_state
                                .on_scroll(&mut instance.frame, vec2(pos.x, pos.y))
                        }
                    }
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
pub struct AppInstance {
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
    input_state: InputState,
    text_system: Arc<Mutex<TextSystem>>,
    layout_changes_rx: flume::Receiver<(Entity, LayoutUpdateEvent)>,
}

impl AppInstance {
    pub fn new(root: impl Widget) -> AppInstance {
        let executor = Executor::new();

        let spawner = executor.spawner();

        let mut frame = Frame::new(spawner, AssetCache::new(), World::new());

        let stylesheet = setup_stylesheet().spawn(frame.world_mut());

        let clipboard = frame.store_mut().insert(Arc::new(Clipboard::new()));
        frame.set_atom(io::clipboard(), clipboard);

        // Mount the root widget
        let root = frame.new_root(Canvas {
            stylesheet,
            size: vec2(0.0, 0.0),
            root,
        });

        let text_system = Arc::new(Mutex::new(TextSystem::new_with_defaults()));
        let (layout_changes_tx, layout_changes_rx) = flume::unbounded();

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
            .with_system(transform_system(root));

        let input_state = InputState::new(root, Vec2::ZERO);

        let start_time = Instant::now();

        Self {
            frame,
            renderer: None,
            root,
            scale_factor: 1.0,
            stats: AppStats::new(60),
            current_time: start_time,
            start_time,
            executor,
            schedule,
            window_size: Default::default(),
            input_state,
            text_system,
            layout_changes_rx,
        }
    }

    pub fn insert_renderer(&mut self, renderer: WindowRenderer) {
        self.renderer = Some(renderer);
    }

    pub fn builder() -> AppBuilder {
        AppBuilder::new()
    }

    pub fn on_resize(&mut self, physical_size: PhysicalSize<u32>) {
        self.window_size = physical_size;

        tracing::info!(?physical_size, self.scale_factor, "Resizing window");

        let logical_size: LogicalSize<f32> = physical_size.to_logical(self.scale_factor);

        let canvas = self.frame.world_mut().entity_mut(self.root).unwrap();
        canvas
            .update_dedup(
                components::rect(),
                Rect::from_size(vec2(logical_size.width, logical_size.height)),
            )
            .unwrap();

        canvas
            .update_dedup(
                components::clip_mask(),
                Rect::from_size(vec2(logical_size.width, logical_size.height)),
            )
            .unwrap();

        self.schedule.execute_seq(&mut self.frame.world).unwrap();

        if let Some(renderer) = &mut self.renderer {
            renderer.resize(physical_size, self.scale_factor);
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

pub struct AppStats {
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

    pub fn report(&self) -> StatsReport {
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
