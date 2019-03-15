use gfx_hal::Backend;
use failure::Error;
use std::ops::Range;
use gfx_hal::Device;
use std::sync::Arc;
use gfx_hal::mapping::Writer;
use colored::*;

#[derive(Debug)]
pub struct  Memory<B: Backend> {
    range: Range<u32>,
    chunk_id: u64,
    memory: Arc<B::Memory>,
    is_freed: bool,
    is_allocated: bool
}

impl<B: Backend> Memory<B> {
    pub fn new(range: Range<u32>, chunk_id: u64, memory: Arc<B::Memory>) -> Self {
        Self {
            range,
            chunk_id,
            memory,
            is_freed: false,
            is_allocated: false
        }
    }

    pub unsafe fn bind_buffer_memory<D: Device<B>>(&mut self, device: &Arc<D>, buffer: &mut B::Buffer) -> Result<(), Error> {
        if !self.is_freed && !self.is_allocated {
            device.bind_buffer_memory(&self.memory, u64::from(self.range.start), buffer)?;
            Ok(())
        } else if self.is_freed {
            bail!("Can't bind to already freed memory!")
        } else {
            bail!("This memory is already allocated")
        }
    }

    pub unsafe fn acquire_mapping_writer<D: Device<B>, T: Copy>(&mut self, device: &Arc<D>, range: Range<u64>) -> Result<Writer<B, T>, Error> {
        let start = u64::from(self.range.start) + range.start;
        let end = u64::from(self.range.end) + range.end;
        Ok(device.acquire_mapping_writer(&self.memory, start..end)?)
    }

    pub unsafe fn bind_image_memory<D: Device<B>>(&mut self, device: &Arc<D>, offset: u64, image: &mut B::Image) -> Result<(), Error> {
        device.bind_image_memory(&self.memory, offset, image)?;
        Ok(())
    }

    pub fn is_freed(&self) -> bool {
        self.is_freed
    }

    pub fn is_allocated(&self) -> bool {
        self.is_allocated
    }

    pub fn chunk_id(&self) -> u64 {
        self.chunk_id
    }

    pub fn memory_range(&self) -> Range<u32> {
        self.range.clone()
    }

    pub(crate) fn set_freed(&mut self) {
        self.is_freed = true;
    }
}

impl<B: Backend> Drop for Memory<B> {
    fn drop(&mut self) {
        trace!(
            "{}", "Dropping memory".red());
        if !self.is_freed {
            error!("Memory dropped while still not freed! This is a memory leak!");
            panic!("Memory dropped while still not freed! This is a memory leak!")
        }
    }
}