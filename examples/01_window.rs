use starstruck::Starstruck;
use simplelog::TermLogger;
use log::LevelFilter;
use simplelog::Config;

fn main() {
    // Lets get some logs out
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();

    // The actual application
    let starstruck = Starstruck::init(
        "01 Simple Window",
        |_| Ok(()),
        |_| Ok(())
    ).unwrap();

    starstruck.run().unwrap();
}