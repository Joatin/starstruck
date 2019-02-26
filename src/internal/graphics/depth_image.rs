use std::mem::ManuallyDrop;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::memory::Requirements;
use std::marker::PhantomData;
use gfx_hal::image::SubresourceRange;
use gfx_hal::MemoryTypeId;
use gfx_hal::Adapter;
use gfx_hal::format::Format;
use gfx_hal::PhysicalDevice;
use gfx_hal::format::Aspects;
use gfx_hal::memory::Properties;
use gfx_hal::window::Extent2D;
use failure::Error;
use std::sync::Arc;

pub struct DepthImage {
    pub image: ManuallyDrop<<backend::Backend as Backend>::Image>,
    pub requirements: Requirements,
    pub memory: ManuallyDrop<<backend::Backend as Backend>::Memory>,
    pub image_view: ManuallyDrop<<backend::Backend as Backend>::ImageView>,
    pub device: Arc<backend::Device>,
    pub phantom: PhantomData<backend::Device>,
}


impl DepthImage {
    pub fn new(device: Arc<backend::Device>, adapter: &Adapter<backend::Backend>, extent: Extent2D) -> Result<Self, Error> {
        unsafe {
            let mut the_image = device
                .create_image(
                    gfx_hal::image::Kind::D2(extent.width, extent.height, 1, 1),
                    1,
                    Format::D32Float,
                    gfx_hal::image::Tiling::Optimal,
                    gfx_hal::image::Usage::DEPTH_STENCIL_ATTACHMENT,
                    gfx_hal::image::ViewCapabilities::empty(),
                )?;
            let requirements = device.get_image_requirements(&the_image);
            let memory_type_id = adapter
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
                .ok_or_else(|| format_err!("Couldn't find a memory type to support the image!"))?;
            let memory = device.allocate_memory(memory_type_id, requirements.size)?;
            device.bind_image_memory(&memory, 0, &mut the_image)?;
            let image_view = device
                .create_image_view(
                    &the_image,
                    gfx_hal::image::ViewKind::D2,
                    Format::D32Float,
                    gfx_hal::format::Swizzle::NO,
                    SubresourceRange {
                        aspects: Aspects::DEPTH,
                        levels: 0..1,
                        layers: 0..1,
                    },
                )?;
            Ok(Self {
                image: ManuallyDrop::new(the_image),
                requirements,
                memory: ManuallyDrop::new(memory),
                image_view: ManuallyDrop::new(image_view),
                device,
                phantom: PhantomData,
            })
        }
    }
}

impl Drop for DepthImage {
    fn drop(&mut self) {
        use core::ptr::read;

        let device = &self.device;

        unsafe {
            device.destroy_image_view(ManuallyDrop::into_inner(read(&self.image_view)));
            device.destroy_image(ManuallyDrop::into_inner(read(&self.image)));
            device.free_memory(ManuallyDrop::into_inner(read(&self.memory)));
        }
    }
}