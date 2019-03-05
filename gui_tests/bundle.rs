use starstruck::Starstruck;
use starstruck::primitive::Vertex2D;
use futures::future::join_all;
use futures::future::Future;

// OUR VERTICES
const VERTICES: [Vertex2D; 3] = [
    Vertex2D { x: -0.5, y: 0.5 },
    Vertex2D { x: 0.0, y: -0.5 },
    Vertex2D { x: 0.5, y: 0.5 },
];

// INDEXES
const INDEXES: [u16; 3] = [0, 1, 2];

pub fn it_should_create_a_lot_of_bundles() {
    let starstruck = Starstruck::init(
        "Test",
        |setup| {
            let mut bundles = Vec::with_capacity(100);
            for _i in 0..100 {
                bundles.push(setup.create_bundle(&INDEXES, &VERTICES));
            }
            join_all(bundles)
        },
        |(state, context)| {
            println!("{}", state.len());
            context.stop_starstruck();
            Ok(())
        }
    ).unwrap();

    starstruck.run().unwrap();
}