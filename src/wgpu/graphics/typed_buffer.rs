use std::{marker::PhantomData, mem};

use bytemuck::Pod;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferDescriptor, BufferUsages, CommandEncoder, Queue,
};

use crate::wgpu::Gpu;

/// Type safe buffer
pub struct TypedBuffer<T> {
    buffer: Buffer,
    len: usize,
    _marker: PhantomData<T>,
}

impl<T> std::ops::Deref for TypedBuffer<T> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T> TypedBuffer<T>
where
    T: Pod,
{
    pub fn new(gpu: &Gpu, label: &str, usage: BufferUsages, data: &[T]) -> Self {
        let buffer = gpu.device.create_buffer_init(&BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage,
        });

        Self {
            buffer,
            len: data.len(),
            _marker: PhantomData,
        }
    }

    pub fn new_uninit(gpu: &Gpu, label: &str, usage: BufferUsages, len: usize) -> Self {
        let buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some(label),
            usage,
            size: (mem::size_of::<T>() as u64 * len as u64),
            mapped_at_creation: false,
        });

        Self {
            buffer,
            len,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn copy_from_buffer(&mut self, encoder: &mut CommandEncoder, src: &Self) {
        encoder.copy_buffer_to_buffer(&src.buffer, 0, &self.buffer, 0, src.len() as _)
    }

    pub fn write(&self, queue: &Queue, data: &[T]) {
        assert!(self.len() >= data.len());
        queue.write_buffer(self, 0, bytemuck::cast_slice(data));
    }
}
