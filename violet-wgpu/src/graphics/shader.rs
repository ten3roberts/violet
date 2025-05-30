use wgpu::{
    BindGroupLayout, PipelineLayoutDescriptor, RenderPipeline, TextureFormat, VertexBufferLayout,
};

use crate::Gpu;

#[derive(Debug, Clone)]
pub struct ShaderDesc<'a> {
    pub label: &'a str,
    pub source: &'a str,
    pub format: TextureFormat,
    pub vertex_layouts: &'a [VertexBufferLayout<'static>],
    pub layouts: &'a [&'a BindGroupLayout],
}

/// Represents a graphics shader
#[derive(Debug)]
pub struct Shader {
    pipeline: RenderPipeline,
}

impl Shader {
    pub fn new(gpu: &Gpu, desc: &ShaderDesc) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(desc.label),
                source: wgpu::ShaderSource::Wgsl(desc.source.into()),
            });

        let layout = gpu
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some(desc.label),
                bind_group_layouts: desc.layouts,
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(desc.label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main", // 1.
                    buffers: desc.vertex_layouts,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    // 3.
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        // 4.
                        format: desc.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: Default::default(),
            });

        Self { pipeline }
    }

    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }
}
