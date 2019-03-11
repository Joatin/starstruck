use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::MemoryTypeId;
use std::sync::Arc;
use failure::Error;
use crate::allocator::Memory;

pub trait Chunk<B: Backend, D: Device<B>>: Send + Sync + 'static where Self: std::marker::Sized {
    fn new(device: Arc<D>, memory_id: MemoryTypeId, size: u32, id: u64) -> Result<Self, Error>;
    fn allocate(&mut self, size: u32) -> Result<Memory<B>, Error>;
    fn deallocate(&mut self, memory: &mut Memory<B>);
    fn memory_id(&self) -> MemoryTypeId;
}