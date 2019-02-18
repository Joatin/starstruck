use crate::errors::Result;
use arrayvec::ArrayVec;
use core::mem::{ManuallyDrop};
use gfx_hal::{
    adapter::{Adapter, PhysicalDevice},
    command::{ClearColor, ClearValue, CommandBuffer, MultiShot, Primary},
    device::Device,
    format::{Aspects, ChannelType, Format, Swizzle},
    image::{Access as ImageAccess, Layout, SubresourceRange, Usage, ViewKind},
    pass::{Attachment, AttachmentLoadOp, AttachmentOps, AttachmentStoreOp, SubpassDesc},
    pool::{CommandPool, CommandPoolCreateFlags},
    pso::{PipelineStage, Rect},
    queue::{family::QueueGroup, Submission},
    window::{Backbuffer, FrameSync, PresentMode, Swapchain, SwapchainConfig},
    Backend, Gpu, Graphics, Instance, QueueFamily, Surface
};
use winit::Window;
use gfx_hal::window::Extent2D;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::pass::SubpassDependency;
use gfx_hal::pass::SubpassRef;
use gfx_hal::command::ClearDepthStencil;
use crate::internal::graphics::depth_image::DepthImage;
use std::fmt::Debug;
use std::fmt::Formatter;


pub struct GraphicsState {
    pub current_frame: usize,
    pub frames_in_flight: usize,
    pub in_flight_fences: Vec<<backend::Backend as Backend>::Fence>,
    pub render_finished_semaphores: Vec<<backend::Backend as Backend>::Semaphore>,
    pub image_available_semaphores: Vec<<backend::Backend as Backend>::Semaphore>,
    pub command_buffers: Vec<CommandBuffer<backend::Backend, Graphics, MultiShot, Primary>>,
    pub command_pool: ManuallyDrop<CommandPool<backend::Backend, Graphics>>,
    pub framebuffers: Vec<<backend::Backend as Backend>::Framebuffer>,
    pub image_views: Vec<(<backend::Backend as Backend>::ImageView)>,
    pub render_pass: ManuallyDrop<<backend::Backend as Backend>::RenderPass>,
    pub render_area: Rect,
    pub queue_group: QueueGroup<backend::Backend, Graphics>,
    pub swapchain: ManuallyDrop<<backend::Backend as Backend>::Swapchain>,
    pub device: ManuallyDrop<backend::Device>,
    pub _adapter: Adapter<backend::Backend>,
    _surface: <backend::Backend as Backend>::Surface,
    _instance: ManuallyDrop<backend::Instance>,
    depth_images: Vec<DepthImage>,
    image_index: usize,
}

impl GraphicsState {

    pub fn new(title: &str, window: &Window) -> Result<Self> {
        let instance = backend::Instance::create(title, 1);
        let mut surface = instance.create_surface(window);

        // Select An Adapter
        let adapter = instance
            .enumerate_adapters()
            .into_iter()
            .find(|a| {
                a.queue_families
                    .iter()
                    .any(|qf| qf.supports_graphics() && surface.supports_queue_family(qf))
            })
            .ok_or("Couldn't find a graphical Adapter!")?;

        // Open A Device and take out a QueueGroup
        let (device, queue_group) = {
            let queue_family = adapter
                .queue_families
                .iter()
                .find(|qf| qf.supports_graphics() && surface.supports_queue_family(qf) || qf.supports_transfer())
                .ok_or("Couldn't find a QueueFamily with graphics!")?;
            let Gpu { device, mut queues } = unsafe {
                adapter
                    .physical_device
                    .open(&[(&queue_family, &[1.0; 1])])
                    .map_err(|_| "Couldn't open the PhysicalDevice!")?
            };
            let queue_group = queues
                .take::<Graphics>(queue_family.id())
                .ok_or("Couldn't take ownership of the QueueGroup!")?;
            if !queue_group.queues.is_empty() {
                Ok(())
            } else {
                Err("The QueueGroup did not have any CommandQueues available!")
            }?;
            (device, queue_group)
        };


        // Create Our CommandPool
        let mut command_pool = unsafe {
            device
                .create_command_pool_typed(&queue_group, CommandPoolCreateFlags::RESET_INDIVIDUAL)
                .map_err(|_| "Could not create the raw command pool!")?
        };

        let (
            swapchain,
            extent,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            render_pass,
            image_views,
            framebuffers,
            command_buffers,
            frames_in_flight,
            depth_images
        ) = Self::setup_swapchain_and_depth_image(&device, &window, &adapter, &mut surface, &mut command_pool).unwrap();

        Ok(Self {
            _instance: ManuallyDrop::new(instance),
            _surface: surface,
            _adapter: adapter,
            device: ManuallyDrop::new(device),
            queue_group,
            swapchain: ManuallyDrop::new(swapchain),
            render_area: Rect {
                x: 0,
                y: 0,
                w: extent.width as i16,
                h: extent.height as i16
            },
            render_pass: ManuallyDrop::new(render_pass),
            image_views,
            framebuffers,
            command_pool: ManuallyDrop::new(command_pool),
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            frames_in_flight,
            current_frame: 0,
            depth_images,
            image_index: 0
        })
    }

