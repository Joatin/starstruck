use crate::allocator::Memory;
use gfx_hal::MemoryTypeId;
use failure::Error;
use gfx_hal::Backend;
use std::sync::Arc;
use gfx_hal::Device;

pub trait GpuAllocator<B: Backend = backend::Backend, D: Device<B> = backend::Device>: Send + Sync + 'static {
    fn init(&mut self, device: Arc<D>);
    fn allocate_memory(&self, memory_id: MemoryTypeId, size: u64) -> Result<Memory<B>, Error>;
    fn free_memory(&self, memory: &mut Memory<B>);
}