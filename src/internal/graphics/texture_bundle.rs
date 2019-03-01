use failure::Error;
use gfx_hal::Backend;
use gfx_hal::Device;
use std::mem::ManuallyDrop;
use gfx_hal::memory::Requirements;
use std::sync::Arc;
use image::RgbaImage;
use std::mem::size_of;
use crate::internal::graphics::GraphicsState;
use crate::internal::graphics::BufferBundle;
use gfx_hal::buffer::Usage as BufferUsage;
use futures::Future;
use gfx_hal::Instance;
use crate::internal::graphics::buffer_bundle::CPU;
use gfx_hal::format::Format;
use gfx_hal::format::Aspects;
use gfx_hal::MemoryTypeId;
use gfx_hal::image::SubresourceRange;
use gfx_hal::adapter::PhysicalDevice;
use gfx_hal::memory::Properties;
use futures::lazy;
use gfx_hal::image::Layout;
use gfx_hal::pso::PipelineStage;
use gfx_hal::pool::CommandPoolCreateFlags;


pub struct TextureBundle<B: Backend, D: Device<B>, I: Instance<Backend=B>> {
    image: ManuallyDrop<B::Image>,
    requirements: Requirements,
    memory: ManuallyDrop<B::Memory>,
    image_view: ManuallyDrop<B::ImageView>,
    sampler: ManuallyDrop<B::Sampler>,
    state: Arc<GraphicsState<B, D, I>>,
    width: u32,
    height: u32,
    row_pitch: usize,
    pixel_size: usize
}

impl<B: Backend, D: Device<B>, I: Instance<Backend=B>> TextureBundle<B, D, I> {
    pub fn new(
        state: Arc<GraphicsState<B, D, I>>,
        image: RgbaImage
    ) -> impl Future<Item=Self, Error=Error> + Send {

        let limits = *state.limits();
        let pixel_size = size_of::<image::Rgba<u8>>();
        let row_size = pixel_size * (image.width() as usize);
        let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
        let row_pitch = ((row_size as u32 + row_alignment_mask) & !row_alignment_mask) as usize;
        debug_assert!(row_pitch as usize >= row_size);
        let required_bytes = (row_pitch * image.height() as usize) as _;

        let width = image.width();
        let height = image.height();


        BufferBundle::<B, D, I, CPU, image::Rgba<u8>>::new(
            Arc::clone(&state),
            required_bytes,
            BufferUsage::TRANSFER_SRC
        )
            .and_then(move |bundle: BufferBundle<B, D, I, CPU, image::Rgba<u8>>| {
                bundle.write_image_data(image, limits)
            })
            .and_then(move |_bundle| {
                unsafe {
                    let mut the_image = state.device()
                        .create_image(
                            gfx_hal::image::Kind::D2(width, height, 1, 1),
                            1,
                            Format::Rgba8Srgb,
                            gfx_hal::image::Tiling::Optimal,
                            gfx_hal::image::Usage::TRANSFER_DST | gfx_hal::image::Usage::SAMPLED,
                            gfx_hal::image::ViewCapabilities::empty(),
                        )?;

                    let requirements = state.device().get_image_requirements(&the_image);
                    let memory_type_id = state.adapter()
                        .physical_device
                        .memory_properties()
                        .memory_types
                        .iter()
                        .enumerate()
                        .find(|&(id, memory_type)| {
                            // BIG NOTE: THIS IS DEVICE LOCAL NOT CPU VISIBLE
                            requirements.type_mask & (1 << id) != 0
                                && memory_type.properties.contains(Properties::DEVICE_LOCAL)
                        })
                        .map(|(id, _)| MemoryTypeId(id))
                        .ok_or(format_err!("No queue group found"))?;
                    let memory = state.device()
                        .allocate_memory(memory_type_id, requirements.size)?;
                    state.device()
                        .bind_image_memory(&memory, 0, &mut the_image)?;

                    let image_view = state.device()
                        .create_image_view(
                            &the_image,
                            gfx_hal::image::ViewKind::D2,
                            Format::Rgba8Srgb,
                            gfx_hal::format::Swizzle::NO,
                            SubresourceRange {
                                aspects: Aspects::COLOR,
                                levels: 0..1,
                                layers: 0..1,
                            },
                        )?;
                    let sampler = state.device()
                        .create_sampler(gfx_hal::image::SamplerInfo::new(
                            gfx_hal::image::Filter::Nearest,
                            gfx_hal::image::WrapMode::Tile,
                        ))?;

                    Ok(Self {
                        image: ManuallyDrop::new(the_image),
                        requirements,
                        memory: ManuallyDrop::new(memory),
                        image_view: ManuallyDrop::new(image_view),
                        sampler: ManuallyDrop::new(sampler),
                        state,
                        width,
                        height,
                        row_pitch,
                        pixel_size
                    })
                }
            })
    }


