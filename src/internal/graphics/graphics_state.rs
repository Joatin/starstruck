use crate::errors::CreateEncoderError;
use crate::internal::graphics::SwapchainBundle;
use core::mem::ManuallyDrop;
use failure::Error;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::window::Extent2D;
use gfx_hal::Limits;
use gfx_hal::{
    adapter::{Adapter, PhysicalDevice},
    device::Device,
    pool::{CommandPool, CommandPoolCreateFlags},
    queue::family::QueueGroup,
    Backend, Gpu, Graphics, Instance, QueueFamily, Surface,
};
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::Arc;
use std::sync::RwLock;
use winit::Window;
use crate::allocator::GpuAllocator;
use crate::allocator::DefaultGpuAllocator;

pub struct GraphicsState<A: GpuAllocator<B, D> = DefaultGpuAllocator, B: Backend = backend::Backend, D: Device<B> = backend::Device, I: Instance<Backend = B> = backend::Instance> {
    command_pool: RwLock<ManuallyDrop<CommandPool<B, Graphics>>>,
    queue_group: RwLock<QueueGroup<B, Graphics>>,
    device: Arc<D>,
    adapter: Adapter<B>,
    _surface: RwLock<B::Surface>,
    _instance: ManuallyDrop<I>,
    swapchain: RwLock<SwapchainBundle<B, D>>,
    limits: Limits,
    allocator: A
}

impl<A: GpuAllocator<backend::Backend, backend::Device>> GraphicsState<A> {
    pub fn new(title: &str, window: &Window, mut allocator: A) -> Result<Self, Error> {
        let instance = backend::Instance::create(title, 1);
        let mut surface = instance.create_surface(window);
        let adapters = instance.enumerate_adapters();

        info!(
            "Found the following physical devices: {:#?}",
            adapters.iter().map(|a| &a.info).collect::<Vec<_>>()
        );

        // Select An Adapter
        let adapter = adapters
            .into_iter()
            .find(|a| {
                a.queue_families
                    .iter()
                    .any(|qf| qf.supports_graphics() && surface.supports_queue_family(qf))
            })
            .ok_or_else(|| format_err!("Couldn't find a graphical Adapter!"))?;

        let limits = adapter.physical_device.limits();

        info!("Selected gpu: {:#?}", adapter.info.name);
        info!("Selected gpu: {:#?}", &limits);

        // Open A Device and take out a QueueGroup
        let (device, queue_group) = {
            let queue_family = adapter
                .queue_families
                .iter()
                .find(|qf| {
                    qf.supports_graphics() && surface.supports_queue_family(qf)
                        || qf.supports_transfer()
                })
                .ok_or_else(|| format_err!("Couldn't find a QueueFamily with graphics!"))?;
            let Gpu { device, mut queues } = unsafe {
                adapter
                    .physical_device
                    .open(&[(&queue_family, &[1.0; 1])])?
            };
            let queue_group: QueueGroup<backend::Backend, Graphics> = queues
                .take::<Graphics>(queue_family.id())
                .ok_or_else(|| format_err!("Couldn't take ownership of the QueueGroup!"))?;
            if !queue_group.queues.is_empty() {
                Ok(())
            } else {
                Err(format_err!(
                    "The QueueGroup did not have any CommandQueues available!"
                ))
            }?;

            (Arc::new(device), queue_group)
        };

        // Create Our CommandPool
        let mut command_pool: CommandPool<backend::Backend, Graphics> = unsafe {
            device
                .create_command_pool_typed(&queue_group, CommandPoolCreateFlags::RESET_INDIVIDUAL)?
        };

        let swapchain = SwapchainBundle::<backend::Backend, backend::Device>::new(
            &adapter,
            Arc::clone(&device),
            window,
            &mut surface,
            &mut command_pool,
        )?;

        allocator.init(Arc::clone(&device));

        Ok(Self {
            _instance: ManuallyDrop::new(instance),
            _surface: RwLock::new(surface),
            adapter,
            device,
            queue_group: RwLock::new(queue_group),
            swapchain: RwLock::new(swapchain),
            command_pool: RwLock::new(ManuallyDrop::new(command_pool)),
            limits,
            allocator
        })
    }
}

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> GraphicsState<A, B, D, I> {
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    pub fn recreate_swapchain(&self, window: &Window) -> Result<(), Error> {
        let adapter = &self.adapter;
        let device = Arc::clone(&self.device);
        let surface = &mut self._surface.write().unwrap();
        let command_pool = &mut self.command_pool.write().unwrap();

        {
            let mut lock = self.swapchain.write().unwrap();
            *lock = SwapchainBundle::new(adapter, device, window, surface, command_pool)?;
        }
        info!("Swapchain recreated");
        Ok(())
    }

    pub fn next_encoder<F: FnOnce(RenderPassInlineEncoder<B>) -> Result<(), Error>>(
        &self,
        callback: F,
    ) -> Result<(), CreateEncoderError> {
        let mut lock = self.swapchain.write().unwrap();
        let encoder = lock.next_encoder()?;
        callback(encoder).unwrap();
        Ok(())
    }

    pub fn present_swapchain(&self) -> Result<(), Error> {
        let mut lock = self.swapchain.write().unwrap();
        lock.present_swapchain(&mut self.queue_group.write().unwrap())
    }

    pub fn adapter(&self) -> &Adapter<B> {
        &self.adapter
    }

    pub fn allocator(&self) -> &GpuAllocator<B, D> {
        &self.allocator
    }

    pub fn device(&self) -> Arc<D> {
        Arc::clone(&self.device)
    }

    pub fn logical_window_size(&self) -> (u32, u32) {
        let lock = self.swapchain.read().unwrap();
        lock.logical_window_size()
    }

    pub fn dpi(&self) -> f64 {
        let lock = self.swapchain.read().unwrap();
        lock.dpi()
    }

    pub fn render_pass<T: FnOnce(&B::RenderPass) -> Result<R, Error>, R>(
        &self,
        callback: T,
    ) -> Result<R, Error> {
        let lock = self.swapchain.read().unwrap();
        callback(lock.render_pass())
    }

    pub fn render_area(&self) -> Extent2D {
        let lock = self.swapchain.read().unwrap();
        lock.render_area()
    }
}

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug for GraphicsState<A, B, D, I> {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        formatter.write_str("Graphics State")?;
        Ok(())
    }
}

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Display for GraphicsState<A, B, D, I> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "Graphics State")?;
        Ok(())
    }
}
