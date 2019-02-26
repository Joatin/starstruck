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


    pub fn create_bundle<'b, I: Index, V: Vertex>(&self, indexes: &'b [I], vertexes: &'b [V]) -> impl Future<Item=Bundle<I, V>, Error=Error> + 'b + Send {
        Bundle::new(self.state.adapter(), indexes, vertexes)
    }

    pub fn create_pipeline<V: 'static + Vertex>(&self, shader_set: ShaderSet) -> impl Future<Item=Arc<Pipeline<V>>, Error=Error> + Send {
        let device = self.state.device();
        let render_pass = self.state.render_pass();
        let render_area = self.state.render_area();
        let pipelines_guard = Arc::clone(&self.pipelines);

        Pipeline::new(device, render_pass, render_area, shader_set).map(move |pipeline| {
            let result = Arc::new(pipeline);
            let mut pipelines = pipelines_guard.lock().unwrap();
            pipelines.push(Arc::clone(&result) as Arc<RecreatePipeline>);
            result
        })
    }

    pub fn drop_swapchain_dependant_data(&self) {
        let pipelines = self.pipelines.lock().unwrap();
        pipelines.iter().for_each(|pipe| {
            pipe.drop_pipeline()
        })
    }

    pub fn recreate_swapchain_dependant_data(&self) -> Result<(), Error> {
        let device = self.state.device();
        let render_pass = self.state.render_pass();
        let render_area = self.state.render_area();

        let pipelines = self.pipelines.lock().unwrap();
        for pipe in pipelines.iter() {
            pipe.recreate_pipeline(Arc::clone(&device), Arc::clone(&render_pass), render_area)?
        };
        Ok(())
    }
}