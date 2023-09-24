use std::sync::{Arc, Mutex};

use wgpu::{BufferUsages, RenderPass};

use super::{
    graphics::{
        multi_buffer::{MultiBuffer, SubBuffer},
        Vertex,
    },
    Gpu,
};

pub struct MeshBufferInner {}

pub struct MeshBuffer {
    next_id: u64,
    label: String,
    pub vertex_buffers: MultiBuffer<Vertex>,
    pub index_buffers: MultiBuffer<u32>,
    dropped: Arc<Mutex<Vec<(SubBuffer<Vertex>, SubBuffer<u32>)>>>,
}

/// Handle to an allocation within a mesh
#[derive(Debug, Clone)]
pub struct MeshHandle {
    id: u64,
    vb: SubBuffer<Vertex>,
    ib: SubBuffer<u32>,
    on_drop: Arc<Mutex<Vec<(SubBuffer<Vertex>, SubBuffer<u32>)>>>,
}

impl std::hash::Hash for MeshHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for MeshHandle {}

impl PartialEq for MeshHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl MeshHandle {
    pub fn vb(&self) -> SubBuffer<Vertex> {
        self.vb
    }

    pub fn ib(&self) -> SubBuffer<u32> {
        self.ib
    }
}

impl Drop for MeshHandle {
    fn drop(&mut self) {
        self.on_drop.lock().unwrap().push((self.vb, self.ib));
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
            dropped: Arc::default(),
            next_id: 0,
        }
    }

    fn reclaim(&mut self) {
        for (vb, ib) in self.dropped.lock().unwrap().drain(..) {
            tracing::debug!(?vb, ?ib, "reclaim");
            self.vertex_buffers.deallocate(vb);
            self.index_buffers.deallocate(ib);
        }
    }

    /// Allocate a mesh in the buffer
    pub fn allocate(&mut self, gpu: &Gpu, vertex_count: usize, index_count: usize) -> MeshHandle {
        self.allocate_exact(
            gpu,
            vertex_count.next_power_of_two(),
            index_count.next_power_of_two(),
        )
    }
    pub fn allocate_exact(
        &mut self,
        gpu: &Gpu,
        vertex_count: usize,
        index_count: usize,
    ) -> MeshHandle {
        self.reclaim();
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

        let next_id = self.next_id;
        self.next_id += 1;

        MeshHandle {
            id: next_id,
            vb,
            ib,
            on_drop: self.dropped.clone(),
        }
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
        if handle.vb.size() >= vertex_count && handle.ib.size() >= index_count {
            panic!("");
            return;
        }

        self.reclaim();
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

    pub fn write(&mut self, gpu: &Gpu, handle: &MeshHandle, vertices: &[Vertex], indices: &[u32]) {
        self.vertex_buffers.write(&gpu.queue, &handle.vb, vertices);
        self.index_buffers.write(&gpu.queue, &handle.ib, indices);
    }
}
