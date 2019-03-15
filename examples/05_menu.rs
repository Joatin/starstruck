use failure::Error;
use futures::future::Future;
use simplelog::Config;
use simplelog::LevelFilter;
use simplelog::TermLogger;
use starstruck::camera::DebugCamera;
use starstruck::graphics::Bundle;
use starstruck::graphics::Pipeline;
use starstruck::graphics::Texture;
use starstruck::primitive::Vertex3DUV;
use starstruck::Context;
use starstruck::CreateBundleFromObj;
use starstruck::CreateTexturedPipeline;
use starstruck::SetupContext;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use vek::vec::Vec3;
use starstruck::StarstruckBuilder;

// THIS IS OUR STATE WHERE WE STORE ALL OUR DATA
struct State {
    camera: DebugCamera,
    triangle_pipeline: Pipeline<Vertex3DUV>,
    triangle_bundle: Bundle<u16, Vertex3DUV>,
    _texture: Texture,
}

impl State {
    pub fn new(setup: Arc<SetupContext>) -> impl Future<Item = Self, Error = Error> {
        let pipeline_promise = setup.create_textured_pipeline();
        let bundle_promise = setup.create_bundle_from_obj(include_bytes!("assets/cube.obj"));
        let texture_promise = setup.create_texture_from_bytes(include_bytes!("assets/bricks.jpg"));

        pipeline_promise.join3(bundle_promise, texture_promise).map(
            |(pipeline, bundle, texture)| {
                let mut camera = DebugCamera::new();
                camera.set_position(Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -3.0,
                });
                sleep(Duration::from_millis(1000));

                pipeline.bind_texture(&texture);

                State {
                    _texture: texture,
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
fn main() -> Result<(), Error> {
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();

    let setup_callback = |setup| State::new(setup);

    let starstruck = StarstruckBuilder::new_with_setup(setup_callback)
        .with_render_callback(|(state, context)| state.render(context))
        .init()?;

    starstruck.run()?;

    Ok(())
}
