//! Live Lead Sheet: windows, authoring, and wiring the parts together.
//!
//! `cargo run --release -p app -- [SONG_FILE [STUDIO_FILE]]`
//!
//! Opens the song (default `songs/Demo.song`) in the editor, with the
//! libraries in `library/`. With a studio file (default `songs/Demo.studio`
//! when the song is the default) it opens the studio's MIDI output ports so
//! the song can be played; without one, everything but playing works.

mod key_map;
mod playback;
mod song_files;
mod window_app;

use std::path::PathBuf;
use std::process::ExitCode;

use composer::library::Libraries;
use composer::session::Session;
use composer::song_document::SongDocument;
use player::studio_configuration::StudioConfiguration;

use crate::song_files::{SongFile, read_libraries, read_studio, read_text, short_name};
use crate::window_app::Startup;

const DEFAULT_SONG_FILE: &str = "songs/Demo.song";
const DEFAULT_STUDIO_FILE: &str = "songs/Demo.studio";

fn main() -> ExitCode {
    match start().and_then(window_app::run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Reads the song, the studio and the libraries.
fn start() -> Result<Startup, String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let song_path: PathBuf =
        PathBuf::from(arguments.first().map_or(DEFAULT_SONG_FILE, String::as_str));
    let studio_path: Option<PathBuf> = match (arguments.first(), arguments.get(1)) {
        (_, Some(studio)) => Some(PathBuf::from(studio)),
        (None, None) => Some(PathBuf::from(DEFAULT_STUDIO_FILE)),
        (Some(_), None) => None,
    };
    let text: String = read_text(&song_path, "song file")?;
    let document: SongDocument =
        SongDocument::new(&short_name(&song_path), text).map_err(|messages| {
            format!(
                "{} has mistakes, so the editor cannot open it. Fix them and start again:\n  {}",
                short_name(&song_path),
                messages.join("\n  ")
            )
        })?;
    let libraries: Libraries = read_libraries()?;
    let studio: Option<StudioConfiguration> = match &studio_path {
        Some(path) => match read_studio(path) {
            Ok(studio) => Some(studio),
            Err(problem) => {
                // The editor still opens; only playing is off.
                eprintln!("{problem}\nPlaying is off.");
                None
            }
        },
        None => None,
    };
    Ok(Startup {
        session: Session::new(document, libraries),
        song_file: SongFile::new(song_path),
        studio_path: studio.as_ref().and(studio_path),
        studio,
    })
}
