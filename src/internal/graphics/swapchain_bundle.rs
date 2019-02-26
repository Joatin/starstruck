use failure::Error;
use std::mem::ManuallyDrop;
use gfx_hal::Backend;
use gfx_hal::Adapter;
use winit::Window;
use gfx_hal::window::Extent2D;
use gfx_hal::PresentMode;
use gfx_hal::SwapchainConfig;
use gfx_hal::pass::Attachment;
use gfx_hal::pass::AttachmentOps;
use gfx_hal::pass::AttachmentLoadOp;
use gfx_hal::command::CommandBuffer;
use crate::internal::graphics::depth_image::DepthImage;
use gfx_hal::Graphics;
use gfx_hal::command::MultiShot;
use gfx_hal::command::Primary;
use gfx_hal::format::ChannelType;
use gfx_hal::Backbuffer;
use gfx_hal::pass::SubpassDependency;
use gfx_hal::pass::SubpassRef;
use gfx_hal::image::Layout;
use gfx_hal::pass::SubpassDesc;
use gfx_hal::pass::AttachmentStoreOp;
use gfx_hal::image::ViewKind;
use gfx_hal::format::Swizzle;
use gfx_hal::image::SubresourceRange;
use gfx_hal::format::Format;
use arrayvec::ArrayVec;
use gfx_hal::device::Device;
use gfx_hal::window::Surface;
use gfx_hal::window::Swapchain;
use gfx_hal::pso::PipelineStage;
use gfx_hal::image::Access as ImageAccess;
use gfx_hal::image::Usage;
use crate::errors::CreateEncoderError;
use gfx_hal::FrameSync;
use gfx_hal::command::ClearColor;
use gfx_hal::command::ClearValue;
use gfx_hal::Submission;
use gfx_hal::QueueGroup;
use gfx_hal::pso::Rect;
use gfx_hal::format::Aspects;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::command::ClearDepthStencil;
use gfx_hal::CommandPool;
use std::sync::Arc;
use gfx_hal::AcquireError;
use crate::errors::CreateEncoderErrorKind;
use colored::*;


pub struct SwapchainBundle {
    device: Arc<backend::Device>,
    swapchain: ManuallyDrop<<backend::Backend as Backend>::Swapchain>,
    command_buffers: Vec<CommandBuffer<backend::Backend, Graphics, MultiShot, Primary>>,
    in_flight_fences: Vec<<backend::Backend as Backend>::Fence>,
    render_finished_semaphores: Vec<<backend::Backend as Backend>::Semaphore>,
    image_available_semaphores: Vec<<backend::Backend as Backend>::Semaphore>,
    image_views: Vec<(<backend::Backend as Backend>::ImageView)>,
    render_pass: Arc<ManuallyDrop<<backend::Backend as Backend>::RenderPass>>,
    framebuffers: Vec<<backend::Backend as Backend>::Framebuffer>,
    depth_images: Vec<DepthImage>,
    render_area: Extent2D,
    current_frame: usize,
    image_index: usize,
    frames_in_flight: usize
}

