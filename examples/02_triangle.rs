use starstruck::Starstruck;
use starstruck::primitive::Vertex2D;
use simplelog::TermLogger;
use simplelog::LevelFilter;
use simplelog::Config;
use starstruck::graphics::Bundle;
use starstruck::graphics::Pipeline;
use starstruck::graphics::ShaderSet;
use starstruck::graphics::ShaderDescription;
use futures::future::Future;
use std::sync::Arc;

const VERTICES: [Vertex2D; 3] = [
   Vertex2D { x: -0.5, y: 0.5 },
   Vertex2D { x: 0.0, y: -0.5 },
   Vertex2D { x: 0.5, y: 0.5 }
];

const INDEXES: [u16; 3] = [0, 1, 2];

struct State {
    triangle_pipeline: Arc<Pipeline<Vertex2D>>,
    triangle_bundle: Bundle<u16, Vertex2D>,
}

const SHADERS: ShaderSet = ShaderSet {
    vertex: ShaderDescription { spirv: include_bytes!("assets/gen/shaders/02_traingle.vert.spv"), constant_byte_size: 0 },
    hull: None,
    domain: None,
    geometry: None,
    fragment: Some(ShaderDescription { spirv: include_bytes!("assets/gen/shaders/02_traingle.frag.spv"), constant_byte_size: 0 })
};

fn main() {
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();

    let starstruck = Starstruck::<State>::init("02 Drawing Triangle", |setup| {
        let pipeline_promise = setup.create_pipeline(SHADERS);
        let bundle_promise = setup.create_bundle(&INDEXES, &VERTICES);

        pipeline_promise.join(bundle_promise).map(|(pipeline, bundle)| {
            State {
                triangle_pipeline: pipeline,
                triangle_bundle: bundle
            }
        })
    }).unwrap();

    starstruck.start(|(state, context)| {
        context.draw(&state.triangle_pipeline, &state.triangle_bundle);
        Ok(())
    }).unwrap();
}