use crate::internal::graphics::buffer_bundle::CPU;
use crate::internal::graphics::BufferBundle;
use crate::internal::graphics::GraphicsState;
use colored::*;
use failure::Error;
use futures::Future;
use gfx_hal::adapter::PhysicalDevice;
use gfx_hal::buffer::Usage as BufferUsage;
use gfx_hal::format::Aspects;
use gfx_hal::image::Anisotropic;
use gfx_hal::image::Filter;
use gfx_hal::image::Layout;
use gfx_hal::image::Lod;
use gfx_hal::image::PackedColor;
use gfx_hal::image::SubresourceRange;
use gfx_hal::image::WrapMode;
use gfx_hal::memory::Properties;
use gfx_hal::memory::Requirements;
use gfx_hal::pool::CommandPoolCreateFlags;
use gfx_hal::pso::PipelineStage;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use gfx_hal::MemoryTypeId;
use image::RgbaImage;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use crate::allocator::GpuAllocator;
use crate::allocator::Memory;
use gfx_hal::format::AsFormat;
use gfx_hal::image::ViewCapabilities;
use std::marker::PhantomData;
use futures::future::IntoFuture;
use image::ImageBuffer;
use image::Pixel;
use std::ops::Deref;
use gfx_hal::format::Rgba8Srgb;


pub trait TextureType {
    fn view_capabilities() -> ViewCapabilities;
}

pub struct Single;
pub struct Array;
pub struct Cube;

impl TextureType for Single {
    #[inline]
    fn view_capabilities() -> ViewCapabilities {
        ViewCapabilities::empty()
    }
}

impl TextureType for Array {
    #[inline]
    fn view_capabilities() -> ViewCapabilities {
        ViewCapabilities::KIND_2D_ARRAY
    }
}

impl TextureType for Cube {
    #[inline]
    fn view_capabilities() -> ViewCapabilities {
        ViewCapabilities::KIND_CUBE
    }
}

pub struct TextureBundle<F: AsFormat + Send, TA: TextureType, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    image: ManuallyDrop<B::Image>,
    requirements: Requirements,
    memory: Memory<B>,
    image_view: ManuallyDrop<B::ImageView>,
    sampler: ManuallyDrop<B::Sampler>,
    state: Arc<GraphicsState<A, B, D, I>>,
    width: u32,
    height: u32,
    row_pitch: u32,
    pixel_size: u32,
    phantom_format: PhantomData<F>,
    phantom_type: PhantomData<TA>,
}

impl<F: AsFormat + Send, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> TextureBundle<F, Single, A, B, D, I> {
    pub fn new(state: Arc<GraphicsState<A, B, D, I>>, mip_map_levels: u8, width: u32, height: u32) -> impl Future<Item = Self, Error = Error> + Send {
        Self::create_image(state, mip_map_levels, 1, width, height).into_future()
    }

    // TODO: We should perhaps not do this if the image is mip mapped
    pub fn write_subset<'a>(&'a self, subset: (u32, u32, u32, u32), data: &[u8]) -> impl Future<Item = &'a Self, Error = Error> + 'a {
        self.do_write_data_from_bytes(subset, data)
    }
}


impl<F: AsFormat + Send, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> TextureBundle<F, Array, A, B, D, I> {
    pub fn new(state: Arc<GraphicsState<A, B, D, I>>, layers: u16, width: u32, height: u32) -> impl Future<Item = Self, Error = Error> + Send {
        Self::create_image(state, 1, layers, width, height).into_future()
    }
}

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> TextureBundle<Rgba8Srgb, Single, A, B, D, I> {
    pub fn write_data_from_image(self, image: RgbaImage) -> impl Future<Item = Self, Error = Error> {
        self.do_write_data_from_image(image)
    }
}


impl<F: AsFormat + Send, TA: TextureType, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> TextureBundle<F, TA, A, B, D, I> {
    fn create_image(state: Arc<GraphicsState<A, B, D, I>>, mip_map_levels: u8, layers: u16, width: u32, height: u32) -> Result<Self, Error> {
        debug_assert!(layers != 0, "Num layers can not be zero");

        // TODO: Assert on the number of mip maps
        debug_assert!(mip_map_levels != 0, "Num mip maps can not be zero");

        unsafe {
            let limits = *state.limits();
            let pixel_size = u32::from(F::SELF.surface_desc().bits) / 8;
            let row_size = pixel_size * width;
            let row_alignment_mask = limits.min_buffer_copy_pitch_alignment as u32 - 1;
            let row_pitch = (row_size as u32 + row_alignment_mask) & !row_alignment_mask;
            debug_assert!(row_pitch >= row_size);

            info!(
                "{} {}",
                "Allocating new texture with dimensions: ".green(),
                format!("{:?}x{:?}", width, height).yellow()
            );

            let mut image = state.device().create_image(
                gfx_hal::image::Kind::D2(width, height, layers, 1),
                mip_map_levels,
                F::SELF,
                gfx_hal::image::Tiling::Optimal,
                gfx_hal::image::Usage::TRANSFER_DST | gfx_hal::image::Usage::SAMPLED,
                TA::view_capabilities(),
            )?;

            let requirements = state.device().get_image_requirements(&image);
            let memory_type_id = state
                .adapter()
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
                .ok_or_else(|| format_err!("No queue group found"))?;

            let mut memory = state.allocator().allocate_memory(memory_type_id, requirements.size)?;
            memory.bind_image_memory(&state.device(), 0, &mut image)?;

            // TODO: Map to better errors
            let image_view = state.device().create_image_view(
                &image,
                gfx_hal::image::ViewKind::D2,
                F::SELF,
                gfx_hal::format::Swizzle::NO,
                SubresourceRange {
                    aspects: Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                },
            )?;

            let sampler = state.device().create_sampler(gfx_hal::image::SamplerInfo {
                min_filter: Filter::Nearest,
                mag_filter: Filter::Nearest,
                mip_filter: Filter::Nearest,
                wrap_mode: (WrapMode::Tile, WrapMode::Tile, WrapMode::Tile),
                lod_bias: Lod::from(0.0),
                lod_range: Lod::from(-1000.0)..Lod::from(1000.0),
                comparison: None,
                border: PackedColor(0),
                anisotropic: Anisotropic::On(16),
            })?;

            Ok(Self {
                image: ManuallyDrop::new(image),
                requirements,
                memory,
                image_view: ManuallyDrop::new(image_view),
                sampler: ManuallyDrop::new(sampler),
                state,
                width,
                height,
                row_pitch,
                pixel_size,
                phantom_format: PhantomData,
                phantom_type: PhantomData
            })
        }
    }

