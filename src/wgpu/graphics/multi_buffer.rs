use std::ops::RangeBounds;

use bytemuck::Pod;
use wgpu::{Buffer, BufferSlice, BufferUsages, Queue};

use super::{
    allocator::{Allocation, BufferAllocator},
    Gpu, TypedBuffer,
};

pub struct MultiBuffer<T> {
    label: String,
    buffer: TypedBuffer<T>,
    allocator: BufferAllocator,
}

impl<T> MultiBuffer<T>
where
    T: Pod,
{
    pub fn new(gpu: &Gpu, label: impl Into<String>, usage: BufferUsages, capacity: usize) -> Self {
        let label = label.into();
        let buffer = TypedBuffer::new_uninit(gpu, &label, usage, capacity);
        let allocator = BufferAllocator::new(capacity);

        Self {
            buffer,
            allocator,
            label,
        }
    }

    pub fn grow(&mut self, gpu: &Gpu, size: usize) {
        let size = (self.buffer.len() + size).next_power_of_two() - self.buffer.len();
        tracing::debug!(?size, "grow");
        self.allocator.grow(size);

        self.buffer.resize(gpu, self.allocator.total_size());
    }

    pub fn allocate(&mut self, len: usize) -> Option<Allocation> {
        self.allocator.allocate(len)
    }

    pub fn try_reallocate(&mut self, allocation: Allocation, new_len: usize) -> Option<Allocation> {
        if allocation.size() >= new_len {
            Some(allocation)
        } else {
            tracing::debug!("reallocating {allocation:?} to {new_len}");
            self.allocator.allocate(new_len)
        }
    }

    pub fn deallocate(&mut self, block: Allocation) {
        self.allocator.deallocate(block)
    }

    pub fn get(&self, block: &Allocation) -> BufferSlice {
        let start = block.start();
        let size = block.size();

        self.buffer.slice(start..start + size)
    }

    pub fn write(&self, queue: &Queue, allocation: &Allocation, data: &[T]) {
        assert!(
            data.len() <= allocation.size(),
            "write exceeds allocation {} > {}",
            data.len(),
            allocation.size()
        );

        self.buffer.write(queue, allocation.start(), data);
    }

    pub fn slice(&self, bounds: impl RangeBounds<usize>) -> BufferSlice {
        self.buffer.slice(bounds)
    }

    pub fn buffer(&self) -> &Buffer {
        self.buffer.buffer()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn label(&self) -> &str {
        self.label.as_ref()
    }
}
