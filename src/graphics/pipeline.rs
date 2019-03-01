use crate::graphics::ShaderSet;
use crate::internal::graphics::GraphicsState;
use crate::internal::graphics::PipelineBundle;
use crate::primitive::Vertex;
use failure::Error;
use futures::lazy;
use futures::Future;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::pso::ShaderStageFlags;
use std::sync::Arc;
use std::sync::RwLock;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;

pub struct Pipeline<V: Vertex, B: Backend = backend::Backend, D: Device<B> = backend::Device> {
    bundle: RwLock<Option<PipelineBundle<B, D, V>>>,
    shader_set: ShaderSet,
}

impl<V: Vertex, B: Backend, D: Device<B>> Pipeline<V, B, D> {
    pub fn new<I: Instance<Backend=B>>(
        state: Arc<GraphicsState<B, D, I>>,
        shader_set: ShaderSet,
    ) -> impl Future<Item = Self, Error = Error> + Send {
        let shader_set_clone = shader_set.clone();
        lazy(move || {
            let render_area = state.render_area();
            let device = state.device();

            state.render_pass(move |render_pass| {
                PipelineBundle::<B, D, V>::new(device, render_pass, render_area, &shader_set_clone)
            })
        })
        .map(move |bundle| Self {
            bundle: RwLock::new(Some(bundle)),
            shader_set,
        })
    }
}

pub trait RecreatePipeline<B: Backend, D: Device<B>, I: Instance<Backend=B>>: Sync + Send {
    fn drop_pipeline(&self);
    fn recreate_pipeline(&self, state: &GraphicsState<B, D, I>) -> Result<(), Error>;
}

impl<B: Backend, D: Device<B>, I: Instance<Backend=B>, V: Vertex> RecreatePipeline<B, D, I> for Pipeline<V, B, D> {
    fn drop_pipeline(&self) {
        let mut bundle = self.bundle.write().unwrap();
        bundle.take();
    }

    fn recreate_pipeline(&self, state: &GraphicsState<B, D, I>) -> Result<(), Error> {
        let render_area = state.render_area();
        let device = state.device();

        state.render_pass(move |render_pass| {
            let mut bundle_lock = self.bundle.write().unwrap();
            let bundle =
                PipelineBundle::new(device, render_pass, render_area, &self.shader_set)?;
            *bundle_lock = Some(bundle);
            Ok(())
        })?;
        Ok(())
    }
}

pub trait PipelineEncoderExt<V: Vertex, B: Backend, D: Device<B>> {
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V, B, D>);
    unsafe fn bind_push_constant(
        &mut self,
        pipeline: &Pipeline<V, B, D>,
        flags: ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    );
}

impl<'a, V: Vertex, B: Backend, D: Device<B>> PipelineEncoderExt<V, B, D> for RenderPassInlineEncoder<'a, B> {
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V, B, D>) {
        let bundle = pipeline.bundle.read().unwrap();
        unsafe { self.bind_graphics_pipeline(bundle.as_ref().unwrap().pipeline()) }
    }
    unsafe fn bind_push_constant(
        &mut self,
        pipeline: &Pipeline<V, B, D>,
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
