use std::sync::Arc;

use wgpu::{Adapter, Backends, CurrentSurfaceTexture, SurfaceConfiguration, TextureFormat};
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

/// Represents the Gpu and graphics state
#[derive(Debug)]
pub struct Gpu {
    pub adapter: Arc<Adapter>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

pub struct Surface {
    size: Option<PhysicalSize<u32>>,
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
}

impl Surface {
    pub fn get_current_texture(&self) -> CurrentSurfaceTexture {
        self.surface.get_current_texture()
    }

    pub fn surface_config(&self) -> &SurfaceConfiguration {
        &self.config
    }

    pub fn resize(&mut self, gpu: &Gpu, new_size: PhysicalSize<u32>) {
        tracing::debug_span!("resize", ?new_size);
        if Some(new_size) == self.size {
            return;
        }

        if new_size.width > 0 && new_size.height > 0 {
            // self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.size = Some(new_size);
            self.reconfigure(gpu);
        } else {
            self.size = None;
        }
    }

    pub fn has_size(&self) -> bool {
        self.size.is_some()
    }

    pub fn reconfigure(&mut self, gpu: &Gpu) {
        self.surface.configure(&gpu.device, &self.config);
    }

    pub fn surface_format(&self) -> TextureFormat {
        self.config.format
    }

    pub fn size(&self) -> Option<PhysicalSize<u32>> {
        self.size
    }
}

impl Gpu {
    /// Creates the gpu and a surface for `window` (async, as adapter and device creation are).
    ///
    /// `display` must be the display connection `window` belongs to; GLES (notably on Wayland)
    /// needs it at instance creation to present.
    pub async fn with_surface(display: OwnedDisplayHandle, window: Arc<Window>) -> (Self, Surface) {
        #[cfg(not(target_arch = "wasm32"))]
        let backends = Backends::all();

        #[cfg(target_arch = "wasm32")]
        let backends = Backends::GL;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display))
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits {
                        max_texture_dimension_2d: 4096,
                        ..wgpu::Limits::downlevel_webgl2_defaults()
                    }
                } else {
                    wgpu::Limits::default()
                },
                label: None,
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or_else(|| surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            ..surface.get_default_config(&adapter, 0, 0).unwrap()
        };

        // surface.configure(&device, &config);

        (
            Self {
                adapter: Arc::new(adapter),
                device: Arc::new(device),
                queue: Arc::new(queue),
            },
            Surface {
                surface,
                config,
                size: None,
            },
        )
    }
}
