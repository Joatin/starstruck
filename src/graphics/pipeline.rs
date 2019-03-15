use crate::graphics::ShaderSet;
use crate::graphics::Texture;
use crate::internal::graphics::GraphicsState;
use crate::internal::graphics::PipelineBundle;
use crate::primitive::Vertex;
use failure::Error;
use futures::lazy;
use futures::Future;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::pso::ShaderStageFlags;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;
use std::sync::RwLock;
use crate::allocator::GpuAllocator;
use crate::allocator::DefaultGpuAllocator;
use crate::allocator::DefaultChunk;
use crate::graphics::AsFormat;
use crate::graphics::TextureType;


#[allow(clippy::type_complexity)]
pub struct Pipeline<
    V: Vertex,
    A: GpuAllocator<B, D> = DefaultGpuAllocator<DefaultChunk<backend::Backend, backend::Device>, backend::Backend, backend::Device>,
    B: Backend = backend::Backend,
    D: Device<B> = backend::Device,
    I: Instance<Backend = B> = backend::Instance,
> {
    bundle: RwLock<Option<PipelineBundle<V, A, B, D, I>>>
}

impl<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Pipeline<V, A, B, D, I> {
    pub fn new(
        state: Arc<GraphicsState<A, B, D, I>>,
        shader_set: ShaderSet,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        lazy(move || {
            let coned_state = Arc::clone(&state);

            state.render_pass(move |render_pass| {
                PipelineBundle::<V, A, B, D, I>::new(
                    coned_state,
                    render_pass,
                    &shader_set,
                )
            })
        })
        .map(move |bundle| Self {
            bundle: RwLock::new(Some(bundle))
        })
    }

    pub(crate) fn layout_and_set<T: FnOnce(&B::PipelineLayout, &B::DescriptorSet) -> ()>(
        &self,
        callback: T,
    ) {
        let lock = self.bundle.read().unwrap();
        if let Some(pipeline) = lock.as_ref() {
            let layout = pipeline.layout();
            let set = pipeline.descriptor_set();
            callback(layout, set);
        } else {
            error!("Could not get pipeline! This should not be possible!");
        }
    }

    pub fn bind_texture<F: AsFormat + Send, TA: TextureType>(&self, texture: &Texture<F, TA, A, B, D, I>) {
        let lock = self.bundle.read().unwrap();
        if let Some(pipeline) = lock.as_ref() {
            let descriptors = texture.get_descriptors();
            pipeline.bind_assets(descriptors);
        }
    }
}

pub trait PipelineEncoderExt<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V, A, B, D, I>);
    unsafe fn bind_push_constant(
        &mut self,
        pipeline: &Pipeline<V, A, B, D, I>,
        flags: ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    );
}

impl<'a, V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>>
    PipelineEncoderExt<V, A, B, D, I> for RenderPassInlineEncoder<'a, B>
{
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V, A, B, D, I>) {
        {
            let bundle = pipeline.bundle.read().unwrap();
            unsafe { self.bind_graphics_pipeline(bundle.as_ref().unwrap().pipeline()) }
        }
        pipeline.layout_and_set(|layout, set| unsafe {
            self.bind_graphics_descriptor_sets(layout, 0, Some(set), &[]);
        });
    }
    unsafe fn bind_push_constant(
        &mut self,
        pipeline: &Pipeline<V, A, B, D, I>,
        flags: ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    ) {
        let pipe = pipeline.bundle.read().unwrap();
        let bundle = pipe
            .as_ref()
            .expect("Bundle can only be None during swapchain recreation");
        self.push_graphics_constants(bundle.layout(), flags, offset, constants);
    }
}

impl<V: Vertex, A: GpuAllocator<B, D>, B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug for Pipeline<V, A, B, D, I> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.bundle)?;
        Ok(())
    }
}
