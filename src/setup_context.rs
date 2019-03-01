use crate::graphics::Bundle;
use crate::graphics::Pipeline;
use crate::graphics::RecreatePipeline;
use crate::graphics::ShaderSet;
use crate::internal::graphics::GraphicsState;
use crate::primitive::Index;
use crate::primitive::Vertex;
use failure::Error;
use futures::Future;
use std::sync::Arc;
use std::sync::Mutex;
use crate::graphics::Texture;
use gfx_hal::Backend;
use gfx_hal::Device;
use gfx_hal::Instance;

pub struct SetupContext<B: Backend = backend::Backend, D: Device<B> = backend::Device, I: Instance<Backend=B> = backend::Instance> {
    state: Arc<GraphicsState<B, D, I>>,
    pipelines: Arc<Mutex<Vec<Arc<RecreatePipeline<B, D, I>>>>>,
}

impl<B: Backend, D: Device<B>, I: Instance<Backend=B>> SetupContext<B, D, I> {
    pub(crate) fn new(state: Arc<GraphicsState<B, D, I>>) -> Self {
        Self {
            state,
            pipelines: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn create_bundle<In: Index, V: Vertex>(
        &self,
        indexes: &'static [In],
        vertexes: &'static [V],
    ) -> impl Future<Item = Bundle<In, V, B, D, I>, Error = Error> + Send {
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
    ) -> impl Future<Item = Bundle<In, V, B, D, I>, Error = Error> + Send {
        Bundle::new(Arc::clone(&self.state), indexes, vertexes)
    }

    pub fn create_pipeline<V: 'static + Vertex>(
        &self,
        shader_set: ShaderSet,
    ) -> impl Future<Item = Arc<Pipeline<V, B, D>>, Error = Error> + Send {
        let pipelines_mutex = Arc::clone(&self.pipelines);

        Pipeline::new(Arc::clone(&self.state), shader_set).map(move |pipeline| {
            let result = Arc::new(pipeline);
            let mut pipelines = pipelines_mutex.lock().unwrap();
            pipelines.push(Arc::clone(&result) as Arc<RecreatePipeline<B, D, I>>);
            result
        })
    }

    pub fn create_texture(&self, image_data: &'static [u8]) -> impl Future<Item = Texture<B, D, I>, Error = Error> + Send {
        Texture::new(Arc::clone(&self.state), image_data)
    }

    pub fn drop_swapchain_dependant_data(&self) {
        let pipelines = self.pipelines.lock().unwrap();
        info!("Dropping all old pipelines");
        pipelines.iter().for_each(|pipe| pipe.drop_pipeline())
    }

    pub fn recreate_swapchain_dependant_data(&self) -> Result<(), Error> {
        info!("Recreating pipelines");
        let pipelines = self.pipelines.lock().unwrap();
        for pipe in pipelines.iter() {
            pipe.recreate_pipeline(&self.state)?
        }
        info!("All pipelines recreated");
        Ok(())
    }
}

pub trait CreateDefaultPipeline<V: Vertex, B: Backend, D: Device<B>> {
    fn create_default_pipeline(&self)
        -> Box<Future<Item = Arc<Pipeline<V, B, D>>, Error = Error> + Send>;
}

pub trait CreateBundleFromObj<In: Index, V: Vertex, B: Backend, D: Device<B>, I: Instance<Backend=B>> {
    fn create_bundle_from_obj(
        &self,
        data: &[u8],
    ) -> Box<Future<Item = Bundle<In, V, B, D, I>, Error = Error> + Send>;
}
