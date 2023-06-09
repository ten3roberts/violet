use anyhow::Context;
use glam::Mat4;
use wgpu::{BufferUsages, Operations, RenderPassDescriptor, ShaderStages, SurfaceError};
use winit::dpi::PhysicalSize;

use crate::Frame;

use super::{
    graphics::{BindGroupBuilder, BindGroupLayoutBuilder, Gpu, Surface, TypedBuffer},
    ShapeRenderer,
};

/// Renders to a window surface
pub struct WindowRenderer {
    surface: Surface,

    globals: Globals,
    globals_buffer: TypedBuffer<Globals>,
    globals_bind_group: wgpu::BindGroup,
    shape_renderer: ShapeRenderer,
}

impl WindowRenderer {
    pub fn new(gpu: &Gpu, frame: &mut Frame, surface: Surface) -> Self {
        let globals_layout = BindGroupLayoutBuilder::new("WindowRenderer::globals_layout")
            .bind_uniform_buffer(ShaderStages::VERTEX)
            .build(gpu);

        let globals = Globals {
            projview: Mat4::IDENTITY,
        };

        let globals_buffer = TypedBuffer::new(
            gpu,
            "WindowRenderer::globals_buffer",
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            &[globals],
        );

        let globals_bind_group = BindGroupBuilder::new("WindowRenderer::globals")
            .bind_buffer(&globals_buffer)
            .build(gpu, &globals_layout);

        let shape_renderer =
            ShapeRenderer::new(gpu, frame, &globals_layout, surface.surface_format());

        Self {
            surface,
            globals_buffer,
            globals_bind_group,
            shape_renderer,
            globals,
        }
    }

    pub fn resize(&mut self, gpu: &Gpu, new_size: PhysicalSize<u32>) {
        let w = new_size.width as f32;
        let h = new_size.height as f32;

        self.globals.projview = Mat4::orthographic_lh(0.0, w, h, 0.0, 0.0, 1000.0);
        self.globals_buffer.write(&gpu.queue, &[self.globals]);

        self.surface.resize(gpu, new_size);
    }

    pub fn draw(&mut self, gpu: &Gpu, frame: &mut Frame) -> anyhow::Result<()> {
        let target = match self.surface.get_current_texture() {
            Ok(v) => v,
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                self.surface.reconfigure(gpu);
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
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.shape_renderer
                .draw(gpu, frame, &self.globals_bind_group, &mut render_pass)
                .context("Failed to draw shapes")?;
        }

        gpu.queue.submit([encoder.finish()]);
        target.present();

        Ok(())
    }
}

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Globals {
    projview: Mat4,
}
