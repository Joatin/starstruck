extern crate starstruck;

mod bundle;

use failure::Error;
use crate::bundle::it_should_create_a_lot_of_bundles;
use std::panic;
use colored::*;
use std::time::Instant;
use simplelog::TermLogger;
use log::LevelFilter;
use simplelog::Config;

/// These tests require that they are started in the main thread
fn main() -> Result<(), Error> {
    TermLogger::init(LevelFilter::Info, Config::default()).unwrap();
    println!();

    let tests = vec![
        ("It should create a lot of bundles", it_should_create_a_lot_of_bundles)
    ];

    println!("running {} tests", tests.len());

    let mut success = 0;
    let mut fail = 0;

    for (test_name, test) in &tests {
        let now = Instant::now();
        match panic::catch_unwind(|| {
            test();
        }) {
            Ok(_) => {
                success += 1;
                println!("test {} ... {} {:?}", test_name, "ok".green(), now.elapsed());
            },
            Err(_) => {
                fail += 1;
                println!("test {} ... {}", test_name, "fail".red());
            }
        }
    }

    let result = if fail == 0 {
        "ok".green()
    } else {
        "fail".red()
    };

    println!();
    println!("test result: {}. {} passed; {} failed;", result, success, fail);
    println!();

    Ok(())
}