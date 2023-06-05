use anyhow::Context;
use wgpu::{Operations, RenderPassDescriptor, SurfaceError};
use winit::dpi::PhysicalSize;

use super::graphics::{Gpu, Surface};

/// Renders to a window surface
pub struct WindowRenderer {
    surface: Surface,
}

impl WindowRenderer {
    pub fn new(surface: Surface) -> Self {
        Self { surface }
    }

    pub fn resize(&mut self, gpu: &Gpu, new_size: PhysicalSize<u32>) {
        self.surface.resize(gpu, new_size);
    }

    pub fn draw(&mut self, gpu: &Gpu) -> anyhow::Result<()> {
        let target = match self.surface.get_current_texture() {
            Ok(v) => v,
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                self.surface.resize(gpu, self.surface.size());
                return Ok(());
            }
            Err(err) => return Err(err).context("Failed to acquire surface texture"),
        };

        let view = target.texture.create_view(&Default::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("WindowRenderer::draw"),
            });

        {
            let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("WindowRenderer::draw"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.0,
                            b: 0.5,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        }

        gpu.queue.submit([encoder.finish()]);
        target.present();

        Ok(())
    }
}
