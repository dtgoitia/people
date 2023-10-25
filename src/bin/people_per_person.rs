use std::process;

use people::use_cases;
use people::use_cases::LogWritten;
use tracing::info;

use people::config;
use people::log;

fn main() {
    info!("Loading config...");
    let config = match config::get_config() {
        Ok(config) => config,
        Err(reason) => {
            eprintln!("ERROR: {}", reason);
            process::exit(1);
        }
    };

    let log = log::read_logs(&config.people_dir);
    let per_person_logs = use_cases::split_log_per_person(log, &config);
    for (person, person_log) in per_person_logs {
        match use_cases::write_person_log(person, person_log, config.get_per_person_dir()) {
            LogWritten::Written(path) => println!("Report written to {path:#?}"),
            LogWritten::FailedToWrite(path, reason) => {
                println!("ERROR: failed to write {path:#?}  --  reason: {reason}")
            }
            LogWritten::NothingToDelete(path) => println!("Nothing to delete: {path:#?}"),
            LogWritten::Deleted(path) => println!("Report deleted: {path:#?}"),
            LogWritten::FailedToDelete(path, reason) => println!("{path:#?}  --  reason: {reason}"),
        }
    }
}
