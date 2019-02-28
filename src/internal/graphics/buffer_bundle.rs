use gfx_hal::memory::Requirements;
use core::mem::{size_of, ManuallyDrop};
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::MemoryTypeId;
use gfx_hal::PhysicalDevice;
use gfx_hal::buffer::{Usage as BufferUsage};
use gfx_hal::memory::Properties;
use gfx_hal::command::BufferCopy;
use gfx_hal::pool::CommandPoolCreateFlags;
use gfx_hal::Transfer;
use gfx_hal::command::CommandBuffer;
use gfx_hal::command::Primary;
use gfx_hal::command::OneShot;
use gfx_hal::Adapter;
use crate::primitive::Vertex;
use gfx_hal::Gpu;
use gfx_hal::QueueGroup;
use gfx_hal::queue::family::QueueFamily;
use failure::Error;
use crate::primitive::Index;
use futures::lazy;
use futures::future::poll_fn;
use futures::prelude::*;
use std::sync::Arc;
use futures::task::current;
use crate::internal::FenceExt;
use colored::*;


pub struct BufferBundle {
    pub buffer: ManuallyDrop<<backend::Backend as Backend>::Buffer>,
    memory: ManuallyDrop<<backend::Backend as Backend>::Memory>,
    queue_group: QueueGroup<backend::Backend, Transfer>,
    device: Arc<backend::Device>,
    requirements: Requirements
}

// TODO: We have to wait for copy operation to complete and then delete the old buffer
impl BufferBundle {
    fn create_async(adapter: Arc<Adapter<backend::Backend>>, buffer_len: u64, usage: BufferUsage, memory_properties: Properties) -> impl Future<Item=Self, Error=Error> + Send {
        lazy( move || {
            Self::create(&adapter, buffer_len, usage, memory_properties)
        })
    }

    fn create(adapter: &Adapter<backend::Backend>, buffer_len: u64, usage: BufferUsage, memory_properties: Properties) -> Result<Self, Error> {
        // Open A Device and take out a QueueGroup
        let (device, queue_group) = {
            let queue_family = adapter
                .queue_families
                .iter()
                .find(|qf| qf.supports_transfer())
                .ok_or_else(|| format_err!("Couldn't find a QueueFamily with Transfer!"))?;
            let Gpu { device, mut queues } = unsafe {
                adapter
                    .physical_device
                    .open(&[(&queue_family, &[1.0; 1])])?
            };
            let queue_group = queues
                .take::<Transfer>(queue_family.id())
                .ok_or_else(|| format_err!("Couldn't take ownership of the QueueGroup!"))?;
            if !queue_group.queues.is_empty() {
                Ok(())
            } else {
                Err(format_err!("The QueueGroup did not have any CommandQueues available!"))
            }?;
            (device, queue_group)
        };

        info!("{} {} {} {} {}", "Allocating new buffer of type".green(), format!("{:?}", usage).yellow(), "that is".green(), buffer_len.to_string().yellow(), "bytes long".green());
        unsafe {
            let mut buffer = device.create_buffer(buffer_len, usage)?;
            let requirements = device.get_buffer_requirements(&buffer);

            let memory_type_id = adapter
                .physical_device
                .memory_properties()
                .memory_types
                .iter()
                .enumerate()
                .find(|&(id, memory_type)| {
                    requirements.type_mask & (1 << id) != 0 && memory_type.properties.contains(memory_properties)
                })
                .map(|(id, _)| MemoryTypeId(id))
                .ok_or_else(|| format_err!("Couldn't find a memory type to support the buffer!"))?;

            let memory = device.allocate_memory(memory_type_id, requirements.size)?;

            device.bind_buffer_memory(&memory, 0, &mut buffer)?;

            Ok(BufferBundle {
                buffer: ManuallyDrop::new(buffer),
                memory: ManuallyDrop::new(memory),
                requirements,
                device: Arc::new(device),
                queue_group
            })
        }
    }

