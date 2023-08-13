use wgpu::{BufferUsages, RenderPass};

use super::{
    graphics::{allocator::Allocation, multi_buffer::MultiBuffer, Vertex},
    Gpu,
};

pub struct MeshBuffer {
    label: String,
    pub vb: MultiBuffer<Vertex>,
    pub ib: MultiBuffer<u32>,
}

/// Handle to an allocation within a mesh
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle {
    vb: Allocation,
    ib: Allocation,
}

impl MeshHandle {
    pub fn vb(&self) -> Allocation {
        self.vb
    }

    pub fn ib(&self) -> Allocation {
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
            vb: vertex_buffer,
            ib: index_buffer,
        }
    }

    /// Allocate a mesh in the buffer
    pub fn allocate(&mut self, gpu: &Gpu, vertex_count: usize, index_count: usize) -> MeshHandle {
        tracing::debug!("Allocating {vertex_count} {index_count}");
        let vb = match self.vb.allocate(vertex_count) {
            Some(v) => v,
            None => {
                self.vb.grow(gpu, vertex_count);
                self.vb.allocate(vertex_count).unwrap()
            }
        };

        let ib = match self.ib.allocate(index_count) {
            Some(v) => v,
            None => {
                self.ib.grow(gpu, index_count);
                self.ib.allocate(index_count).unwrap()
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
        render_pass.set_vertex_buffer(0, self.vb.slice(..));
        render_pass.set_index_buffer(self.ib.slice(..), wgpu::IndexFormat::Uint32);
    }

    pub(crate) fn reallocate(
        &mut self,
        gpu: &Gpu,
        handle: &mut MeshHandle,
        vertex_count: usize,
        index_count: usize,
    ) {
        handle.vb = match self.vb.try_reallocate(handle.vb, vertex_count) {
            Some(v) => v,
            None => {
                self.vb.grow(gpu, vertex_count);
                self.vb.allocate(vertex_count).unwrap()
            }
        };

        handle.ib = match self.ib.try_reallocate(handle.ib, index_count) {
            Some(v) => v,
            None => {
                self.ib.grow(gpu, index_count);
                self.ib.allocate(index_count).unwrap()
            }
        };
    }

    pub fn write(&mut self, gpu: &Gpu, handle: &MeshHandle, vertices: &[Vertex], indices: &[u32]) {
        self.vb.write(&gpu.queue, &handle.vb, vertices);
        self.ib.write(&gpu.queue, &handle.ib, indices);
    }
}
