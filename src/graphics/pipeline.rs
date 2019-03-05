use crate::graphics::ShaderSet;
use crate::graphics::Texture;
use crate::internal::graphics::GraphicsState;
use crate::internal::graphics::PipelineBundle;
use crate::internal::graphics::PipelineLayoutBundle;
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

pub struct Pipeline<
    V: Vertex,
    B: Backend = backend::Backend,
    D: Device<B> = backend::Device,
    I: Instance<Backend = B> = backend::Instance,
> {
    bundle: RwLock<Option<PipelineBundle<V, B, D, I>>>,
    shader_set: ShaderSet,
}

impl<V: Vertex, B: Backend, D: Device<B>, I: Instance<Backend = B>> Pipeline<V, B, D, I> {
    pub fn new(
        state: Arc<GraphicsState<B, D, I>>,
        shader_set: ShaderSet,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        let shader_set_clone = shader_set.clone();
        lazy(move || {
            let render_area = state.render_area();
            let coned_state = Arc::clone(&state);

            state.render_pass(move |render_pass| {
                PipelineBundle::<V, B, D, I>::new(
                    coned_state,
                    render_pass,
                    render_area,
                    &shader_set_clone,
                )
            })
        })
        .map(move |bundle| Self {
            bundle: RwLock::new(Some(bundle)),
            shader_set,
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

    pub fn bind_texture(&self, texture: &Texture<B, D, I>) {
        info!("Binding texture to pipeline");
        let lock = self.bundle.read().unwrap();
        if let Some(pipeline) = lock.as_ref() {
            let descriptors = texture.get_descriptors();
            pipeline.bind_assets(descriptors);
        }
    }
}

pub trait RecreatePipeline<B: Backend, D: Device<B>, I: Instance<Backend = B>>:
    Sync + Send
{
    fn drop_pipeline(&self);
    fn recreate_pipeline(&self, state: Arc<GraphicsState<B, D, I>>) -> Result<(), Error>;
}

impl<B: Backend, D: Device<B>, I: Instance<Backend = B>, V: Vertex> RecreatePipeline<B, D, I>
    for Pipeline<V, B, D, I>
{
    fn drop_pipeline(&self) {
        let mut bundle = self.bundle.write().unwrap();
        bundle.take();
    }

    fn recreate_pipeline(&self, state: Arc<GraphicsState<B, D, I>>) -> Result<(), Error> {
        let render_area = state.render_area();
        let cloned_state = Arc::clone(&state);

        state.render_pass(move |render_pass| {
            let mut bundle_lock = self.bundle.write().unwrap();
            let bundle =
                PipelineBundle::new(cloned_state, render_pass, render_area, &self.shader_set)?;
            *bundle_lock = Some(bundle);
            Ok(())
        })?;
        Ok(())
    }
}

pub trait PipelineEncoderExt<V: Vertex, B: Backend, D: Device<B>, I: Instance<Backend = B>> {
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V, B, D, I>);
    unsafe fn bind_push_constant(
        &mut self,
        pipeline: &Pipeline<V, B, D, I>,
        flags: ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    );
}

impl<'a, V: Vertex, B: Backend, D: Device<B>, I: Instance<Backend = B>>
    PipelineEncoderExt<V, B, D, I> for RenderPassInlineEncoder<'a, B>
{
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V, B, D, I>) {
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
        pipeline: &Pipeline<V, B, D, I>,
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

impl<V: Vertex, B: Backend, D: Device<B>, I: Instance<Backend = B>> Debug for Pipeline<V, B, D, I> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{:?}", self.bundle)?;
        Ok(())
    }
}