    pub fn next_encoder(&mut self) -> Result<RenderPassInlineEncoder<backend::Backend>> {
        let encoder = unsafe {
            let flight_fence = &self.in_flight_fences[self.current_frame];
            // Advance the frame _before_ we start using the `?` operator
            self.current_frame = (self.current_frame + 1) % self.frames_in_flight;

            self
                .device
                .wait_for_fence(flight_fence, core::u64::MAX)
                .map_err(|_| "Failed to wait on the fence!")?;
            self
                .device
                .reset_fence(flight_fence)
                .map_err(|_| "Couldn't reset the fence!")?;
            let image_index = self
                .swapchain
                .acquire_image(core::u64::MAX, FrameSync::Semaphore(
                    &self.image_available_semaphores[self.current_frame]
                ))
                .map_err(|_| "Couldn't acquire an image from the swapchain!")?;
            self.image_index = image_index as usize;


            let clear_values = [ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0])), ClearValue::DepthStencil(ClearDepthStencil(1.0, 0))];

            self.command_buffers[self.image_index].begin(false);
            self.command_buffers[self.image_index].begin_render_pass_inline(
                &self.render_pass,
                &self.framebuffers[self.image_index],
                self.render_area,
                clear_values.iter(),
            )
        };
        Ok(encoder)
    }

    pub fn present_swapchain(&mut self) -> Result<()> {
        unsafe {
            self.command_buffers[self.image_index].finish();

            let flight_fence = &self.in_flight_fences[self.current_frame];
            let image_available = &self.image_available_semaphores[self.current_frame];
            let render_finished = &self.render_finished_semaphores[self.current_frame];

            let command_buffers = &self.command_buffers[self.image_index..=self.image_index];
            let wait_semaphores: ArrayVec<[_; 1]> =
                [(image_available, PipelineStage::COLOR_ATTACHMENT_OUTPUT)].into();
            let signal_semaphores: ArrayVec<[_; 1]> = [render_finished].into();
            // yes, you have to write it twice like this. yes, it's silly.
            let present_wait_semaphores: ArrayVec<[_; 1]> = [render_finished].into();
            let submission = Submission {
                command_buffers,
                wait_semaphores,
                signal_semaphores,
            };
            let the_command_queue = &mut self.queue_group.queues[0];
            the_command_queue.submit(submission, Some(flight_fence));
            self
                .swapchain
                .present(the_command_queue, self.image_index as u32, present_wait_semaphores)
                .map_err(|_| "Failed to present into the swapchain!")?;
        };

        Ok(())
    }

    fn setup_swapchain_and_depth_image(device: &backend::Device, window: &Window, adapter: &Adapter<backend::Backend>, surface: &mut <backend::Backend as Backend>::Surface, command_pool: &mut CommandPool<backend::Backend, Graphics>) -> Result<(
        <backend::Backend as Backend>::Swapchain, Extent2D,
        Vec<<backend::Backend as Backend>::Semaphore>,
        Vec<<backend::Backend as Backend>::Semaphore>,
        Vec<<backend::Backend as Backend>::Fence>,
        <backend::Backend as Backend>::RenderPass,
        Vec<(<backend::Backend as Backend>::ImageView)>,
        Vec<<backend::Backend as Backend>::Framebuffer>,
        Vec<CommandBuffer<backend::Backend, Graphics, MultiShot, Primary>>,
        usize,
        Vec<DepthImage>
    )> {
        let (caps, preferred_formats, present_modes, composite_alphas) =
            surface.compatibility(&adapter.physical_device);
        info!("{:?}", caps);
        info!("Preferred Formats: {:?}", preferred_formats);
        info!("Present Modes: {:?}", present_modes);
        info!("Composite Alphas: {:?}", composite_alphas);

        // Find the window mode
        let present_mode = {
            use gfx_hal::window::PresentMode::*;
            [Mailbox, Fifo, Relaxed, Immediate]
                .iter()
                .cloned()
                .find(|pm| present_modes.contains(pm))
                .ok_or("No PresentMode values specified!")?
        };

        // Find window alpha
        let composite_alpha = {
            use gfx_hal::window::CompositeAlpha::*;
            [Opaque, Inherit, PreMultiplied, PostMultiplied]
                .iter()
                .cloned()
                .find(|ca| composite_alphas.contains(ca))
                .ok_or("No CompositeAlpha values specified!")?
        };

        // Select format
        let format = match preferred_formats {
            None => Format::Rgba8Srgb,
            Some(formats) => match formats
                .iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .cloned()
                {
                    Some(srgb_format) => srgb_format,
                    None => formats
                        .get(0)
                        .cloned()
                        .ok_or("Preferred format list was empty!")?,
                },
        };

        // Find the window size
        let extent = {
            let window_client_area = window.get_inner_size().ok_or("Window doesn't exist!")?;
            let dpi_factor = window.get_hidpi_factor();
            Extent2D {
                width: caps.extents.end.width.min((window_client_area.width * dpi_factor) as u32),
                height: caps.extents.end.height.min((window_client_area.height * dpi_factor) as u32),
            }
        };
        let image_count = if present_mode == PresentMode::Mailbox {
            (caps.image_count.end - 1).min(3)
        } else {
            (caps.image_count.end - 1).min(2)
        };
        let image_layers = 1;
        let image_usage = if caps.usage.contains(Usage::COLOR_ATTACHMENT) {
            Usage::COLOR_ATTACHMENT
        } else {
            Err("The Surface isn't capable of supporting color!")?
        };
        let swapchain_config = SwapchainConfig {
            present_mode,
            composite_alpha,
            format,
            extent,
            image_count,
            image_layers,
            image_usage,
        };
        info!("{:?}", swapchain_config);
        //
        let (swapchain, backbuffer) = unsafe {
            device
                .create_swapchain(surface, swapchain_config, None)
                .map_err(|_| "Failed to create the swapchain!")?
        };

        let frames_in_flight = image_count as usize;
        // Create Our Sync Primitives
        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) = {
            let mut image_available_semaphores: Vec<<backend::Backend as Backend>::Semaphore> = vec![];
            let mut render_finished_semaphores: Vec<<backend::Backend as Backend>::Semaphore> = vec![];
            let mut in_flight_fences: Vec<<backend::Backend as Backend>::Fence> = vec![];
            for _ in 0..frames_in_flight {
                in_flight_fences.push(
                    device
                        .create_fence(true)
                        .map_err(|_| "Could not create a fence!")?,
                );
                image_available_semaphores.push(
                    device
                        .create_semaphore()
                        .map_err(|_| "Could not create a semaphore!")?,
                );
                render_finished_semaphores.push(
                    device
                        .create_semaphore()
                        .map_err(|_| "Could not create a semaphore!")?,
                );
            }
            (
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
            )
        };

        // Define A RenderPass
        let render_pass = {
            let color_attachment = Attachment {
                format: Some(format),
                samples: 1,
                ops: AttachmentOps {
                    load: AttachmentLoadOp::Clear,
                    store: AttachmentStoreOp::Store,
                },
                stencil_ops: AttachmentOps::DONT_CARE,
                layouts: Layout::Undefined..Layout::Present,
            };
            let depth_attachment = Attachment {
                format: Some(Format::D32Float),
                samples: 1,
                ops: AttachmentOps {
                    load: AttachmentLoadOp::Clear,
                    store: AttachmentStoreOp::DontCare,
                },
                stencil_ops: AttachmentOps::DONT_CARE,
                layouts: Layout::Undefined..Layout::DepthStencilAttachmentOptimal,
            };
            let subpass = SubpassDesc {
                colors: &[(0, Layout::ColorAttachmentOptimal)],
                depth_stencil: Some(&(1, Layout::DepthStencilAttachmentOptimal)),
                inputs: &[],
                resolves: &[],
                preserves: &[],
            };
            let in_dependency = SubpassDependency {
                passes: SubpassRef::External..SubpassRef::Pass(0),
                stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT
                    ..PipelineStage::COLOR_ATTACHMENT_OUTPUT | PipelineStage::EARLY_FRAGMENT_TESTS,
                accesses: ImageAccess::empty()
                    ..(ImageAccess::COLOR_ATTACHMENT_READ
                    | ImageAccess::COLOR_ATTACHMENT_WRITE
                    | ImageAccess::DEPTH_STENCIL_ATTACHMENT_READ
                    | ImageAccess::DEPTH_STENCIL_ATTACHMENT_WRITE),
            };
            let out_dependency = SubpassDependency {
                passes: SubpassRef::Pass(0)..SubpassRef::External,
                stages: PipelineStage::COLOR_ATTACHMENT_OUTPUT | PipelineStage::EARLY_FRAGMENT_TESTS
                    ..PipelineStage::COLOR_ATTACHMENT_OUTPUT,
                accesses: (ImageAccess::COLOR_ATTACHMENT_READ
                    | ImageAccess::COLOR_ATTACHMENT_WRITE
                    | ImageAccess::DEPTH_STENCIL_ATTACHMENT_READ
                    | ImageAccess::DEPTH_STENCIL_ATTACHMENT_WRITE)..ImageAccess::empty(),
            };
            unsafe {
                device
                    .create_render_pass(&[color_attachment, depth_attachment],
                                        &[subpass],
                                        &[in_dependency, out_dependency])
                    .map_err(|_| "Couldn't create a render pass!")?
            }
        };

        // Create The ImageViews
        let image_views: Vec<_> = match backbuffer {
            Backbuffer::Images(images) => images
                .into_iter()
                .map(|image| unsafe {
                    device
                        .create_image_view(
                            &image,
                            ViewKind::D2,
                            format,
                            Swizzle::NO,
                            SubresourceRange {
                                aspects: Aspects::COLOR,
                                levels: 0..1,
                                layers: 0..1,
                            },
                        )
                        .map_err(|_| "Couldn't create the image_view for the image!")
                })
                .collect::<core::result::Result<Vec<_>, &str>>()?,
            Backbuffer::Framebuffer(_) => unimplemented!("Can't handle framebuffer backbuffer!"),
        };

        // Create Our FrameBuffers
        let depth_images = image_views
            .iter()
            .map(|_| DepthImage::new(&device, &adapter, extent))
            .collect::<core::result::Result<Vec<_>, &str>>()?;
        let image_extent = gfx_hal::image::Extent {
            width: extent.width as _,
            height: extent.height as _,
            depth: 1,
        };
        let framebuffers = image_views
            .iter()
            .zip(depth_images.iter())
            .map(|(view, depth_image)| unsafe {
                let attachments: ArrayVec<[_; 2]> = [view, &depth_image.image_view].into();
                device
                    .create_framebuffer(&render_pass, attachments, image_extent)
                    .map_err(|_| "Couldn't crate the framebuffer!")
            })
            .collect::<core::result::Result<Vec<_>, &str>>()?;

        // Create Our CommandBuffers
        let command_buffers: Vec<_> = framebuffers
            .iter()
            .map(|_| command_pool.acquire_command_buffer())
            .collect();

        Ok((swapchain, extent, image_available_semaphores, render_finished_semaphores, in_flight_fences, render_pass, image_views, framebuffers, command_buffers, frames_in_flight, depth_images))
    }
}

impl Debug for GraphicsState {
    fn fmt(&self, formatter: &mut Formatter) -> core::result::Result<(), std::fmt::Error> {
        formatter.write_str("Graphics State")?;
        Ok(())
    }
}