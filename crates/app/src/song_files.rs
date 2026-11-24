//! The files the editor works with: the song (read, saved, and watched for
//! changes made in another editor), the studio configuration (read once,
//! to play), and the two libraries. This is the file adapter (A-3): the
//! composer and render crates never touch a file.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use composer::library::Libraries;
use player::studio_configuration::StudioConfiguration;
use song_text::parse_error::ParseError;
use song_text::studio_file::parse_studio;

/// The project folder this program was built in: where the libraries are
/// found when the program is not run from the project folder.
const PROJECT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

const PROGRESSION_LIBRARY: &str = "library/Progressions.library";
const RHYTHM_LIBRARY: &str = "library/Rhythms.library";

/// A file's own name, for messages read at a glance.
#[must_use]
pub fn short_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().to_string(),
    )
}

/// Reads a text file, saying which file and why when it cannot.
///
/// # Errors
///
/// When the file cannot be read.
pub fn read_text(path: &Path, what: &str) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("Cannot read the {what} {}: {error}.", path.display()))
}

/// The library file, from the folder the program runs in, else from the
/// project folder it was built in.
fn library_path(relative: &str) -> PathBuf {
    let here: PathBuf = PathBuf::from(relative);
    if here.exists() {
        here
    } else {
        Path::new(PROJECT_DIR).join(relative)
    }
}

/// Reads both libraries.
///
/// # Errors
///
/// When a library cannot be read or has mistakes; the message lists them.
pub fn read_libraries() -> Result<Libraries, String> {
    let progression_path: PathBuf = library_path(PROGRESSION_LIBRARY);
    let rhythm_path: PathBuf = library_path(RHYTHM_LIBRARY);
    let progressions: String = read_text(&progression_path, "progression library")?;
    let rhythms: String = read_text(&rhythm_path, "rhythm library")?;
    Libraries::read(
        &short_name(&progression_path),
        &progressions,
        &short_name(&rhythm_path),
        &rhythms,
    )
    .map_err(|messages| format!("The libraries have mistakes:\n  {}", messages.join("\n  ")))
}

/// Reads a studio configuration.
///
/// # Errors
///
/// When it cannot be read or has mistakes.
pub fn read_studio(path: &Path) -> Result<StudioConfiguration, String> {
    let text: String = read_text(path, "studio file")?;
    parse_studio(&short_name(path), &text).map_err(|errors: Vec<ParseError>| {
        let lines: Vec<String> = errors.iter().map(ParseError::line_and_message).collect();
        format!(
            "{} has mistakes:\n  {}",
            short_name(path),
            lines.join("\n  ")
        )
    })
}

/// The song file on disk, and when it last changed, so a change made in
/// another editor can be taken up (EFS-5.8, EFS-5.11).
pub struct SongFile {
    pub path: PathBuf,
    modified: Option<SystemTime>,
}

impl SongFile {
    #[must_use]
    pub fn new(path: PathBuf) -> SongFile {
        let modified: Option<SystemTime> = modified_time(&path);
        SongFile { path, modified }
    }

    /// Writes the text, and notes the time so the write is not taken for a
    /// change made elsewhere.
    ///
    /// # Errors
    ///
    /// When the file cannot be written.
    pub fn save(&mut self, text: &str) -> Result<(), String> {
        std::fs::write(&self.path, text)
            .map_err(|error| format!("Cannot save {}: {error}.", self.path.display()))?;
        self.modified = modified_time(&self.path);
        Ok(())
    }

    /// The file's new text, when it changed since it was last read or
    /// written.
    ///
    /// # Errors
    ///
    /// When the file changed but cannot be read.
    pub fn changed_text(&mut self) -> Result<Option<String>, String> {
        let now: Option<SystemTime> = modified_time(&self.path);
        if now.is_none() || now == self.modified {
            return Ok(None);
        }
        self.modified = now;
        read_text(&self.path, "song file").map(Some)
    }
}

/// When the file last changed. `None` when that cannot be read (the file
/// is being replaced by another editor at that moment): the next look,
/// a second later, finds it, so nothing is lost by waiting.
fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}
