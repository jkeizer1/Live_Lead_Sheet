//! Live Lead Sheet: windows, authoring, and wiring the parts together.
//!
//! `cargo run --release -p app -- [SONG_FILE [STUDIO_FILE]]`
//!
//! With no arguments the window opens with no song, and File, Open song
//! chooses one. A song given on the command line (or opened by double-click)
//! is opened with the studio of the same name beside it, unless a studio file
//! is given. A song that does not open never stops the program: the window
//! opens and says why, and a song or studio with mistakes opens the repair
//! view.

mod clipboard;
mod key_map;
mod repair_work;
mod song_files;
mod window_app;
mod workspace;

use std::path::PathBuf;
use std::process::ExitCode;

use crate::song_files::{load_song, report_failure};
use crate::window_app::Startup;

fn main() -> ExitCode {
    match window_app::run(start()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// What to open first, from the command line.
fn start() -> Startup {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some(song_argument) = arguments.first() else {
        return Startup {
            song: None,
            repair: None,
            message: None,
        };
    };
    let song_path: PathBuf = PathBuf::from(song_argument);
    let studio_path: Option<PathBuf> = arguments.get(1).map(PathBuf::from);
    match load_song(&song_path, studio_path.as_deref()) {
        Ok(song) => Startup {
            song: Some(song),
            repair: None,
            message: None,
        },
        Err(failure) => Startup {
            song: None,
            message: Some(report_failure(&song_path, &failure)),
            repair: failure.repair,
        },
    }
}
