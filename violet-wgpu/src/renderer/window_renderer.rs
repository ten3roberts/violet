use std::sync::Arc;

use anyhow::Context;
use flax::Entity;
use glam::Mat4;
use parking_lot::Mutex;
use puffin::profile_scope;
use wgpu::{Operations, RenderPassDescriptor, StoreOp, SurfaceError};
use winit::dpi::{LogicalSize, PhysicalSize};

use violet_core::{layout::cache::LayoutUpdate, Frame};

use crate::{graphics::Surface, text::TextSystem, Gpu};

use super::{MainRenderer, RendererConfig, RendererContext};

/// Renders to a window surface
pub struct WindowRenderer {
    surface: Surface,

    ctx: RendererContext,
    widget_renderer: MainRenderer,
}

impl WindowRenderer {
    pub fn new(
        frame: &mut Frame,
        gpu: Gpu,
        root: Entity,
        text_system: Arc<Mutex<TextSystem>>,
        surface: Surface,
        layout_changes_rx: flume::Receiver<(Entity, LayoutUpdate)>,
        config: RendererConfig,
    ) -> Self {
        let mut ctx = RendererContext::new(gpu);

        let widget_renderer = MainRenderer::new(
            frame,
            &mut ctx,
            root,
            text_system,
            surface.surface_format(),
            layout_changes_rx,
            config,
        );

        Self {
            surface,
            widget_renderer,
            ctx,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>, scale_factor: f64) {
        let logical_size: LogicalSize<f32> = new_size.to_logical(scale_factor);
        let w = logical_size.width;
        let h = logical_size.height;

        self.ctx.globals.projview = Mat4::orthographic_lh(0.0, w, h, 0.0, 0.0, 1000.0);
        self.ctx
            .globals_buffer
            .write(&self.ctx.gpu.queue, 0, &[self.ctx.globals]);

        self.surface.resize(&self.ctx.gpu, new_size);
    }

    pub fn draw(&mut self, frame: &mut Frame) -> anyhow::Result<()> {
        if !self.surface.has_size() {
            tracing::info!("No surface size, skipping draw");
            return Ok(());
        }
        let target = match self.surface.get_current_texture() {
            Ok(v) => v,
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                self.surface.reconfigure(&self.ctx.gpu);
                return Ok(());
            }
            Err(err) => return Err(err).context("Failed to acquire surface texture"),
        };

        let view = target.texture.create_view(&Default::default());

        let mut encoder =
            self.ctx
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
                            // #3b4141
                            r: 0.04,
                            g: 0.05,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.widget_renderer
                .draw(&mut self.ctx, frame, &mut render_pass)
                .context("Failed to draw shapes")?;
        }

        {
            profile_scope!("submit");
            self.ctx.gpu.queue.submit([encoder.finish()]);
            target.present();
        }

        Ok(())
    }

    pub fn surface(&self) -> &Surface {
        &self.surface
    }
}
