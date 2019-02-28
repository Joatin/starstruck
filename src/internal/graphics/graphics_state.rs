use crate::errors::CreateEncoderError;
use crate::internal::graphics::SwapchainBundle;
use core::mem::ManuallyDrop;
use failure::Error;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::window::Extent2D;
use gfx_hal::{
    adapter::{Adapter, PhysicalDevice},
    device::Device,
    pool::{CommandPool, CommandPoolCreateFlags},
    queue::family::QueueGroup,
    Backend, Gpu, Graphics, Instance, QueueFamily, Surface,
};
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;
use std::sync::RwLock;
use winit::Window;

pub struct GraphicsState {
    command_pool: RwLock<ManuallyDrop<CommandPool<backend::Backend, Graphics>>>,
    queue_group: RwLock<QueueGroup<backend::Backend, Graphics>>,
    device: Arc<backend::Device>,
    adapter: Arc<Adapter<backend::Backend>>,
    _surface: RwLock<<backend::Backend as Backend>::Surface>,
    _instance: ManuallyDrop<backend::Instance>,
    swapchain: RwLock<SwapchainBundle>,
}

impl GraphicsState {
    pub fn new(title: &str, window: &Window) -> Result<Self, Error> {
        let instance = backend::Instance::create(title, 1);
        let mut surface = instance.create_surface(window);

        let adapters = instance.enumerate_adapters();

        info!(
            "Found the following physical devices: {:?}",
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

        info!("Selected gpu: {:?}", adapter.info.name);
        info!("Selected gpu: {:?}", adapter.physical_device.limits());

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
            let queue_group = queues
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
        let mut command_pool = unsafe {
            device
                .create_command_pool_typed(&queue_group, CommandPoolCreateFlags::RESET_INDIVIDUAL)?
        };

        let swapchain = SwapchainBundle::new(
            &adapter,
            Arc::clone(&device),
            window,
            &mut surface,
            &mut command_pool,
        )?;

        Ok(Self {
            _instance: ManuallyDrop::new(instance),
            _surface: RwLock::new(surface),
            adapter: Arc::new(adapter),
            device,
            queue_group: RwLock::new(queue_group),
            swapchain: RwLock::new(swapchain),
            command_pool: RwLock::new(ManuallyDrop::new(command_pool)),
        })
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

    pub fn next_encoder<
        F: FnOnce(RenderPassInlineEncoder<backend::Backend>) -> Result<(), Error>,
    >(
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

    pub fn adapter(&self) -> Arc<Adapter<backend::Backend>> {
        Arc::clone(&self.adapter)
    }

    pub fn device(&self) -> Arc<backend::Device> {
        Arc::clone(&self.device)
    }

    pub fn render_pass<
        T: FnOnce(&<backend::Backend as Backend>::RenderPass) -> Result<R, Error>,
        R,
    >(
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

impl Debug for GraphicsState {
    fn fmt(&self, formatter: &mut Formatter) -> core::result::Result<(), std::fmt::Error> {
        formatter.write_str("Graphics State")?;
        Ok(())
    }
}
