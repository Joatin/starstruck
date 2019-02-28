use crate::primitive::Index;
use crate::graphics::Bundle;
use failure::Error;
use crate::primitive::Vertex;
use std::sync::Arc;
use crate::graphics::Pipeline;
use futures::Future;
use crate::graphics::ShaderSet;
use crate::internal::graphics::GraphicsState;
use crate::graphics::RecreatePipeline;
use std::sync::Mutex;


pub struct SetupContext {
    state: Arc<GraphicsState>,
    pipelines: Arc<Mutex<Vec<Arc<RecreatePipeline>>>>
}

impl SetupContext {
    pub(crate) fn new(state: Arc<GraphicsState>) -> Self {
        Self {
            state,
            pipelines: Arc::new(Mutex::new(Vec::new()))
        }
    }


    pub fn create_bundle<I: Index, V: Vertex>(&self, indexes: &'static [I], vertexes: &'static [V]) -> impl Future<Item=Bundle<I, V>, Error=Error> + Send {
        Bundle::new(self.state.adapter(), Arc::new(Vec::from(indexes)), Arc::new(Vec::from(vertexes)))
    }

    pub(crate) fn create_bundle_owned<I: Index, V: Vertex>(&self, indexes: Arc<Vec<I>>, vertexes: Arc<Vec<V>>) -> impl Future<Item=Bundle<I, V>, Error=Error> + Send {
        Bundle::new(self.state.adapter(), indexes, vertexes)
    }

    pub fn create_pipeline<V: 'static + Vertex>(&self, shader_set: ShaderSet) -> impl Future<Item=Arc<Pipeline<V>>, Error=Error> + Send {
        let pipelines_mutex = Arc::clone(&self.pipelines);

        Pipeline::new(Arc::clone(&self.state), shader_set).map(move |pipeline| {
            let result = Arc::new(pipeline);
            let mut pipelines = pipelines_mutex.lock().unwrap();
            pipelines.push(Arc::clone(&result) as Arc<RecreatePipeline>);
            result
        })
    }

    pub fn drop_swapchain_dependant_data(&self) {
        let pipelines = self.pipelines.lock().unwrap();
        info!("Dropping all old pipelines");
        pipelines.iter().for_each(|pipe| {
            pipe.drop_pipeline()
        })
    }

    pub fn recreate_swapchain_dependant_data(&self) -> Result<(), Error> {

        info!("Recreating pipelines");
        let pipelines = self.pipelines.lock().unwrap();
        for pipe in pipelines.iter() {
            pipe.recreate_pipeline(& self.state)?
        };
        info!("All pipelines recreated");
        Ok(())
    }
}

pub trait CreateDefaultPipeline<V: Vertex> {
    fn create_default_pipeline(&self) -> Box<Future<Item=Arc<Pipeline<V>>, Error=Error> + Send>;
}

pub trait CreateBundleFromObj<I: Index, V: Vertex> {
    fn create_bundle_from_obj(&self, data: &[u8]) -> Box<Future<Item=Bundle<I, V>, Error=Error> + Send>;
}