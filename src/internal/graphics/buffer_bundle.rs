use crate::internal::graphics::GraphicsState;
use crate::internal::FenceExt;
use colored::*;
use core::mem::{size_of, ManuallyDrop};
use failure::Error;
use futures::future::poll_fn;
use futures::lazy;
use futures::prelude::*;
use futures::task::current;
use gfx_hal::buffer::Usage as BufferUsage;
use gfx_hal::command::BufferCopy;
use gfx_hal::command::CommandBuffer;
use gfx_hal::command::OneShot;
use gfx_hal::command::Primary;
use gfx_hal::memory::Properties;
use gfx_hal::memory::Requirements;
use gfx_hal::pool::CommandPoolCreateFlags;
use gfx_hal::queue::family::QueueFamily;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Gpu;
use gfx_hal::Instance;
use gfx_hal::Limits;
use gfx_hal::MemoryTypeId;
use gfx_hal::PhysicalDevice;
use gfx_hal::QueueGroup;
use gfx_hal::Transfer;
use image::RgbaImage;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::marker::PhantomData;
use std::sync::Arc;

pub trait BufferBundlePlace {}
pub struct CPU {}
pub struct GPU {}
impl BufferBundlePlace for CPU {}
impl BufferBundlePlace for GPU {}

/// A buffer bundle contain all resources necessary to maintain a bundle. It also contains a reference to the device used to create in order to take care of it's own destructuring.
pub struct BufferBundle<
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>,
    P: BufferBundlePlace,
    T: Copy + Send + Sync,
> {
    pub buffer: ManuallyDrop<B::Buffer>,
    pub memory: ManuallyDrop<B::Memory>,
    pub queue_group: QueueGroup<B, Transfer>,
    state: Arc<GraphicsState<B, D, I>>,
    pub requirements: Requirements,
    buffer_len: u64,
    usage: BufferUsage,
    phantom: PhantomData<T>,
    phantom_place: PhantomData<fn() -> P>,
}

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>, T: Copy + Send + Sync>
    BufferBundle<B, D, I, CPU, T>
{
    pub fn new(
        state: Arc<GraphicsState<B, D, I>>,
        buffer_len: u64,
        usage: BufferUsage,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        Self::create_buffer(state, buffer_len, usage, Properties::CPU_VISIBLE)
    }

    pub fn write_data(self, data: Arc<Vec<T>>) -> impl Future<Item = Self, Error = Error> + Send {
        lazy(move || {
            info!("Writing data into buffer");
            unsafe {
                let mut writer = self
                    .state
                    .device()
                    .acquire_mapping_writer(&self.memory, 0..self.requirements.size)?;
                writer[..data.len()].copy_from_slice(&data);
                self.state.device().release_mapping_writer(writer)?;
            }
            Ok(self)
        })
    }
}

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>>
    BufferBundle<B, D, I, CPU, image::Rgba<u8>>
{
    pub fn write_image_data(
        self,
        image: RgbaImage,
        limits: Limits,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        lazy(move || {
            info!("Writing data into buffer");
            unsafe {
                let pixel_size = size_of::<image::Rgba<u8>>();
                let row_size = pixel_size * (image.width() as usize);
                let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
                let row_pitch =
                    ((row_size as u32 + row_alignment_mask) & !row_alignment_mask) as usize;
                debug_assert!(row_pitch as usize >= row_size);

                let mut writer = self
                    .state
                    .device()
                    .acquire_mapping_writer(&self.memory, 0..self.requirements.size)?;

                for y in 0..image.height() as usize {
                    let row = &(*image)[y * row_size..(y + 1) * row_size];
                    let dest_base = y * row_pitch;
                    writer[dest_base..dest_base + row.len()].copy_from_slice(row);
                }

                self.state.device().release_mapping_writer(writer)?;
            }
            Ok(self)
        })
    }
}

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>, T: Copy + Send + Sync>
    BufferBundle<B, D, I, GPU, T>
{
    pub fn new(
        state: Arc<GraphicsState<B, D, I>>,
        usage: BufferUsage,
        data: Arc<Vec<T>>,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        let buffer_len = (size_of::<T>() * data.len()) as _;
        let bundle_future = Self::create_buffer(
            Arc::clone(&state),
            buffer_len,
            usage | BufferUsage::TRANSFER_DST,
            Properties::DEVICE_LOCAL,
        );
        let transfer_bundle_future = BufferBundle::<B, D, I, CPU, T>::create_buffer(
            state,
            buffer_len,
            BufferUsage::TRANSFER_SRC,
            Properties::CPU_VISIBLE,
        );

        bundle_future
            .join(transfer_bundle_future)
            .and_then(move |(bundle, transfer_bundle)| {
                transfer_bundle
                    .write_data(data)
                    .and_then(move |t_bundle| bundle.import_data_from(t_bundle))
            })
    }

    fn import_data_from(
        mut self,
        source: BufferBundle<B, D, I, CPU, T>,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        lazy(move || unsafe {
            let device = self.state.device();
            let queue_group = &mut self.queue_group;
            let (buff, mut poo) = {
                let mut pool = device
                    .create_command_pool_typed(&queue_group, CommandPoolCreateFlags::TRANSIENT)?;
                let mut buffer: CommandBuffer<B, Transfer, OneShot, Primary> =
                    pool.acquire_command_buffer();

                info!("Copying one buffer into another");
                buffer.begin();
                buffer.copy_buffer(
                    &source.buffer,
                    &self.buffer,
                    &[BufferCopy {
                        src: 0,
                        dst: 0,
                        size: self.buffer_len,
                    }],
                );
                buffer.finish();
                (buffer, pool)
            };

            let raw_fence = device.create_fence(false).unwrap();
            let fence = Arc::new(raw_fence.into_promise(Arc::clone(&device)));
            let queue = &mut queue_group.queues[0];
            queue.submit_nosemaphores(&[buff], Some(&fence));
            poo.reset();

            Ok((device, fence, self))
        })
        .and_then(|(dev, fe, se)| {
            poll_fn(move || unsafe {
                match dev.get_fence_status(&fe) {
                    Ok(signaled) => {
                        if signaled {
                            Ok(Async::Ready(()))
                        } else {
                            current().notify();
                            Ok(Async::NotReady)
                        }
                    }
                    Err(e) => bail!(e),
                }
            })
            .map(|_| se)
        })
    }
}

