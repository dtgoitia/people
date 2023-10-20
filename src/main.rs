use std::process;

use tracing::info;

use crate::log::read_logs;

mod config;
mod log;
mod model;

fn exit_with_error(message: String) -> () {
    println!("ERROR: {}", message);
    process::exit(1);
}

fn main() {
    info!("Loading config...");
    let config = match config::get_config() {
        Ok(config) => config,
        Err(reason) => {
            return exit_with_error(reason);
        }
    };

    let log = read_logs(config.people_dir);
    println!("{log:#?}");
}