    fn do_write_data_from_bytes<'a>(&'a self, subset: (u32, u32, u32, u32), data: &[u8]) -> impl Future<Item = &'a Self, Error = Error> + 'a {
        let arc_data = Arc::new(Vec::from(data));
        debug_assert!((subset.2 * subset.3) * (u32::from(F::SELF.surface_desc().bits) / 8) == arc_data.len() as u32, "Data must contain enough bytes for the subset");

        BufferBundle::<A, B, D, I, CPU, u8>::new(
            Arc::clone(&self.state),
            data.len() as _,
            BufferUsage::TRANSFER_SRC,
        ).and_then(| bundle| {
            bundle.write_data(arc_data)
        }).and_then(move |bundle| {
            self.write_data_into_texture(subset, bundle)?;
            Ok(self)
        })

    }

    fn do_write_data_from_image<P, C>(self, image: ImageBuffer<P, C>) -> impl Future<Item = Self, Error = Error> where
        P: Pixel + 'static + Send + Sync,
        P::Subpixel: 'static,
        C: Deref<Target = [P::Subpixel]> + Send {

        debug_assert!(image.width() == self.width, "Image must be of same width as bundle");
        debug_assert!(image.height() == self.height, "Image must be of same height as bundle");


        let required_bytes = u64::from(self.row_pitch * self.height);
        let row_size = self.pixel_size * self.width;
        let row_pitch = self.row_pitch;

        BufferBundle::<A, B, D, I, CPU, P>::new(
            Arc::clone(&self.state),
            required_bytes,
            BufferUsage::TRANSFER_SRC,
        )
        .and_then(move |bundle: BufferBundle<A, B, D, I, CPU, P>| {
            bundle.write_image_data(image, row_size, row_pitch)
        })
        .and_then(move |bundle| {
            self.write_data_into_texture((0, 0, self.width, self.height), bundle)?;
            Ok(self)
        })
    }

    fn write_data_into_texture<T: Copy + Send + Sync>(&self, subset: (u32, u32, u32, u32), mut bundle: BufferBundle<A, B, D, I, CPU, T>) -> Result<(), Error> {
        unsafe {
            let mut pool = self.state.device().create_command_pool_typed(
                &bundle.queue_group,
                CommandPoolCreateFlags::TRANSIENT,
            )?;

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

            trace!("Copying bundle to texture");
            cmd_buffer.copy_buffer_to_image(
                &bundle.buffer,
                &*self.image,
                Layout::TransferDstOptimal,
                &[gfx_hal::command::BufferImageCopy {
                    buffer_offset: 0,
                    buffer_width: subset.2,
                    buffer_height: subset.3,
                    image_layers: gfx_hal::image::SubresourceLayers {
                        aspects: Aspects::COLOR,
                        level: 0,
                        layers: 0..1,
                    },
                    image_offset: gfx_hal::image::Offset { x: subset.0 as _, y: subset.1 as _, z: 0 },
                    image_extent: gfx_hal::image::Extent {
                        width: subset.2,
                        height: subset.3,
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
            let upload_fence = self.state.device().create_fence(false)?;
            let queue = &mut bundle.queue_group.queues[0];
            queue.submit_nosemaphores(Some(&cmd_buffer), Some(&upload_fence));
            self.state
                .device()
                .wait_for_fence(&upload_fence, core::u64::MAX)?;
            self.state.device().destroy_fence(upload_fence);

            // 11. Destroy the staging bundle and one shot buffer now that we're done
            pool.free(Some(cmd_buffer));

            Ok(())
        }
    }

    pub fn sampler(&self) -> &B::Sampler {
        &self.sampler
    }

    pub fn image_view(&self) -> &B::ImageView {
        &self.image_view
    }
}

impl<F: AsFormat + Send, TA: TextureType, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Drop for TextureBundle<F, TA, A, B, D, I> {
    fn drop(&mut self) {
        use core::ptr::read;

        info!("{}", "Dropping texture".red());

        let device = self.state.device();

        self.state.allocator().free_memory(&mut self.memory);

        unsafe {
            device.destroy_sampler(ManuallyDrop::into_inner(read(&self.sampler)));
            device.destroy_image_view(ManuallyDrop::into_inner(read(&self.image_view)));
            device.destroy_image(ManuallyDrop::into_inner(read(&self.image)));
        }
    }
}

impl<F: AsFormat + Send, TA: TextureType, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug for TextureBundle<F, TA, A, B, D, I> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.requirements)?;
        write!(f, "{:?}", self.width)?;
        write!(f, "{:?}", self.height)?;
        write!(f, "{}", self.state)?;
        Ok(())
    }
}
