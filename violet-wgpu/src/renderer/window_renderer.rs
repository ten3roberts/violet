use std::sync::Arc;

use anyhow::Context;
use flax::Entity;
use glam::Mat4;
use parking_lot::Mutex;
use puffin::profile_scope;
use wgpu::{Operations, RenderPassDescriptor, StoreOp, SurfaceError};
use winit::dpi::{LogicalSize, PhysicalSize};

use violet_core::{layout::cache::LayoutUpdate, Frame};

use crate::{graphics::Surface, text::TextSystem};

use super::{GlobalBuffers, Gpu, MainRenderer, RendererConfig, RendererProps};

/// Renders to a window surface
pub struct WindowRenderer {
    gpu: Gpu,
    surface: Surface,
    globals: GlobalBuffers,
    main_renderer: MainRenderer,
}

impl WindowRenderer {
    pub fn new(
        frame: &mut Frame,
        mut gpu: Gpu,
        root: Entity,
        text_system: Arc<Mutex<TextSystem>>,
        surface: Surface,
        layout_changes_rx: flume::Receiver<(Entity, LayoutUpdate)>,
        scale_factor: f64,
        config: RendererConfig,
    ) -> Self {
        let mut globals = GlobalBuffers::new(&mut gpu);
        let widget_renderer = MainRenderer::new(
            frame,
            &mut gpu,
            RendererProps {
                root,
                text_system,
                color_format: surface.surface_format(),
                globals: &mut globals,
                layout_changes_rx,
                config,
                scale_factor,
            },
        );

        Self {
            surface,
            main_renderer: widget_renderer,
            globals,
            gpu,
        }
    }

    pub fn resize(&mut self, physical_size: PhysicalSize<u32>, scale_factor: f64) {
        let logical_size: LogicalSize<f32> = physical_size.to_logical(scale_factor);
        let w = logical_size.width;
        let h = logical_size.height;
        // tracing::info!("resizing canvas size to {w}x{h}");

        self.globals.globals.projview = Mat4::orthographic_lh(0.0, w, h, 0.0, 0.0, 1000.0);
        self.globals
            .globals_buffer
            .write(&self.gpu.queue, 0, &[self.globals.globals]);

        self.main_renderer
            .resize(&self.gpu, physical_size, scale_factor);
        self.surface.resize(&self.gpu, physical_size);
    }

    pub fn draw(&mut self, frame: &mut Frame) -> anyhow::Result<()> {
        if !self.surface.has_size() {
            tracing::info!("No surface size, skipping draw");
            return Ok(());
        }
        let target = match self.surface.get_current_texture() {
            Ok(v) => v,
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                self.surface.reconfigure(&self.gpu);
                return Ok(());
            }
            Err(err) => return Err(err).context("Failed to acquire surface texture"),
        };

        let view = target.texture.create_view(&Default::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("WindowRenderer::draw"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("WindowRenderer::draw"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.4,
                            g: 0.05,
                            b: 0.2,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.main_renderer
                .draw(&self.gpu, &mut self.globals, frame, &mut render_pass)
                .context("Failed to draw shapes")?;
        }

        {
            profile_scope!("submit");
            self.gpu.queue.submit([encoder.finish()]);
            target.present();
        }

        Ok(())
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }
}