impl SwapchainBundle {
    pub(crate) fn new(
        adapter: &Adapter<backend::Backend>,
        device: Arc<backend::Device>,
        window: &Window,
        surface: &mut <backend::Backend as Backend>::Surface,
        command_pool: &mut CommandPool<backend::Backend, Graphics>
    ) -> Result<Self, Error> {

        info!("{}", "Creating new swapchain".green());

        let (swapchain, backbuffer, format, render_area, image_count) = Self::create_swapchain(adapter, &device, window, surface)?;
        let render_pass = Self::create_render_pass(&device, format)?;


        let (image_available_semaphores, render_finished_semaphores, in_flight_fences) = {
            let mut image_available_semaphores: Vec<<backend::Backend as Backend>::Semaphore> = vec![];
            let mut render_finished_semaphores: Vec<<backend::Backend as Backend>::Semaphore> = vec![];
            let mut in_flight_fences: Vec<<backend::Backend as Backend>::Fence> = vec![];
            for _ in 0..image_count {
                in_flight_fences.push(device.create_fence(true)?);
                image_available_semaphores.push(device.create_semaphore()?);
                render_finished_semaphores.push(device.create_semaphore()?);
            }
            (
                image_available_semaphores,
                render_finished_semaphores,
                in_flight_fences,
            )
        };

        // Create The ImageViews
        let image_views = match backbuffer {
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
                })
                .collect::<Result<Vec<<backend::Backend as Backend>::ImageView>, gfx_hal::image::ViewError>>()?,
            Backbuffer::Framebuffer(_) => unimplemented!("Can't handle framebuffer backbuffer!"),
        };

        // Create Our FrameBuffers
        let depth_images = image_views
            .iter()
            .map(|_| DepthImage::new(Arc::clone(&device), &adapter, render_area))
            .collect::<core::result::Result<Vec<DepthImage>, Error>>()?;
        let image_extent = gfx_hal::image::Extent {
            width: render_area.width as _,
            height: render_area.height as _,
            depth: 1,
        };
        let framebuffers = image_views
            .iter()
            .zip(depth_images.iter())
            .map(|(view, depth_image)| unsafe {
                let attachments: ArrayVec<[_; 2]> = [view, &depth_image.image_view].into();
                device.create_framebuffer(&render_pass, attachments, image_extent)
            })
            .collect::<Result<Vec<<backend::Backend as Backend>::Framebuffer>, gfx_hal::device::OutOfMemory>>()?;

        // Create Our CommandBuffers
        let command_buffers: Vec<_> = framebuffers
            .iter()
            .map(|_| command_pool.acquire_command_buffer())
            .collect();


        Ok(Self {
            device,
            swapchain: ManuallyDrop::new(swapchain),
            command_buffers,
            image_views,
            render_pass: Arc::new(ManuallyDrop::new(render_pass)),
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            framebuffers,
            depth_images,
            render_area,
            current_frame: 0,
            image_index: 0,
            frames_in_flight: image_count
        })
    }

    pub fn render_pass(&self) -> Arc<ManuallyDrop<<backend::Backend as Backend>::RenderPass>> {
        Arc::clone(&self.render_pass)
    }

    pub fn render_area(&self) -> Extent2D {
        self.render_area
    }

    pub fn next_encoder(&mut self) -> Result<RenderPassInlineEncoder<backend::Backend>, CreateEncoderError> {
        let encoder = unsafe {
            let flight_fence = &self.in_flight_fences[self.current_frame];
            self.current_frame = (self.current_frame + 1) % self.frames_in_flight;

            if self.device.wait_for_fence(flight_fence, core::u64::MAX).is_err() {
                Err(CreateEncoderErrorKind::DeviceLost)?;
            }

            if self.device.reset_fence(flight_fence).is_err() {
                Err(CreateEncoderErrorKind::OutOfMemory)?;
            }

            //  Get the new index from the buffer
            self.image_index = self.swapchain
                .acquire_image(core::u64::MAX, FrameSync::Semaphore(
                    &self.image_available_semaphores[self.current_frame]
                )).map_err(|err| {
                match err {
                    AcquireError::NotReady => {
                        warn!("Unable to retrieve swapchain within timeout, this can be ignored");
                        CreateEncoderErrorKind::Timeout
                    },
                    AcquireError::OutOfDate => {
                        warn!("The swapchain is out of sync and needs to be reconstructed");
                        CreateEncoderErrorKind::RecreateSwapchain
                    },
                    AcquireError::SurfaceLost(_) => {
                        warn!("The old surface was lost perhaps you entered full screen");
                        CreateEncoderErrorKind::RecreateSwapchain
                    }
                }
            })? as _;


            let clear_values = [ClearValue::Color(ClearColor::Float([0.0, 0.0, 0.0, 1.0])), ClearValue::DepthStencil(ClearDepthStencil(1.0, 0))];
            self.command_buffers[self.image_index].begin(false);
            self.command_buffers[self.image_index].begin_render_pass_inline(
                &self.render_pass,
                &self.framebuffers[self.image_index],
                Rect {
                    x: 0,
                    y: 0,
                    h: self.render_area.width as _,
                    w: self.render_area.height as _
                },
                clear_values.iter(),
            )
        };
        Ok(encoder)
    }

    pub fn present_swapchain(&mut self, queue_group: &mut QueueGroup<backend::Backend, Graphics>) -> Result<(), Error> {
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
            let the_command_queue = &mut queue_group.queues[0];

            the_command_queue.submit(submission, Some(flight_fence));
            if self.swapchain.present(the_command_queue, self.image_index as u32, present_wait_semaphores).is_err() {
                //panic!() // No idea why this happens...
                warn!("No frame presented");
            };
        };

        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn create_swapchain(
        adapter: &Adapter<backend::Backend>,
        device: &backend::Device,
        window: &Window,
        surface: &mut <backend::Backend as Backend>::Surface
    ) -> Result<(<backend::Backend as Backend>::Swapchain, Backbuffer<backend::Backend>, Format, Extent2D, usize), Error> {
        let (caps, preferred_formats, present_modes, composite_alphas) = surface.compatibility(&adapter.physical_device);
        debug!("{:?}", caps);
        debug!("Preferred Formats: {:?}", preferred_formats);
        debug!("Present Modes: {:?}", present_modes);
        debug!("Composite Alphas: {:?}", composite_alphas);

        // Find the window mode
        let present_mode = {
            use gfx_hal::window::PresentMode::*;
            [Mailbox, Fifo, Relaxed, Immediate]
                .iter()
                .cloned()
                .find(|pm| present_modes.contains(pm))
                .ok_or_else(|| format_err!("No PresentMode values specified!"))?
        };

        // Find window alpha
        let composite_alpha = {
            use gfx_hal::window::CompositeAlpha::*;
            [Opaque, Inherit, PreMultiplied, PostMultiplied]
                .iter()
                .cloned()
                .find(|ca| composite_alphas.contains(ca))
                .ok_or_else(|| format_err!("No CompositeAlpha values specified!"))?
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
                        .ok_or_else(|| format_err!("Preferred format list was empty!"))?,
                },
        };

        // Find the window size
        let extent = {
            let window_client_area = window.get_inner_size().ok_or_else(|| format_err!("Window doesn't exist!"))?;
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
            Err(format_err!("The Surface isn't capable of supporting color!"))?
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
        let (swapchain, backbuffer) = unsafe {
            device.create_swapchain(surface, swapchain_config, None)?
        };

        Ok((swapchain, backbuffer, format, extent, image_count as _))
    }

    fn create_render_pass(device: &backend::Device, format: Format) -> Result<<backend::Backend as Backend>::RenderPass, Error> {
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
        Ok(unsafe {
            device
                .create_render_pass(&[color_attachment, depth_attachment],
                                    &[subpass],
                                    &[in_dependency, out_dependency])?
        })
    }
}

impl Drop for SwapchainBundle {
    fn drop(&mut self) {
        info!("{}", "Dropping Swapchain".red());
        let _ = self.device.wait_idle();
        unsafe {
            for depth_image in self.depth_images.drain(..) {
                drop(depth_image)
            }
            for fence in self.in_flight_fences.drain(..) {
                self.device.destroy_fence(fence)
            }
            for semaphore in self.render_finished_semaphores.drain(..) {
                self.device.destroy_semaphore(semaphore)
            }
            for semaphore in self.image_available_semaphores.drain(..) {
                self.device.destroy_semaphore(semaphore)
            }
            for framebuffer in self.framebuffers.drain(..) {
                self.device.destroy_framebuffer(framebuffer);
            }
            for image_view in self.image_views.drain(..) {
                self.device.destroy_image_view(image_view);
            }
            // LAST RESORT STYLE CODE, NOT TO BE IMITATED LIGHTLY
            use core::ptr::read;
            self
                .device
                .destroy_render_pass(ManuallyDrop::into_inner(Arc::try_unwrap(read(&self.render_pass)).unwrap()));
            self
                .device
                .destroy_swapchain(ManuallyDrop::into_inner(read(&self.swapchain)));
        }
    }
}