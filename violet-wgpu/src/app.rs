use std::{mem, sync::Arc};

use cosmic_text::fontdb::Source;
use flax::{components::name, Entity, EntityBuilder, Schedule, World};
use glam::{vec2, Vec2};
use parking_lot::Mutex;
use violet_core::{
    animation::update_animations,
    assets::AssetCache,
    components::{self, rect},
    executor::Executor,
    input::{request_focus_sender, InputState},
    io::{self, Clipboard},
    layout::cache::LayoutUpdateEvent,
    style::{stylesheet, StylesheetOptions},
    systems::{
        compute_transform_system, hydrate_text, invalidate_cached_layout_system, layout_system,
        templating_system, transform_system,
    },
    widget::interactive::overlay::OverlayStack,
    Frame, FutureEffect, Rect, Scope, Widget,
};
use web_time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use crate::{
    graphics::Gpu,
    renderer::{MainRendererConfig, WindowRenderer},
    systems::{register_text_buffers, update_text_buffers},
    text::{TextSystem, INTER_FONT, INTER_FONT_BOLD, INTER_FONT_ITALIC},
};

pub struct Canvas<W> {
    pub stylesheet: Entity,
    pub root: W,
}

impl<W: Widget> Widget for Canvas<W> {
    fn mount(self, scope: &mut Scope<'_>) {
        scope
            .set(name(), "Canvas".into())
            .set(stylesheet(self.stylesheet), ())
            .flush();

        OverlayStack::new().mount(scope);
        scope.attach(self.root);
    }
}

pub struct AppBuilder {
    renderer_config: MainRendererConfig,
    allow_resize: bool,
    title: String,
    fonts: Vec<Source>,
    stylesheet: Option<EntityBuilder>,
}

impl AppBuilder {
    pub fn new() -> Self {
        let fonts = vec![
            Source::Binary(Arc::new(INTER_FONT.to_vec())),
            Source::Binary(Arc::new(INTER_FONT_BOLD.to_vec())),
            Source::Binary(Arc::new(INTER_FONT_ITALIC.to_vec())),
        ];

        Self {
            renderer_config: Default::default(),
            title: "Violet".to_string(),
            allow_resize: false,
            fonts,
            stylesheet: None,
        }
    }

    /// Add a font to the text system
    pub fn with_font(mut self, font: Source) -> Self {
        self.fonts.push(font);
        self
    }

    /// Provide a custom stylesheet
    pub fn with_stylesheet(mut self, stylesheet: EntityBuilder) -> Self {
        self.stylesheet = Some(stylesheet);
        self
    }

