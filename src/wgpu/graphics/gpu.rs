use wgpu::{
    Adapter, Backends, SurfaceCapabilities, SurfaceConfiguration, SurfaceError, SurfaceTexture,
    TextureFormat,
};
use winit::{dpi::PhysicalSize, window::Window};

/// Represents the Gpu and graphics state
#[derive(Debug)]
pub struct Gpu {
    adapter: Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub struct Surface {
    size: PhysicalSize<u32>,
    window: Window,
    surface: wgpu::Surface,
    surface_format: TextureFormat,
    surface_caps: SurfaceCapabilities,
}

impl Surface {
    pub fn get_current_texture(&self) -> Result<SurfaceTexture, SurfaceError> {
        self.surface.get_current_texture()
    }

    pub fn surface_config(&self) -> SurfaceConfiguration {
        SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            width: self.size.width,
            height: self.size.height,
            present_mode: self.surface_caps.present_modes[0],
            alpha_mode: self.surface_caps.alpha_modes[0],
            view_formats: vec![],
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, gpu: &Gpu, new_size: PhysicalSize<u32>) {
        tracing::info_span!("resize", ?new_size);
        if new_size.width > 0 && new_size.height > 0 {
            // self.size = new_size;
            // self.config.width = new_size.width;
            // self.config.height = new_size.height;
            self.surface.configure(&gpu.device, &self.surface_config());
            self.size = new_size;
        }
    }

    pub fn surface_format(&self) -> TextureFormat {
        self.surface_format
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }
}

impl Gpu {
    // Creating some of the wgpu types requires async code
    #[tracing::instrument(level = "info")]
    pub async fn with_surface(window: Window) -> (Self, Surface) {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        tracing::debug!("Surface capabilities: {surface_caps:?}");

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or_else(|| surface_caps.formats[0]);

        tracing::debug!("Found surface format: {:?}", surface_format);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            // present_mode: surface_caps.present_modes[0],
            // alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        tracing::debug!("Surface configuration: {config:?}");

        surface.configure(&device, &config);

        (
            Self {
                adapter,
                device,
                queue,
            },
            Surface {
                window,
                surface,
                surface_format,
                surface_caps,
                size,
            },
        )
    }

    // pub fn surface_caps(&self) -> &SurfaceCapabilities {
    //     &self.surface_caps
    // }
}
