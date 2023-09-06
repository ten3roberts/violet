use wgpu::{BufferUsages, RenderPass};

use super::{
    graphics::{
        multi_buffer::{MultiBuffer, SubBuffer},
        Vertex,
    },
    Gpu,
};

pub struct MeshBuffer {
    label: String,
    pub vertex_buffers: MultiBuffer<Vertex>,
    pub index_buffers: MultiBuffer<u32>,
}

/// Handle to an allocation within a mesh
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle {
    vb: SubBuffer<Vertex>,
    ib: SubBuffer<u32>,
}

impl MeshHandle {
    pub fn vb(&self) -> SubBuffer<Vertex> {
        self.vb
    }

    pub fn ib(&self) -> SubBuffer<u32> {
        self.ib
    }
}

impl MeshBuffer {
    pub fn new(gpu: &Gpu, label: impl Into<String>, capacity: usize) -> Self {
        let label = label.into();

        let vertex_buffer = MultiBuffer::new(
            gpu,
            format!("{}::vertex_buffer", label),
            BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            capacity,
        );
        let index_buffer = MultiBuffer::new(
            gpu,
            format!("{}::index_buffer", label),
            BufferUsages::INDEX | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            capacity,
        );

        Self {
            label,
            vertex_buffers: vertex_buffer,
            index_buffers: index_buffer,
        }
    }

    /// Allocate a mesh in the buffer
    pub fn allocate(&mut self, gpu: &Gpu, vertex_count: usize, index_count: usize) -> MeshHandle {
        tracing::debug!("Allocating {vertex_count} {index_count}");
        let vb = match self.vertex_buffers.allocate(vertex_count) {
            Some(v) => v,
            None => {
                self.vertex_buffers.grow(gpu, vertex_count);
                self.vertex_buffers.allocate(vertex_count).unwrap()
            }
        };

        let ib = match self.index_buffers.allocate(index_count) {
            Some(v) => v,
            None => {
                self.index_buffers.grow(gpu, index_count);
                self.index_buffers.allocate(index_count).unwrap()
            }
        };

        MeshHandle { vb, ib }
    }

    pub fn insert(&mut self, gpu: &Gpu, vertices: &[Vertex], indices: &[u32]) -> MeshHandle {
        let mesh = self.allocate(gpu, vertices.len(), indices.len());
        self.write(gpu, &mesh, vertices, indices);
        mesh
    }

    pub fn bind<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffers.slice(..));
        render_pass.set_index_buffer(self.index_buffers.slice(..), wgpu::IndexFormat::Uint32);
    }

    pub(crate) fn reallocate(
        &mut self,
        gpu: &Gpu,
        handle: &mut MeshHandle,
        vertex_count: usize,
        index_count: usize,
    ) {
        handle.vb = match self.vertex_buffers.try_reallocate(handle.vb, vertex_count) {
            Some(v) => v,
            None => {
                self.vertex_buffers.grow(gpu, vertex_count);
                self.vertex_buffers.allocate(vertex_count).unwrap()
            }
        };

        handle.ib = match self.index_buffers.try_reallocate(handle.ib, index_count) {
            Some(v) => v,
            None => {
                self.index_buffers.grow(gpu, index_count);
                self.index_buffers.allocate(index_count).unwrap()
            }
        };
    }

    pub(crate) fn deallocate(&mut self, gpu: &Gpu, handle: &MeshHandle) {
        self.vertex_buffers.deallocate(handle.vb);
        self.index_buffers.deallocate(handle.ib);
    }

    pub fn write(&mut self, gpu: &Gpu, handle: &MeshHandle, vertices: &[Vertex], indices: &[u32]) {
        self.vertex_buffers.write(&gpu.queue, &handle.vb, vertices);
        self.index_buffers.write(&gpu.queue, &handle.ib, indices);
    }
}
