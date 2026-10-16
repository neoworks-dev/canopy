//! The shared Neoworks background daemon. Every Neoworks desktop application
//! ships this binary and starts it on demand; the first one to launch wins the
//! daemon lock and the rest connect to it as clients.

use std::process::ExitCode;

use canopy_index::daemon;

fn main() -> ExitCode {
    let configuration = match daemon::Configuration::from_arguments(std::env::args().skip(1)) {
        Ok(configuration) => configuration,
        Err(message) => {
            eprintln!("canopyd: {message}");
            return ExitCode::from(2);
        }
    };
    match daemon::run(configuration) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("canopyd: {message}");
            ExitCode::FAILURE
        }
    }
}
