use std::sync::Arc;

use anyhow::Context;
use wgpu::{
    Adapter, Backends, CurrentSurfaceTexture, SurfaceConfiguration, SurfaceTexture, TextureFormat,
};
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

/// Represents the Gpu and graphics state
#[derive(Debug)]
pub struct Gpu {
    pub adapter: Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub struct Surface {
    /// Kept to recreate [`Self::surface`] when it is lost.
    instance: wgpu::Instance,
    window: Arc<Window>,
    size: Option<PhysicalSize<u32>>,
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
    /// The last acquired texture was suboptimal; reconfigure once it has been presented.
    needs_reconfigure: bool,
}

impl Surface {
    /// Acquires the next texture to render to.
    ///
    /// Returns `Ok(None)` when this frame should be skipped:
    /// - the surface has no size yet, or a zero size;
    /// - the surface is outdated: it is reconfigured, so the next frame can proceed;
    /// - the surface is lost: it is recreated and configured, so the next frame can proceed;
    /// - acquiring timed out, or the window is occluded.
    ///
    /// A suboptimal texture is still returned, and the surface is reconfigured after it has been
    /// handed to [`Self::present`].
    pub fn acquire_texture(&mut self, gpu: &Gpu) -> anyhow::Result<Option<SurfaceTexture>> {
        if !self.has_size() {
            tracing::debug!("No surface size, skipping frame");
            return Ok(None);
        }

        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => Ok(Some(texture)),
            CurrentSurfaceTexture::Suboptimal(texture) => {
                self.needs_reconfigure = true;
                Ok(Some(texture))
            }
            CurrentSurfaceTexture::Outdated => {
                tracing::debug!("Surface outdated, reconfiguring and skipping frame");
                self.reconfigure(gpu);
                Ok(None)
            }
            CurrentSurfaceTexture::Lost => {
                tracing::debug!("Surface lost, recreating and skipping frame");
                self.surface = self
                    .instance
                    .create_surface(self.window.clone())
                    .context("Failed to recreate lost surface")?;
                self.reconfigure(gpu);
                Ok(None)
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                tracing::debug!("Surface texture unavailable, skipping frame");
                Ok(None)
            }
            CurrentSurfaceTexture::Validation => {
                anyhow::bail!("Validation error while acquiring surface texture")
            }
        }
    }

    /// Presents a texture from [`Self::acquire_texture`], reconfiguring the surface afterwards if
    /// it was suboptimal.
    pub fn present(&mut self, gpu: &Gpu, texture: SurfaceTexture) {
        gpu.queue.present(texture);

        if self.needs_reconfigure {
            tracing::debug!("Surface suboptimal, reconfiguring");
            self.reconfigure(gpu);
        }
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
        self.needs_reconfigure = false;
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

        // The wgpu environment variables (e.g. `WGPU_BACKEND=gl`) take precedence
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor {
                backends,
                ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display))
            }
            .with_env(),
        );

        let surface = instance.create_surface(window.clone()).unwrap();

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
                adapter,
                device,
                queue,
            },
            Surface {
                instance,
                window,
                surface,
                config,
                size: None,
                needs_reconfigure: false,
            },
        )
    }
}
