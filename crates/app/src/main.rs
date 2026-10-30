//! Live Lead Sheet: windows, authoring, and wiring the parts together.

mod window_app;

use std::process::ExitCode;

fn main() -> ExitCode {
    match window_app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