    fn create_staged_bundle<D :Copy + Send + Sync>(adapter: Arc<Adapter<backend::Backend>>, buffer_size: u64, usage: BufferUsage, data: Arc<Vec<D>>) -> impl Future<Item=BufferBundle, Error=Error> + Send {
        BufferBundle::create_async(Arc::clone(&adapter),buffer_size, BufferUsage::TRANSFER_SRC, Properties::CPU_VISIBLE)
            .and_then( move |s_buffer| {
                s_buffer.bind_data_async(data).and_then( move |bound_buffer| {
                BufferBundle::create_async(adapter, buffer_size, BufferUsage::TRANSFER_DST | usage, Properties::DEVICE_LOCAL)
                    .and_then( move|v_bundle| {
                        bound_buffer.copy_data_into_other(v_bundle, buffer_size)
                })
            })
        })
    }

    pub fn copy_data_into_other(mut self, dst: BufferBundle, buffer_size: u64) -> impl Future<Item=Self, Error=Error> + Send {
        let bundle = dst;
        let device = Arc::clone(&self.device);

        Box::new(lazy(move || {
            unsafe {
                let queue_group  = &mut self.queue_group;
                let (buff, mut poo) = {

                    let mut pool = device.create_command_pool_typed(&queue_group, CommandPoolCreateFlags::TRANSIENT).unwrap();
                    let mut buffer: CommandBuffer<backend::Backend, Transfer, OneShot, Primary> = pool.acquire_command_buffer();


                    info!("Copying one buffer into another");
                    buffer.begin();
                    buffer.copy_buffer(&self.buffer, &bundle.buffer, &[BufferCopy {
                        src: 0,
                        dst: 0,
                        size: buffer_size
                    }]);
                    buffer.finish();
                    (buffer, pool)
                };

                let raw_fence = device.create_fence(false).unwrap();
                let fence = Arc::new(raw_fence.into_promise(Arc::clone(&device)));
                let queue = &mut queue_group.queues[0];
                queue.submit_nosemaphores(&[buff], Some(&fence));
                poo.reset();

                let fut = poll_fn(move || {
                    match device.get_fence_status(&fence) {
                        Ok(signaled) => {
                            if signaled {
                                Ok(Async::Ready(()))
                            } else {
                                current().notify();
                                Ok(Async::NotReady)
                            }
                        },
                        Err(e) => bail!(e)
                    }
                });

                Ok(fut.map(move |_| {
                    drop(self);
                    bundle
                }))
            }
        }).and_then(|poll| {
            poll
        }))
    }

     fn bind_data_async<D :Copy + Send + Sync>(self, points: Arc<Vec<D>>) -> impl Future<Item=Self, Error=Error> + Send {
         Box::new(lazy(move || {
             self.bind_data(points)?;
             Ok(self)
         }))
     }

    fn bind_data<D :Copy + Send + Sync>(&self, points: Arc<Vec<D>>) -> Result<(), Error> {
        // Write the index data just once.
        info!("Writing data into buffer");
        unsafe {
            let mut data_target = self
                .device
                .acquire_mapping_writer(&self.memory, 0..self.requirements.size)?;
            data_target[..points.len()].copy_from_slice(&points);
            self.device
                .release_mapping_writer(data_target)?;
        }
        Ok(())
    }
    pub fn new_vertex<D: Vertex>(adapter: Arc<Adapter<backend::Backend>>, points: Arc<Vec<D>>) -> impl Future<Item=BufferBundle, Error=Error> + Send {
        let buffer_size = (D::stride() * points.len()) as u64;
        BufferBundle::create_staged_bundle(adapter, buffer_size, BufferUsage::VERTEX, points)
    }
    pub fn new_index<D: Index>(adapter: Arc<Adapter<backend::Backend>>, points: Arc<Vec<D>>) -> impl Future<Item=BufferBundle, Error=Error> + Send {
        let buffer_size = (size_of::<D>() * points.len()) as u64;
        BufferBundle::create_staged_bundle(adapter, buffer_size, BufferUsage::INDEX, points)
    }
}

impl Drop for BufferBundle {
    fn drop(&mut self) {
        use core::ptr::read;

        info!("{} {} {}", "Dropping buffer bundle,".red(), self.requirements.size.to_string().yellow(), "bytes of memory will be freed".red());

        let device = &self.device;
        let buffer = &self.buffer;
        let memory  = &self.memory;

        unsafe {
            device.destroy_buffer(ManuallyDrop::into_inner(read(buffer)));
            device.free_memory(ManuallyDrop::into_inner(read(memory)));
        }
    }
}