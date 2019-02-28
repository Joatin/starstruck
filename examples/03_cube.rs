use starstruck::Starstruck;
use starstruck::primitive::Vertex3D;
use simplelog::TermLogger;
use simplelog::LevelFilter;
use simplelog::Config;
use starstruck::graphics::Bundle;
use starstruck::graphics::Pipeline;
use futures::future::Future;
use std::sync::Arc;
use failure::Error;
use starstruck::setup_context::SetupContext;
use starstruck::setup_context::CreateDefaultPipeline;
use starstruck::setup_context::CreateBundleFromObj;
use starstruck::graphics::DebugCamera;
use starstruck::context::Context;

// THIS IS OUR STATE WHERE WE STORE ALL OUR DATA
struct State {
    camera: DebugCamera,
    triangle_pipeline: Arc<Pipeline<Vertex3D>>,
    triangle_bundle: Bundle<u16, Vertex3D>,
}

impl State {
    pub fn new(setup: Arc<SetupContext>) -> impl Future<Item=Self, Error=Error> {
        let pipeline_promise = setup.create_default_pipeline();
        let bundle_promise = setup.create_bundle_from_obj(include_bytes!("assets/cube.obj"));

        pipeline_promise.join(bundle_promise).map(|(pipeline, bundle)| {
            State {
                camera: DebugCamera::new(),
                triangle_pipeline: pipeline,
                triangle_bundle: bundle
            }
        })
    }

    pub fn render(&mut self, context: &mut Context) -> Result<(), Error> {
        self.camera.update_from_context(context);
        context.draw_with_camera(&self.triangle_pipeline, &self.triangle_bundle, &self.camera);
        Ok(())
    }
}

// MAIN
fn main() {
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();

    let starstruck = Starstruck::init(
        "02 Drawing Triangle",
        |setup| State::new(setup),
        |(state, context)| state.render(context)
    ).unwrap();

    starstruck.run().unwrap();
}