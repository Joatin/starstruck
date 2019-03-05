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
use image::load_from_memory;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;

pub struct Texture<
    B: Backend = backend::Backend,
    D: Device<B> = backend::Device,
    I: Instance<Backend = B> = backend::Instance,
> {
    texture: TextureBundle<B, D, I>,
}

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>> Texture<B, D, I> {
    pub fn new(
        state: Arc<GraphicsState<B, D, I>>,
        data: &'static [u8],
    ) -> impl Future<Item = Self, Error = Error> + Send {
        lazy(move || {
            let image = load_from_memory(data)?;
            let rgb_image = image.to_rgba();

            Ok((state, rgb_image))
        })
        .and_then(|(st, rgb_image)| TextureBundle::new(st, rgb_image))
        .map(|texture| Self { texture })
    }

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

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug for Texture<B, D, I> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.texture)?;
        Ok(())
    }
}