impl<
        B: Backend,
        D: Device<B>,
        I: Instance<Backend = B>,
        P: BufferBundlePlace,
        T: Copy + Send + Sync,
    > BufferBundle<B, D, I, P, T>
{
    pub fn create_buffer(
        state: Arc<GraphicsState<B, D, I>>,
        buffer_len: u64,
        usage: BufferUsage,
        memory_properties: Properties,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        lazy(move || {
            let queue_group = {
                let queue_family = state
                    .adapter()
                    .queue_families
                    .iter()
                    .find(|qf| qf.supports_transfer())
                    .ok_or_else(|| format_err!("Couldn't find a QueueFamily with Transfer!"))?;
                let Gpu {
                    mut queues, ..
                } = unsafe {
                    state
                        .adapter()
                        .physical_device
                        .open(&[(&queue_family, &[1.0; 1])])?
                };
                let queue_group = queues
                    .take::<Transfer>(queue_family.id())
                    .ok_or_else(|| format_err!("Couldn't take ownership of the QueueGroup!"))?;
                if !queue_group.queues.is_empty() {
                    Ok(())
                } else {
                    Err(format_err!(
                        "The QueueGroup did not have any CommandQueues available!"
                    ))
                }?;
                queue_group
            };

            info!(
                "{} {} {} {} {}",
                "Allocating new buffer of type".green(),
                format!("{:?}", usage).yellow(),
                "that is".green(),
                buffer_len.to_string().yellow(),
                "bytes long".green()
            );
            unsafe {
                let mut buffer = state.device().create_buffer(buffer_len, usage)?;
                let requirements = state.device().get_buffer_requirements(&buffer);

                let memory_type_id = state
                    .adapter()
                    .physical_device
                    .memory_properties()
                    .memory_types
                    .iter()
                    .enumerate()
                    .find(|&(id, memory_type)| {
                        requirements.type_mask & (1 << id) != 0
                            && memory_type.properties.contains(memory_properties)
                    })
                    .map(|(id, _)| MemoryTypeId(id))
                    .ok_or_else(|| {
                        format_err!("Couldn't find a memory type to support the buffer!")
                    })?;

                let memory = state
                    .device()
                    .allocate_memory(memory_type_id, requirements.size)?;

                state.device().bind_buffer_memory(&memory, 0, &mut buffer)?;

                Ok(BufferBundle {
                    buffer: ManuallyDrop::new(buffer),
                    memory: ManuallyDrop::new(memory),
                    requirements,
                    state,
                    queue_group,
                    buffer_len,
                    usage,
                    phantom: PhantomData,
                    phantom_place: PhantomData,
                })
            }
        })
    }
}

impl<
        B: Backend,
        D: Device<B>,
        I: Instance<Backend = B>,
        P: BufferBundlePlace,
        T: Copy + Send + Sync,
    > Drop for BufferBundle<B, D, I, P, T>
{
    fn drop(&mut self) {
        use core::ptr::read;

        info!(
            "{} {} {}",
            "Dropping buffer bundle,".red(),
            self.requirements.size.to_string().yellow(),
            "bytes of memory will be freed".red()
        );

        let device = &self.state.device();
        let buffer = &self.buffer;
        let memory = &self.memory;

        unsafe {
            device.destroy_buffer(ManuallyDrop::into_inner(read(buffer)));
            device.free_memory(ManuallyDrop::into_inner(read(memory)));
        }
    }
}

impl<
        B: Backend,
        D: Device<B>,
        I: Instance<Backend = B>,
        P: BufferBundlePlace,
        T: Copy + Send + Sync,
    > Debug for BufferBundle<B, D, I, P, T>
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "buffer lenght: {}", self.buffer_len)?;
        write!(f, "usage: {:#?}", self.usage)?;
        write!(f, "requirements: {:#?}", self.requirements)?;
        Ok(())
    }
}
