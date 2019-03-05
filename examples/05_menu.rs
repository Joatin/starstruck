use failure::Error;
use futures::future::Future;
use simplelog::Config;
use simplelog::LevelFilter;
use simplelog::TermLogger;
use starstruck::graphics::Bundle;
use starstruck::graphics::DebugCamera;
use starstruck::graphics::Pipeline;
use starstruck::graphics::Texture;
use starstruck::primitive::Vertex3DUV;
use starstruck::Context;
use starstruck::CreateBundleFromObj;
use starstruck::CreateTexturedPipeline;
use starstruck::SetupContext;
use starstruck::Starstruck;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use vek::vec::Vec3;

// THIS IS OUR STATE WHERE WE STORE ALL OUR DATA
struct State {
    camera: DebugCamera,
    triangle_pipeline: Arc<Pipeline<Vertex3DUV>>,
    triangle_bundle: Bundle<u16, Vertex3DUV>,
    texture: Texture,
}

impl State {
    pub fn new(setup: Arc<SetupContext>) -> impl Future<Item = Self, Error = Error> {
        let pipeline_promise = setup.create_textured_pipeline();
        let bundle_promise = setup.create_bundle_from_obj(include_bytes!("assets/cube.obj"));
        let texture_promise = setup.create_texture(include_bytes!("assets/bricks.jpg"));

        pipeline_promise.join3(bundle_promise, texture_promise).map(
            |(pipeline, bundle, texture)| {
                let mut camera = DebugCamera::new();
                camera.set_position(Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -3.0,
                });
                sleep(Duration::from_millis(5000));

                pipeline.bind_texture(&texture);

                State {
                    texture,
                    camera,
                    triangle_pipeline: pipeline,
                    triangle_bundle: bundle,
                }
            },
        )
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
        "05 Menu",
        |setup| State::new(setup),
        |(state, context)| state.render(context),
    )
    .unwrap();

    starstruck.run().unwrap();
}