    pub fn with_resize_window(mut self, enable: bool) -> Self {
        self.allow_resize = enable;
        self
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

    /// Build the application without running it
    pub fn build(self, root: impl Widget) -> AppInstance {
        AppInstance::new(
            root,
            self.allow_resize,
            Arc::new(Mutex::new(TextSystem::new_with_fonts(
                self.fonts.into_iter().rev(),
            ))),
            self.stylesheet
                .unwrap_or_else(|| StylesheetOptions::new().build()),
        )
    }

    /// Build and run the app in an event loop
    pub fn run(self, root: impl Widget) -> anyhow::Result<()> {
        let event_loop = EventLoop::builder().build()?;

        let resize_window = self.allow_resize;
        let renderer_config = self.renderer_config.clone();
        let title = self.title.clone();

        let instance = self.build(root);

        let (renderer_tx, renderer_rx) = flume::unbounded();

        #[cfg(not(target_arch = "wasm32"))]
        let _puffin_server = setup_puffin();

        let event_handler = WindowEventHandler {
            instance,
            renderer: None,
            window: None,
            renderer_tx,
            renderer_rx,
            renderer_config,
            title,
            resize_window,
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut event_handler = event_handler;
            event_loop.run_app(&mut event_handler)?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(event_handler);
        }

        Ok(())
    }
}

/// A running application instance of violet
pub struct AppInstance {
    pub frame: Frame,
    pub root: Entity,
    scale_factor: f64,
    current_time: Instant,
    start_time: Instant,
    executor: Executor,
    schedule: Schedule,
    window_size: PhysicalSize<u32>,
    pub input_state: InputState,
    text_system: Arc<Mutex<TextSystem>>,
    layout_changes_rx: flume::Receiver<(Entity, LayoutUpdateEvent)>,
    pub needs_update: bool,
}

impl AppInstance {
    pub fn new(
        root: impl Widget,
        resize_canvas: bool,
        text_system: Arc<Mutex<TextSystem>>,
        mut stylesheet: EntityBuilder,
    ) -> AppInstance {
        let executor = Executor::new();

        let spawner = executor.spawner();

        let mut frame = Frame::new(spawner, AssetCache::new(), World::new());

        let stylesheet = stylesheet.spawn(frame.world_mut());

        let clipboard = frame.store_mut().insert(Arc::new(Clipboard::new()));
        frame.set_atom(io::clipboard(), clipboard);

        let (request_focus_tx, request_focus_rx) = flume::unbounded();
        frame.set_atom(request_focus_sender(), request_focus_tx);

        // Mount the root widget
        let root = frame.new_root(Canvas { stylesheet, root });

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
            .with_system(layout_system(root, resize_canvas))
            .with_system(compute_transform_system())
            .with_system(transform_system(root));

        let input_state = InputState::new(root, Vec2::ZERO, request_focus_rx);

        let start_time = Instant::now();

        Self {
            frame,
            root,
            scale_factor: 1.0,
            current_time: start_time,
            start_time,
            executor,
            schedule,
            window_size: Default::default(),
            input_state,
            text_system,
            layout_changes_rx,
            needs_update: false,
        }
    }

    pub fn builder() -> AppBuilder {
        AppBuilder::new()
    }

    pub fn on_resize(&mut self, physical_size: PhysicalSize<u32>) {
        self.window_size = physical_size;
        self.needs_update = true;

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
    }

    pub fn update(&mut self) {
        if self.is_minimized() {
            return;
        }

        self.needs_update = false;

        let new_time = Instant::now();

        self.current_time = new_time;

        self.executor.tick(&mut self.frame);

        update_animations(&mut self.frame, self.current_time - self.start_time);

        {
            self.schedule.execute_seq(&mut self.frame.world).unwrap();
        }
    }

