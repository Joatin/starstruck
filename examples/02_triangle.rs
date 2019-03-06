use failure::Error;
use futures::future::Future;
use simplelog::Config;
use simplelog::LevelFilter;
use simplelog::TermLogger;
use starstruck::graphics::Bundle;
use starstruck::graphics::Pipeline;
use starstruck::primitive::Vertex2D;
use starstruck::CreateDefaultPipeline;
use starstruck::SetupContext;
use starstruck::Starstruck;
use std::sync::Arc;
use starstruck::StarstruckBuilder;

// OUR VERTICES
const VERTICES: [Vertex2D; 3] = [
    Vertex2D { x: -0.5, y: 0.5 },
    Vertex2D { x: 0.0, y: -0.5 },
    Vertex2D { x: 0.5, y: 0.5 },
];

// INDEXES
const INDEXES: [u16; 3] = [0, 1, 2];

// THIS IS OUR STATE WHERE WE STORE ALL OUR DATA
struct State {
    triangle_pipeline: Arc<Pipeline<Vertex2D>>,
    triangle_bundle: Bundle<u16, Vertex2D>,
}

impl State {
    pub fn new(setup: &SetupContext) -> impl Future<Item = Self, Error = Error> {
        let pipeline_promise = setup.create_default_pipeline();
        let bundle_promise = setup.create_bundle(&INDEXES, &VERTICES);

        pipeline_promise
            .join(bundle_promise)
            .map(|(pipeline, bundle)| State {
                triangle_pipeline: pipeline,
                triangle_bundle: bundle,
            })
    }
}

// MAIN
fn main() -> Result<(), Error> {
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();

    let starstruck = StarstruckBuilder::new_with_setup(|setup| State::new(&setup))
        .with_render_callback(|(state, context)| {
            context.draw(&state.triangle_pipeline, &state.triangle_bundle);
            Ok(())
        })
        .init()?;

    starstruck.run()?;

    Ok(())
}
