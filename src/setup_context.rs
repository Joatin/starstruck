use crate::graphics::Bundle;
use crate::graphics::Pipeline;
use crate::graphics::ShaderSet;
use crate::graphics::Texture;
use crate::internal::graphics::GraphicsState;
use crate::primitive::Index;
use crate::primitive::Vertex;
use failure::Error;
use futures::Future;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use std::sync::Arc;
use crate::allocator::GpuAllocator;
use crate::allocator::DefaultGpuAllocator;
use crate::allocator::DefaultChunk;
use image::DynamicImage;
use crate::graphics::AsFormat;
use image::load_from_memory;
use crate::graphics::Single;
use crate::graphics::Rgba8Srgb;
use futures::lazy;

#[allow(clippy::type_complexity)]
pub struct SetupContext<
    A: GpuAllocator<B, D> = DefaultGpuAllocator<DefaultChunk<backend::Backend, backend::Device>, backend::Backend, backend::Device>,
    B: Backend = backend::Backend,
    D: Device<B> = backend::Device,
    I: Instance<Backend = B> = backend::Instance,
> {
    state: Arc<GraphicsState<A, B, D, I>>
}

impl<A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> SetupContext<A, B, D, I> {
    pub(crate) fn new(state: Arc<GraphicsState<A, B, D, I>>) -> Self {
        Self {
            state
        }
    }

    pub fn create_bundle<In: Index, V: Vertex>(
        &self,
        indexes: &'static [In],
        vertexes: &'static [V],
    ) -> impl Future<Item = Bundle<In, V, A, B, D, I>, Error = Error> + Send {
        Bundle::new(
            Arc::clone(&self.state),
            Arc::new(Vec::from(indexes)),
            Arc::new(Vec::from(vertexes)),
        )
    }

    pub(crate) fn create_bundle_owned<In: Index, V: Vertex>(
        &self,
        indexes: Arc<Vec<In>>,
        vertexes: Arc<Vec<V>>,
    ) -> impl Future<Item = Bundle<In, V, A, B, D, I>, Error = Error> + Send {
        Bundle::new(Arc::clone(&self.state), indexes, vertexes)
    }

    pub fn create_pipeline<V: 'static + Vertex>(
        &self,
        shader_set: ShaderSet,
    ) -> impl Future<Item = Pipeline<V, A, B, D, I>, Error = Error> + Send {
        Pipeline::new(Arc::clone(&self.state), shader_set)
    }

    pub fn create_texture_from_bytes(
        &self,
        image_data: &'static [u8],
    ) -> impl Future<Item = Texture<Rgba8Srgb, Single, A, B, D, I>, Error = Error> + Send {
        let cloned_state = Arc::clone(&self.state);
        lazy(move || {
            Ok(load_from_memory(image_data)?)
        }).and_then(move |image| {
            Texture::<Rgba8Srgb, Single, A, B, D, I>::new(cloned_state, image)
        })
    }

    pub fn create_texture_from_image(&self, image: DynamicImage) -> impl Future<Item = Texture<Rgba8Srgb, Single, A, B, D, I>, Error = Error> + Send {
        Texture::<Rgba8Srgb, Single, A, B, D, I>::new(Arc::clone(&self.state), image)
    }

    pub fn create_texture_sized<F: AsFormat + Send>(&self, width: u32, height: u32) -> impl Future<Item = Texture<F, Single, A, B, D, I>, Error = Error> + Send {
        Texture::<F, Single, A, B, D, I>::sized(Arc::clone(&self.state), 1, width, height)
    }

    pub fn logical_window_size(&self) -> (u32, u32) {
        self.state.logical_window_size()
    }

    pub fn dpi(&self) -> f64 {
        self.state.dpi()
    }
}

pub trait CreateDefaultPipeline<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    #[allow(clippy::type_complexity)]
    fn create_default_pipeline(
        &self,
    ) -> Box<Future<Item = Pipeline<V, A, B, D, I>, Error = Error> + Send>;
}

pub trait CreateTexturedPipeline<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    #[allow(clippy::type_complexity)]
    fn create_textured_pipeline(
        &self,
    ) -> Box<Future<Item = Pipeline<V, A, B, D, I>, Error = Error> + Send>;
}

pub trait CreateBundleFromObj<
    In: Index,
    V: Vertex,
    A: GpuAllocator<B, D>,
    B: Backend,
    D: Device<B>,
    I: Instance<Backend = B>,
>
{
    #[allow(clippy::type_complexity)]
    fn create_bundle_from_obj(
        &self,
        data: &[u8],
    ) -> Box<Future<Item = Bundle<In, V, A, B, D, I>, Error = Error> + Send>;
}
