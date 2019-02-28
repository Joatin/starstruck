use crate::graphics::ShaderSet;
use crate::internal::graphics::GraphicsState;
use crate::internal::graphics::PipelineBundle;
use crate::primitive::Vertex;
use failure::Error;
use futures::lazy;
use futures::Future;
use gfx_hal::command::RenderPassInlineEncoder;
use gfx_hal::pso::ShaderStageFlags;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::RwLock;

pub struct Pipeline<V: Vertex> {
    bundle: RwLock<Option<PipelineBundle>>,
    phantom_vertex: PhantomData<fn() -> V>,
    shader_set: ShaderSet,
}

impl<V: Vertex> Pipeline<V> {
    pub fn new<'a>(
        state: Arc<GraphicsState>,
        shader_set: ShaderSet,
    ) -> impl Future<Item = Self, Error = Error> + 'a + Send {
        let shader_set_clone = shader_set.clone();
        lazy(move || {
            let render_area = state.render_area();
            let device = state.device();

            state.render_pass(move |render_pass| {
                PipelineBundle::new::<V>(device, render_pass, render_area, &shader_set_clone)
            })
        })
        .map(move |bundle| Self {
            bundle: RwLock::new(Some(bundle)),
            phantom_vertex: PhantomData,
            shader_set,
        })
    }
}

pub trait RecreatePipeline: Sync + Send {
    fn drop_pipeline(&self);
    fn recreate_pipeline(&self, state: &GraphicsState) -> Result<(), Error>;
}

impl<V: Vertex> RecreatePipeline for Pipeline<V> {
    fn drop_pipeline(&self) {
        let mut bundle = self.bundle.write().unwrap();
        bundle.take();
    }

    fn recreate_pipeline(&self, state: &GraphicsState) -> Result<(), Error> {
        let render_area = state.render_area();
        let device = state.device();

        state.render_pass(move |render_pass| {
            let mut bundle_lock = self.bundle.write().unwrap();
            let bundle =
                PipelineBundle::new::<V>(device, render_pass, render_area, &self.shader_set)?;
            *bundle_lock = Some(bundle);
            Ok(())
        })?;
        Ok(())
    }
}

pub trait PipelineEncoderExt<V: Vertex> {
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V>);
    unsafe fn bind_push_constant(
        &mut self,
        pipeline: &Pipeline<V>,
        flags: ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    );
}

impl<'a, V: Vertex> PipelineEncoderExt<V> for RenderPassInlineEncoder<'a, backend::Backend> {
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V>) {
        let bundle = pipeline.bundle.read().unwrap();
        unsafe { self.bind_graphics_pipeline(bundle.as_ref().unwrap().pipeline()) }
    }
    unsafe fn bind_push_constant(
        &mut self,
        pipeline: &Pipeline<V>,
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
