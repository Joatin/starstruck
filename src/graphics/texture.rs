use crate::internal::graphics::GraphicsState;
use crate::internal::graphics::TextureBundle;
use failure::Error;
use futures::lazy;
use futures::Future;
use gfx_hal::image::Layout;
use gfx_hal::pso::Descriptor;
use gfx_hal::pso::DescriptorArrayIndex;
use gfx_hal::pso::DescriptorBinding;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;
use crate::allocator::GpuAllocator;
use crate::allocator::DefaultGpuAllocator;
use crate::allocator::DefaultChunk;
use crate::internal::graphics::Single;
use gfx_hal::format::Rgba8Srgb;
use gfx_hal::format::AsFormat;
use image::DynamicImage;
use crate::internal::graphics::TextureType;

macro_rules! implement_format {
    ( $x:ty, $y:expr ) => {
        impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Texture<$x, Single, A, B, D, I> {
            pub fn new(
                state: Arc<GraphicsState<A, B, D, I>>,
                image: DynamicImage,
            ) -> impl Future<Item = Self, Error = Error> + Send {
                lazy(move || {
                    let the_image = $y(&image);

                    Ok((state, the_image))
                })
                    .map(|(st, the_image)| (TextureBundle::<$x, Single, A, B, D, I>::new(st, 1, the_image.width(), the_image.height()), the_image))
                    .and_then(|(texture, the_image)| texture.and_then(|tex| tex.write_data_from_image(the_image)))
                    .map(|texture| Self { texture })
            }
        }
    };
}

pub struct Texture<
    F: AsFormat + Send = Rgba8Srgb,
    TA: TextureType = Single,
    A: GpuAllocator<B, D> = DefaultGpuAllocator<DefaultChunk<backend::Backend, backend::Device>, backend::Backend, backend::Device>,
    B: Backend = backend::Backend,
    D: Device<B> = backend::Device,
    I: Instance<Backend = B> = backend::Instance,
> {
    texture: TextureBundle<F, TA, A, B, D, I>,
}

implement_format!(Rgba8Srgb, DynamicImage::to_rgba);

impl<F: AsFormat + Send, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Texture<F, Single, A, B, D, I> {
    pub fn sized(state: Arc<GraphicsState<A, B, D, I>>, mip_map_levels: u8, width: u32, height: u32) -> impl Future<Item = Self, Error = Error> + Send {
        TextureBundle::<F, Single, A, B, D, I>::new(state, mip_map_levels, width, height).map(|texture| {
            Self {
                texture
            }
        })
    }

    pub fn write_subset<'a>(&'a self, subset: (u32, u32, u32, u32), data: &[u8]) -> impl Future<Item = &'a Self, Error = Error> + 'a {
        let texture = &self.texture;
        texture.write_subset(subset, data).map(move |_| self)
    }
}

impl<F: AsFormat + Send, TA: TextureType, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Texture<F, TA, A, B, D, I> {

    pub(crate) fn get_descriptors(
        &self,
    ) -> Vec<(DescriptorBinding, DescriptorArrayIndex, Descriptor<B>)> {
        vec![
            (
                0,
                0,
                Descriptor::Image(self.texture.image_view(), Layout::Undefined),
            ),
            (1, 0, Descriptor::Sampler(self.texture.sampler())),
        ]
    }
}

impl<F: AsFormat + Send, TA: TextureType, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug for Texture<F, TA, A, B, D, I> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        if f.alternate() {
            write!(f, "Texture {:#?}", self.texture)?;
        } else {
            write!(f, "Texture {:?}", self.texture)?;
        }
        Ok(())
    }
}
