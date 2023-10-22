use std::process;

use tracing::info;

use people::config;
use people::log;

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

    let log = log::read_logs(config.people_dir);
    println!("{log:#?}");
}
