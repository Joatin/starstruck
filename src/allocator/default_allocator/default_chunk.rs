use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::MemoryTypeId;
use std::ops::Range;
use std::sync::Arc;
use failure::Error;
use crate::allocator::Memory;
use crate::allocator::default_allocator::chunk::Chunk;

#[derive(Debug)]
pub struct DefaultChunk<B: Backend, D: Device<B>> {
    pub size: u32,
    pub free: u32,
    pub memory_id: MemoryTypeId,
    pub regions: Vec<Range<u32>>,
    pub memory: Arc<B::Memory>,
    pub device: Arc<D>,
    pub id: u64
}

impl<B: Backend, D: Device<B>> Chunk<B, D> for DefaultChunk<B, D> {
    fn new(device: Arc<D>, memory_id: MemoryTypeId, size: u32, id: u64) -> Result<Self, Error> {

        info!("Allocating new memory chunk that is {} bytes long", size);
        let memory = Arc::new(unsafe { device.allocate_memory(memory_id, u64::from(size)) }?);

        Ok(Self {
            size,
            free: size,
            memory_id,
            regions: vec![0..size],
            memory,
            device,
            id
        })
    }

    fn allocate(&mut self, size: u32) -> Result<Memory<B>, Error> {
        if size > self.free {
            bail!("No available region found")
        }
        match self.regions.iter_mut().find(|range| range.len() as u32 >= size).map(|range| {
            let start = range.start;
            range.start += size;

            start..range.start
        }) {
            Some(range) => {
                self.free -= range.len() as u32;
                Ok(Memory::new(range, self.id, Arc::clone(&self.memory)))
            },
            None => {
                bail!("No available region found")
            }
        }
    }

    fn deallocate(&mut self, memory: &mut Memory<B>) {
        let range = memory.memory_range();
        self.free += range.len() as u32;
        self.regions.push(range);
    }

    fn memory_id(&self) -> MemoryTypeId {
        self.memory_id
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use pretty_assertions::assert_ne;
    use gfx_hal::MemoryTypeId;
    use std::sync::Arc;
    use crate::allocator::default_allocator::default_chunk::DefaultChunk;
    use crate::allocator::default_allocator::chunk::Chunk;

    #[test]
    #[should_panic(expected = "not yet implemented")]
    #[allow(unused_must_use)]
    fn new_chunk_should_fail_by_calling_allocate() {
        DefaultChunk::new(Arc::new(gfx_backend_empty::Device), MemoryTypeId(0), 1000, 0);
    }

    #[test]
    fn allocate_should_return_memory_on_success() {
        let mut chunk = DefaultChunk {
            size: 1000,
            free: 1000,
            memory_id: MemoryTypeId(0),
            regions: vec![0..1000],
            memory: Arc::new(()),
            device: Arc::new(gfx_backend_empty::Device),
            id: 0
        };

        let mut memory = chunk.allocate(100).unwrap();

        // To avoid panic when dropping
        memory.set_freed();

        assert_eq!(100, memory.memory_range().len())

    }

    #[test]
    fn allocate_should_fail_if_no_memory_is_available() {
        let mut chunk = DefaultChunk {
            size: 1000,
            free: 1000,
            memory_id: MemoryTypeId(0),
            regions: vec![0..1000],
            memory: Arc::new(()),
            device: Arc::new(gfx_backend_empty::Device),
            id: 0
        };
        let error = chunk.allocate(10000).unwrap_err();

        assert_eq!("No available region found", format!("{}", error));
    }

    #[test]
    fn allocate_should_allow_multiple_subsequent_allocations() {
        let mut chunk = DefaultChunk {
            size: 1000,
            free: 1000,
            memory_id: MemoryTypeId(0),
            regions: vec![0..1000],
            memory: Arc::new(()),
            device: Arc::new(gfx_backend_empty::Device),
            id: 0
        };

        for _ in 0..10 {
            let mut memory = chunk.allocate(100).unwrap();
            // To avoid panic when dropping
            memory.set_freed();
        }

        // this should error
        let error = chunk.allocate(100).unwrap_err();
        assert_eq!("No available region found", format!("{}", error));
    }

    #[test]
    fn two_allocations_should_not_share_range() {
        let mut chunk = DefaultChunk {
            size: 1000,
            free: 1000,
            memory_id: MemoryTypeId(0),
            regions: vec![0..1000],
            memory: Arc::new(()),
            device: Arc::new(gfx_backend_empty::Device),
            id: 0
        };
        let mut memory1 = chunk.allocate(100).unwrap();
        let mut memory2 = chunk.allocate(100).unwrap();

        // To avoid panic when dropping
        memory1.set_freed();
        memory2.set_freed();

        assert_ne!(memory1.memory_range(), memory2.memory_range())
    }
}


#[cfg(all(feature = "unstable", test))]
mod bench {
    extern crate test;
    use self::test::Bencher;
    use crate::allocator::default_allocator::default_chunk::DefaultChunk;
    use gfx_hal::MemoryTypeId;
    use std::sync::Arc;
    use bencher::black_box;
    use crate::allocator::default_allocator::chunk::Chunk;

    #[bench]
    fn allocate_bench(b: &mut Bencher) {
        let mut chunk = DefaultChunk {
            size: 10000,
            free: 10000,
            memory_id: MemoryTypeId(0),
            regions: vec![0..10000],
            memory: Arc::new(()),
            device: Arc::new(gfx_backend_empty::Device),
            id: 0
        };

        b.iter(|| {
            let mut memory = chunk.allocate(100).unwrap();
            // To avoid panic when dropping
            memory.set_freed();
            chunk.deallocate(&mut memory);
            black_box(memory);
        })
    }

    #[bench]
    fn allocate_to_big_bench(b: &mut Bencher) {
        let mut chunk = DefaultChunk {
            size: 10,
            free: 10,
            memory_id: MemoryTypeId(0),
            regions: vec![0..10],
            memory: Arc::new(()),
            device: Arc::new(gfx_backend_empty::Device),
            id: 0
        };

        b.iter(|| {
            let result = chunk.allocate(1000);
            black_box(result);
        })
    }



}