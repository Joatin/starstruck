use failure::Error;
use gfx_hal::Backend;
use gfx_hal::Device;
use futures::Future;
use crate::internal::graphics::TextureBundle;
use std::sync::Arc;
use crate::internal::graphics::GraphicsState;
use gfx_hal::Instance;
use image::load_from_memory;
use futures::future::lazy;

pub struct Texture<B: Backend, D: Device<B>, I: Instance<Backend=B>> {
    texture: TextureBundle<B, D, I>
}

impl<B: Backend, D: Device<B>, I: Instance<Backend=B>> Texture<B, D, I> {

    pub fn new(
        state: Arc<GraphicsState<B, D, I>>,
        data: &'static [u8]
    ) -> impl Future<Item=Self, Error=Error> + Send {
        lazy(move || {
            let image = load_from_memory(data)?;
            let rgb_image = image.to_rgba();

            Ok((state, rgb_image))
        }).and_then(|(st, rgb_image)| {
            TextureBundle::new(
                st,
                rgb_image
            )
        }).map(|texture| {
            Self {
                texture
            }
        })
    }
}