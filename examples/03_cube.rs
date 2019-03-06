use failure::Error;
use futures::future::Future;
use simplelog::Config;
use simplelog::LevelFilter;
use simplelog::TermLogger;
use starstruck::camera::DebugCamera;
use starstruck::graphics::Bundle;
use starstruck::graphics::Pipeline;
use starstruck::primitive::Vertex3D;
use starstruck::Context;
use starstruck::CreateBundleFromObj;
use starstruck::CreateDefaultPipeline;
use starstruck::SetupContext;
use starstruck::Starstruck;
use std::sync::Arc;
use vek::vec::Vec3;
use starstruck::StarstruckBuilder;

// THIS IS OUR STATE WHERE WE STORE ALL OUR DATA
struct State {
    camera: DebugCamera,
    triangle_pipeline: Arc<Pipeline<Vertex3D>>,
    triangle_bundle: Bundle<u16, Vertex3D>,
}

impl State {
    pub fn new(setup: Arc<SetupContext>) -> impl Future<Item = Self, Error = Error> {
        let pipeline_promise = setup.create_default_pipeline();
        let bundle_promise = setup.create_bundle_from_obj(include_bytes!("assets/cube.obj"));

        pipeline_promise
            .join(bundle_promise)
            .map(|(pipeline, bundle)| {
                let mut camera = DebugCamera::new();
                camera.set_position(Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -3.0,
                });

                State {
                    camera,
                    triangle_pipeline: pipeline,
                    triangle_bundle: bundle,
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
fn main() -> Result<(), Error> {
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();

    let setup_callback = |setup| State::new(setup);

    let starstruck = StarstruckBuilder::new_with_setup(setup_callback)
        .with_render_callback(|(state, context)| state.render(context))
        .init()?;

    starstruck.run()?;

    Ok(())
}
