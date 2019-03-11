use crate::allocator::GpuAllocator;
use gfx_hal::MemoryTypeId;
use failure::Error;
use gfx_hal::Backend;
use std::sync::Arc;
use gfx_hal::Device;
use crate::allocator::Memory;
use std::sync::Mutex;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use crate::allocator::default_allocator::chunk::Chunk;
use crate::allocator::default_allocator::default_chunk::DefaultChunk;
use std::marker::PhantomData;


#[derive(Debug)]
pub struct DefaultGpuAllocator<C: Chunk<B, D> = DefaultChunk<backend::Backend, backend::Device>, B: Backend = backend::Backend, D: Device<B> = backend::Device> {
    device: Option<Arc<D>>,
    chunks: Mutex<HashMap<u64, C>>,
    next_id: AtomicUsize,
    phantom: PhantomData<B>
}

impl<C: Chunk<B, D>, B: Backend, D: Device<B>> DefaultGpuAllocator<C, B, D> {
    const CHUNK_DEFAULT_SIZE: u32 = 67_108_864; // 64mb

    pub fn new() -> Self {
        Self {
            device: None,
            chunks: Mutex::new(HashMap::new()),
            next_id: AtomicUsize::new(0),
            phantom: PhantomData
        }
    }

    fn create_chunk_and_get_memory(&self, memory_id: MemoryTypeId, chunk_size: u32, memory_size: u32) -> Result<Memory<B>, Error> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as u64;
        let mut chunk = C::new(Arc::clone(self.device.as_ref().ok_or(format_err!("This allocator has not been initialized"))?), memory_id, chunk_size, id)?;
        let mem = chunk.allocate(memory_size).expect("This should always work since we just allocated it");
        {
            let mut lock = self.chunks.lock().unwrap();
            lock.insert(id, chunk);
        }
        Ok(mem)
    }
}

impl<C: Chunk<B, D>, B: Backend, D: Device<B>> GpuAllocator<B, D> for DefaultGpuAllocator<C, B, D> {

    fn init(&mut self, device: Arc<D>) {
        self.device = Some(device);
    }

    fn allocate_memory(&self, memory_id: MemoryTypeId, size: u64) -> Result<Memory<B>, Error> {
        if size as u32 <= Self::CHUNK_DEFAULT_SIZE {
            {
                let mut lock = self.chunks.lock().unwrap();
                for (_id, chunk) in &mut *lock {
                    if chunk.memory_id() == memory_id {
                        if let Ok(res) = chunk.allocate(size as u32) {
                            return Ok(res)
                        }
                    }
                }
            }
            self.create_chunk_and_get_memory(memory_id, Self::CHUNK_DEFAULT_SIZE, size as u32)
        } else {
            self.create_chunk_and_get_memory(memory_id, size as u32, size as u32)
        }
    }

    fn free_memory(&self, memory: &mut Memory<B>) {
        let mut lock = self.chunks.lock().unwrap();
        match lock.get_mut(&memory.chunk_id()) {
            Some(chunk) => {
                chunk.deallocate(memory);
            },
            None => {
                error!("Chunk has already been freed! This should not happen! Seems like we are over freeing memory");
            }
        }
        memory.set_freed();
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use crate::allocator::DefaultGpuAllocator;
    use crate::allocator::default_allocator::chunk::Chunk;
    use gfx_hal::MemoryTypeId;
    use std::sync::Arc;
    use failure::Error;
    use crate::allocator::Memory;
    use crate::allocator::gpu_allocator::GpuAllocator;

    struct MockChunk;

    impl Chunk<gfx_backend_empty::Backend, gfx_backend_empty::Device> for MockChunk {
        fn new(_device: Arc<gfx_backend_empty::Device>, _memory_id: MemoryTypeId, _size: u32, _id: u64) -> Result<Self, Error> {
            Ok(MockChunk)
        }

        fn allocate(&mut self, _size: u32) -> Result<Memory<gfx_backend_empty::Backend>, Error> {
            unimplemented!()
        }

        fn deallocate(&mut self, _memory: &mut Memory<gfx_backend_empty::Backend>) {
            unimplemented!()
        }

        fn memory_id(&self) -> MemoryTypeId {
            unimplemented!()
        }
    }

    #[test]
    fn default_chunk_size_should_be_correct() {
        assert_eq!(67_108_864, DefaultGpuAllocator::<MockChunk, gfx_backend_empty::Backend, gfx_backend_empty::Device>::CHUNK_DEFAULT_SIZE);
    }

    #[test]
    fn allocator_should_throw_if_not_initialized() {
        let allocator = DefaultGpuAllocator::<MockChunk, gfx_backend_empty::Backend, gfx_backend_empty::Device>::new();
        let error = allocator.allocate_memory(MemoryTypeId(1), 1000).unwrap_err();
        assert_eq!("This allocator has not been initialized", format!("{}", error));
    }

}