use log::LevelFilter;
use simplelog::Config;
use simplelog::TermLogger;
use starstruck::Starstruck;
use starstruck::StarstruckBuilder;
use failure::Error;

fn main() -> Result<(), Error> {
    // Lets get some logs out
    TermLogger::init(LevelFilter::Info, Config::default())?;

    let starstruck = StarstruckBuilder::new().init()?;
    starstruck.run()?;

    Ok(())
}
