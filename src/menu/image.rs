use crate::menu::Component;
use futures::Future;
use failure::Error;
use crate::graphics::Bundle;
use crate::graphics::Texture;
use std::sync::Arc;
use crate::setup_context::SetupContext;
use crate::primitive::Vertex2DUV;
use crate::allocator::GpuAllocator;
use crate::allocator::DefaultGpuAllocator;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use crate::allocator::DefaultChunk;
use crate::context::Context;
use crate::graphics::Pipeline;
use crate::setup_context::CreateTexturedPipeline;
use crate::graphics::Rgba8Srgb;
use crate::graphics::Single;


pub struct Image<
    A: GpuAllocator<B, D> = DefaultGpuAllocator<DefaultChunk<backend::Backend, backend::Device>, backend::Backend, backend::Device>,
    B: Backend = backend::Backend,
    D: Device<B> = backend::Device,
    I: Instance<Backend = B> = backend::Instance,
> {
    bundle: Bundle<u16, Vertex2DUV, A, B, D, I>,
    _texture: Texture<Rgba8Srgb, Single, A, B, D, I>,
    pipeline: Pipeline<Vertex2DUV, A, B, D, I>
}

impl<
    A: GpuAllocator<B, D>,
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>,
> Image<A, B, D, I> {
    const INDEXES: [u16; 6] = [0, 1, 2, 3, 0, 1];


    pub fn new(setup: Arc<SetupContext<A, B, D, I>>, image_data: &'static [u8]) -> impl Future<Item=Self, Error=Error> {
        let (_window_width, _window_height) = setup.logical_window_size();

        const VERTICES: [Vertex2DUV; 4] = [
            Vertex2DUV { x: -1.0, y: 1.0, r: 0.0, g: 1.0 },
            Vertex2DUV { x: 1.0, y: -1.0, r: 1.0, g: 0.0 },
            Vertex2DUV { x: 1.0, y: 1.0, r: 1.0, g: 1.0 },
            Vertex2DUV { x: -1.0, y: -1.0, r: 0.0, g: 0.0 },
        ];

        let bundle_future = setup.create_bundle(&Self::INDEXES, &VERTICES);
        let texture_future = setup.create_texture_from_bytes(image_data);
        let pipeline_future = setup.create_textured_pipeline();

        bundle_future.join3(texture_future, pipeline_future).map(|(bundle, texture, pipeline)| {

            pipeline.bind_texture(&texture);

            Self {
                bundle,
                _texture: texture,
                pipeline
            }
        })
    }
}

impl<
    A: GpuAllocator<B, D>,
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>,
> Component<A, B, D, I> for Image<A, B, D, I> {
    fn resize(&mut self, _size: (u32, u32)) {
        unimplemented!()
    }

    fn draw(&self, context: &mut Context<A, B, D, I>) -> Result<(), Error> {
        context.draw(&self.pipeline, &self.bundle);
        Ok(())
    }
}
