use crate::graphics::ShaderSet;
use crate::internal::graphics::PipelineBundle;
use failure::Error;
use gfx_hal::Backend;
use crate::primitive::Vertex;
use std::marker::PhantomData;
use futures::Future;
use futures::lazy;
use gfx_hal::window::Extent2D;
use std::sync::Arc;
use std::mem::ManuallyDrop;
use gfx_hal::command::RenderPassInlineEncoder;
use std::sync::RwLock;

pub struct Pipeline<V: Vertex> {
    bundle: RwLock<Option<PipelineBundle>>,
    phantom_vertex: PhantomData<fn() -> V>,
    shader_set: ShaderSet,
}

impl<V: Vertex> Pipeline<V> {

    pub fn new<'a>(device: Arc<backend::Device>, render_pass: Arc<ManuallyDrop<<backend::Backend as Backend>::RenderPass>>, render_area: Extent2D, shader_set: ShaderSet) -> impl Future<Item=Self, Error=Error> + 'a + Send {
        let shader_set_clone = shader_set.clone();
        lazy(move || {
            PipelineBundle::new::<V>(device, render_pass, render_area, &shader_set_clone)
        }).map(move |bundle| {
            Self {
                bundle: RwLock::new(Some(bundle)),
                phantom_vertex: PhantomData,
                shader_set
            }
        })
    }
}

pub trait RecreatePipeline: Sync + Send {
    fn drop_pipeline(&self);
    fn recreate_pipeline(&self, device: Arc<backend::Device>, render_pass: Arc<ManuallyDrop<<backend::Backend as Backend>::RenderPass>>, render_area: Extent2D) -> Result<(), Error> ;
}

impl<V: Vertex> RecreatePipeline for Pipeline<V> {
    fn drop_pipeline(&self) {
        let mut bundle = self.bundle.write().unwrap();
        bundle.take();
    }

    fn recreate_pipeline(&self, device: Arc<backend::Device>, render_pass: Arc<ManuallyDrop<<backend::Backend as Backend>::RenderPass>>, render_area: Extent2D) -> Result<(), Error> {
        let mut bundle_lock = self.bundle.write().unwrap();
        let bundle = PipelineBundle::new::<V>(device, render_pass, render_area, &self.shader_set)?;
        *bundle_lock = Some(bundle);
        Ok(())
    }
}

pub trait PipelineEncoderExt<V: Vertex> {
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V>);
}

impl<'a, V: Vertex> PipelineEncoderExt<V> for RenderPassInlineEncoder<'a, backend::Backend> {
    fn bind_pipeline(&mut self, pipeline: &Pipeline<V>) {
        let bundle = pipeline.bundle.read().unwrap();
        unsafe { self.bind_graphics_pipeline(bundle.as_ref().unwrap().pipeline()) }
    }
}