    fn import_data_from_bundle(
        self,
        mut bundle: BufferBundle<B, D, I, CPU, image::Rgba<u8>>
    ) -> impl Future<Item=Self, Error=Error> {
        lazy(move || {
            unsafe {
                let mut pool = self.state.device().create_command_pool_typed(&bundle.queue_group, CommandPoolCreateFlags::TRANSIENT)?;

                let mut cmd_buffer = pool.acquire_command_buffer::<gfx_hal::command::OneShot>();
                cmd_buffer.begin();

                let image_barrier = gfx_hal::memory::Barrier::Image {
                    states: (gfx_hal::image::Access::empty(), Layout::Undefined)
                        ..(
                        gfx_hal::image::Access::TRANSFER_WRITE,
                        Layout::TransferDstOptimal,
                    ),
                    target: &*self.image,
                    families: None,
                    range: SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    },
                };

                cmd_buffer.pipeline_barrier(
                    PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
                    gfx_hal::memory::Dependencies::empty(),
                    &[image_barrier],
                );


                cmd_buffer.copy_buffer_to_image(
                    &bundle.buffer,
                    &self.image,
                    Layout::TransferDstOptimal,
                    &[gfx_hal::command::BufferImageCopy {
                        buffer_offset: 0,
                        buffer_width: (self.row_pitch / self.pixel_size) as u32,
                        buffer_height: self.height,
                        image_layers: gfx_hal::image::SubresourceLayers {
                            aspects: Aspects::COLOR,
                            level: 0,
                            layers: 0..1,
                        },
                        image_offset: gfx_hal::image::Offset { x: 0, y: 0, z: 0 },
                        image_extent: gfx_hal::image::Extent {
                            width: self.width,
                            height: self.height,
                            depth: 1,
                        },
                    }],
                );

                let image_barrier = gfx_hal::memory::Barrier::Image {
                    states: (
                        gfx_hal::image::Access::TRANSFER_WRITE,
                        Layout::TransferDstOptimal,
                    )
                        ..(
                        gfx_hal::image::Access::SHADER_READ,
                        Layout::ShaderReadOnlyOptimal,
                    ),
                    target: &*self.image,
                    families: None,
                    range: SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    },
                };
                cmd_buffer.pipeline_barrier(
                    PipelineStage::TRANSFER..PipelineStage::FRAGMENT_SHADER,
                    gfx_hal::memory::Dependencies::empty(),
                    &[image_barrier],
                );

                cmd_buffer.finish();
                let upload_fence = self.state.device()
                    .create_fence(false)?;
                let queue = &mut bundle.queue_group.queues[0];
                queue.submit_nosemaphores(Some(&cmd_buffer), Some(&upload_fence));
                self.state.device()
                    .wait_for_fence(&upload_fence, core::u64::MAX)?;
                self.state.device().destroy_fence(upload_fence);

                // 11. Destroy the staging bundle and one shot buffer now that we're done
                pool.free(Some(cmd_buffer));

                Ok(self)
            }
        })
    }
}

impl<B: Backend, D: Device<B>, I: Instance<Backend=B>> Drop for TextureBundle<B, D, I> {
    fn drop(&mut self) {
        use core::ptr::read;
        let device = self.state.device();
        unsafe {
            device.destroy_sampler(ManuallyDrop::into_inner(read(&self.sampler)));
            device.destroy_image_view(ManuallyDrop::into_inner(read(&self.image_view)));
            device.destroy_image(ManuallyDrop::into_inner(read(&self.image)));
            device.free_memory(ManuallyDrop::into_inner(read(&self.memory)));
        }
    }
}