use log::LevelFilter;
use simplelog::Config;
use simplelog::TermLogger;
use starstruck::Starstruck;

fn main() {
    // Lets get some logs out
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();

    // The actual application
    let starstruck = Starstruck::init("01 Simple Window", |_| Ok(()), |_| Ok(())).unwrap();

    starstruck.run().unwrap();
}