    pub fn is_minimized(&self) -> bool {
        self.window_size.width == 0 || self.window_size.height == 0
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    pub fn input_state(&self) -> &InputState {
        &self.input_state
    }

    pub fn input_state_mut(&mut self) -> &mut InputState {
        &mut self.input_state
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn root(&self) -> Entity {
        self.root
    }

    pub fn text_system(&self) -> &Arc<Mutex<TextSystem>> {
        &self.text_system
    }

    pub fn layout_changes_rx(&self) -> &flume::Receiver<(Entity, LayoutUpdateEvent)> {
        &self.layout_changes_rx
    }
}

struct WindowEventHandler {
    instance: AppInstance,
    renderer: Option<WindowRenderer>,
    window: Option<Arc<Window>>,
    renderer_rx: flume::Receiver<WindowRenderer>,
    renderer_tx: flume::Sender<WindowRenderer>,
    renderer_config: MainRendererConfig,
    title: String,
    resize_window: bool,
}

impl WindowEventHandler {
    pub fn draw(&mut self) -> anyhow::Result<()> {
        puffin::profile_function!();
        if mem::take(&mut self.instance.needs_update) {
            self.instance.update();
        }

        if let Some(renderer) = &mut self.renderer {
            renderer.draw(&mut self.instance.frame)?;
        }

        Ok(())
    }
}

impl ApplicationHandler for WindowEventHandler {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            #[allow(unused_mut)]
            let mut window_attributes = Window::default_attributes()
                .with_resizable(!self.resize_window)
                .with_decorations(true)
                .with_title(self.title.clone());

            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                use winit::platform::web::WindowAttributesExtWebSys;

                let canvas = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("canvas")
                    .unwrap()
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .unwrap();

                window_attributes = window_attributes.with_canvas(Some(canvas));
            }

            let Ok(window) = event_loop.create_window(window_attributes) else {
                tracing::error!("Failed to create window");
                event_loop.exit();
                return;
            };

            // Install size listener
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

            let window = Arc::new(window);
            let root = self.instance.root;
            let renderer_tx = self.renderer_tx.clone();
            let renderer_config = self.renderer_config.clone();
            let text_system = self.instance.text_system.to_owned();
            let layout_changes_rx = self.instance.layout_changes_rx.to_owned();

            self.instance
                .frame
                .spawn(FutureEffect::new(Gpu::with_surface(window.clone()), {
                    move |frame: &mut Frame, (gpu, surface)| {
                        let window_renderer = WindowRenderer::new(
                            frame,
                            gpu,
                            root,
                            text_system.clone(),
                            surface,
                            layout_changes_rx.clone(),
                            renderer_config,
                        );
                        renderer_tx.send(window_renderer).ok();
                    }
                }));

            self.window = Some(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let instance = &mut self.instance;

        instance.input_state.update_external_focus(&instance.frame);

        match event {
            WindowEvent::RedrawRequested => {
                puffin::profile_scope!("RedrawRequested");
                if let Err(err) = self.draw() {
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
                instance.input_state.on_keyboard_input(
                    &mut instance.frame,
                    event.logical_key,
                    event.state,
                    event.text,
                );
            }
            WindowEvent::CursorMoved { position, .. } => {
                puffin::profile_scope!("CursorMoved");
                instance.input_state.on_cursor_move(
                    &mut instance.frame,
                    vec2(position.x as f32, position.y as f32),
                );
            }
            WindowEvent::MouseWheel { delta, .. } => {
                puffin::profile_scope!("MouseWheel");
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        const LINE_SIZE: f32 = 16.0;
                        instance
                            .input_state
                            .on_scroll(&mut instance.frame, vec2(x * LINE_SIZE, y * LINE_SIZE));
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        let pos = pos.to_logical::<f32>(instance.scale_factor);
                        instance
                            .input_state
                            .on_scroll(&mut instance.frame, vec2(pos.x, pos.y));
                    }
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor: s, ..
            } => {
                instance.set_scale_factor(s);

                let size = instance.window_size;
                instance.on_resize(size);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size, self.instance.scale_factor);
                }
            }
            WindowEvent::Resized(size) => {
                instance.on_resize(size);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size, self.instance.scale_factor);
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            event => {
                tracing::trace!(?event, ?window_id, "Window event")
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(window) = &self.window else {
            return;
        };

        puffin::profile_scope!("AboutToWait");

        if let Ok(mut window_renderer) = self.renderer_rx.try_recv() {
            window_renderer.resize(self.instance.window_size, self.instance.scale_factor);
            self.renderer = Some(window_renderer);
        }

        let monitor = window.current_monitor();
        let max_size: LogicalSize<f32> = monitor
            .map(|v| v.size())
            .unwrap_or(PhysicalSize::new(800, 600))
            .to_logical(self.instance.scale_factor);

        let max_size = vec2(max_size.width, max_size.height) - 20.0;

        if self.resize_window {
            let canvas = self
                .instance
                .frame
                .world_mut()
                .entity_mut(self.instance.root)
                .unwrap();
            canvas
                .update_dedup(
                    components::rect(),
                    Rect::from_size(vec2(max_size.x, max_size.y)),
                )
                .unwrap();
        }

        self.instance.update();

        if self.resize_window {
            let canvas_size = *self
                .instance
                .frame
                .world
                .get(self.instance.root, rect())
                .unwrap();

            let wanted_size = canvas_size.size().min(max_size);

            let new_size =
                window.request_inner_size(LogicalSize::new(wanted_size.x, wanted_size.y));

            if let Some(new_size) = new_size {
                self.instance.on_resize(new_size);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size, self.instance.scale_factor);
                }
            }
        }
        if !self.instance.is_minimized() {
            self.instance.frame.world.prune_archetypes();
        }

        event_loop.set_control_flow(ControlFlow::Poll);
        window.request_redraw();
        puffin::GlobalProfiler::lock().new_frame();
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
