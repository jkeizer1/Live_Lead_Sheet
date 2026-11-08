//! Test: writes a song as a LilyPond file, to read it as sheet music.
//!
//! ```text
//! cargo run -p lilypond_export --example lilypond_export_test -- SONG_FILE [OUTPUT_FILE]
//! lilypond -o ~/tmp ~/tmp/Demo.ly
//! ```
//!
//! | Argument | Default | Meaning |
//! |---|---|---|
//! | `SONG_FILE` | (none) | the song to write |
//! | `OUTPUT_FILE` | `~/tmp/<song file name>.ly` | where the LilyPond file goes |
//!
//! Exit status is 0 when the file is written. See `explainer.md`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lilypond_export::score::song_to_lilypond;
use song_text::song_file::parse_song;
use vocabulary::song::Song;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some(song_file) = arguments.first() else {
        eprintln!(
            "Give the song file: cargo run -p lilypond_export --example lilypond_export_test \
             -- songs/Demo.song"
        );
        return ExitCode::FAILURE;
    };
    let output: PathBuf = match arguments.get(1) {
        Some(path) => PathBuf::from(path),
        None => match default_output(song_file) {
            Ok(path) => path,
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::FAILURE;
            }
        },
    };
    let text: String = match std::fs::read_to_string(song_file) {
        Ok(text) => text,
        Err(error) => {
            eprintln!(
                "Cannot read the song {song_file}: {error}. Run from the project folder, or \
                 give the file's full path."
            );
            return ExitCode::FAILURE;
        }
    };
    let song: Song = match parse_song(song_file, &text) {
        Ok(song) => song,
        Err(errors) => {
            eprintln!("{song_file} is not valid; nothing was written.");
            for (index, error) in errors.iter().enumerate() {
                eprintln!("  Issue {}: {error}", index + 1);
            }
            return ExitCode::FAILURE;
        }
    };
    let lilypond: String = match song_to_lilypond(&song) {
        Ok(lilypond) => lilypond,
        Err(error) => {
            eprintln!("{error}. Nothing was written.");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = std::fs::write(&output, lilypond) {
        eprintln!(
            "Cannot write {}: {error}. Give another OUTPUT_FILE.",
            output.display()
        );
        return ExitCode::FAILURE;
    }
    // A file name with no folder is in the current folder.
    let folder: &Path = match output.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    println!("Wrote {}.", output.display());
    println!(
        "Engrave it: lilypond -o {} {}",
        folder.display(),
        output.display()
    );
    ExitCode::SUCCESS
}

/// `~/tmp/<song file name>.ly`; ~/tmp must exist.
fn default_output(song_file: &str) -> Result<PathBuf, String> {
    let home: String = std::env::var("HOME").map_err(|_| {
        String::from("HOME is not set, so ~/tmp cannot be found. Give an OUTPUT_FILE.")
    })?;
    let folder: PathBuf = Path::new(&home).join("tmp");
    if !folder.is_dir() {
        return Err(format!(
            "{} does not exist. Make it (mkdir ~/tmp) or give an OUTPUT_FILE.",
            folder.display()
        ));
    }
    let stem: &str = Path::new(song_file)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("\"{song_file}\" has no file name. Give an OUTPUT_FILE."))?;
    Ok(folder.join(format!("{stem}.ly")))
}
