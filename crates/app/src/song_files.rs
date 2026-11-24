//! The files the editor works with: the song (read, saved, and watched for
//! changes made in another editor), the studio configuration (read once,
//! to play), and the two libraries. This is the file adapter (A-3): the
//! composer and render crates never touch a file.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use composer::folder_browser::{BrowserEntry, BrowserEntryKind};
use composer::library::Libraries;
use composer::new_song::{NewSongFiles, check_song_name, new_song_files};
use composer::repair::{RepairFile, RepairFileKind};
use composer::session::{MessageKind, Session};
use composer::song_document::SongDocument;
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

/// A song read from its folder, ready to edit.
pub struct LoadedSong {
    pub session: Session,
    pub song_file: SongFile,
    /// The studio, when it was found and read.
    pub studio: Option<StudioConfiguration>,
    pub studio_path: Option<PathBuf>,
    /// The studio file's text as read, whose top comment block is kept when
    /// the studio is written back.
    pub studio_text: Option<String>,
}

/// Why a song did not open: a line for the message, and every mistake
/// for the terminal.
pub struct LoadFailure {
    pub summary: String,
    pub details: Vec<String>,
    /// The file texts to repair in the window, when the song or its studio
    /// was read but has mistakes.
    pub repair: Option<RepairSource>,
}

/// The files of a song whose readers refused them, to be repaired.
pub struct RepairSource {
    pub song_path: PathBuf,
    /// The studio file, when there is one beside the song.
    pub studio_path: Option<PathBuf>,
    /// The song first, then the studio.
    pub files: Vec<RepairFile>,
}

/// Writes every mistake to the terminal, where it can be copied whole, and
/// returns the one line for the window.
#[must_use]
pub fn report_failure(song_path: &Path, failure: &LoadFailure) -> (MessageKind, String) {
    eprintln!("Cannot open {}:", song_path.display());
    for detail in &failure.details {
        eprintln!("  {detail}");
    }
    (MessageKind::Problem, failure.summary.clone())
}

/// The studio beside a song: the same name, in the same folder.
#[must_use]
pub fn studio_path_beside(song_path: &Path) -> PathBuf {
    song_path.with_extension("studio")
}

/// Reads a song and the studio it plays through (the one beside it unless
/// `studio_path` names another), and the libraries. A studio that is missing
/// cannot be read does not stop the song opening: playing is off, and the
/// message says why.
///
/// # Errors
///
/// When the song or the libraries cannot be read, or the song or its studio
/// has mistakes (then `repair` holds both texts).
pub fn load_song(song_path: &Path, studio_path: Option<&Path>) -> Result<LoadedSong, LoadFailure> {
    let name: String = short_name(song_path);
    let text: String =
        read_text(song_path, "song file").map_err(|problem: String| LoadFailure {
            summary: problem.clone(),
            details: vec![problem],
            repair: None,
        })?;
    let studio_path: PathBuf =
        studio_path.map_or_else(|| studio_path_beside(song_path), Path::to_path_buf);
    let studio_name: String = short_name(&studio_path);
    // A studio that is missing, or cannot be read, leaves the song open
    // with playing off; one that is read but has mistakes goes to repair.
    let (studio_text, mut studio_note): (Option<String>, Option<String>) = if studio_path.exists() {
        match read_text(&studio_path, "studio file") {
            Ok(studio_text) => (Some(studio_text), None),
            Err(problem) => (None, Some(format!("{problem}\nPlaying is off."))),
        }
    } else {
        (
            None,
            Some(format!(
                "No {studio_name} beside the song, so playing is off."
            )),
        )
    };
    let document: Result<SongDocument, Vec<String>> = SongDocument::new(&name, text.clone());
    let studio: Option<Result<StudioConfiguration, Vec<ParseError>>> = studio_text
        .as_ref()
        .map(|studio_text: &String| parse_studio(&studio_name, studio_text));
    let studio_mistakes: Vec<String> = match &studio {
        Some(Err(errors)) => errors.iter().map(ParseError::to_string).collect(),
        _ => Vec::new(),
    };
    let document: SongDocument = match document {
        Ok(document) if studio_mistakes.is_empty() => document,
        failed => {
            // Either file has mistakes: both go to the repair view.
            let mut mistakes: Vec<String> = failed.err().unwrap_or_default();
            mistakes.extend(studio_mistakes);
            let mut files: Vec<RepairFile> = vec![RepairFile {
                kind: RepairFileKind::Song,
                file_name: name.clone(),
                text,
            }];
            let mut repair_studio_path: Option<PathBuf> = None;
            if let Some(studio_text) = studio_text {
                files.push(RepairFile {
                    kind: RepairFileKind::Studio,
                    file_name: studio_name,
                    text: studio_text,
                });
                repair_studio_path = Some(studio_path);
            }
            return Err(LoadFailure {
                summary: format!(
                    "{} to repair in {name}. Playing and editing are off until they are fixed.",
                    if mistakes.len() == 1 {
                        String::from("1 mistake")
                    } else {
                        format!("{} mistakes", mistakes.len())
                    }
                ),
                details: mistakes,
                repair: Some(RepairSource {
                    song_path: song_path.to_path_buf(),
                    studio_path: repair_studio_path,
                    files,
                }),
            });
        }
    };
    let libraries: Libraries = read_libraries().map_err(|problem: String| LoadFailure {
        summary: problem.clone(),
        details: vec![problem],
        repair: None,
    })?;
    let studio: Option<StudioConfiguration> = match studio {
        Some(Ok(studio)) => Some(studio),
        _ => None,
    };
    if studio.is_none() && studio_note.is_none() {
        studio_note = Some(String::from(
            "The studio could not be read. Playing is off.",
        ));
    }
    let mut session: Session = Session::new(document, libraries);
    if let Some(note) = studio_note {
        session.say(MessageKind::Problem, note);
    }
    Ok(LoadedSong {
        session,
        song_file: SongFile::new(song_path.to_path_buf()),
        studio_path: studio.as_ref().map(|_| studio_path),
        studio_text: studio.as_ref().and(studio_text),
        studio,
    })
}

/// The folders and song files in a folder, for the browser. Names that
/// start with a dot are left out.
///
/// # Errors
///
/// When the folder cannot be listed.
pub fn list_folder(path: &Path) -> Result<Vec<BrowserEntry>, String> {
    let listing: std::fs::ReadDir = std::fs::read_dir(path)
        .map_err(|error| format!("Cannot list {}: {error}.", path.display()))?;
    let mut entries: Vec<BrowserEntry> = Vec::new();
    for item in listing {
        let item: std::fs::DirEntry =
            item.map_err(|error| format!("Cannot list {}: {error}.", path.display()))?;
        let name: String = item.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let is_folder: bool = item.path().is_dir();
        if is_folder {
            let song_inside: PathBuf = item.path().join(format!("{name}.song"));
            entries.push(BrowserEntry {
                kind: if song_inside.is_file() {
                    BrowserEntryKind::SongFolder
                } else {
                    BrowserEntryKind::Folder
                },
                name,
            });
        } else if Path::new(&name)
            .extension()
            .is_some_and(|extension| extension == "song")
        {
            entries.push(BrowserEntry {
                name,
                kind: BrowserEntryKind::SongFile,
            });
        }
    }
    Ok(entries)
}

/// Makes the folder `name` inside `parent`, and gives its path.
///
/// # Errors
///
/// A sentence for the musician: the name is not allowed, the name is taken,
/// or what the system said.
pub fn make_folder(parent: &Path, name: &str) -> Result<PathBuf, String> {
    check_song_name(name)?;
    let folder: PathBuf = parent.join(name);
    if folder.exists() {
        return Err(format!(
            "A folder or file named {name} is already there. Type another name."
        ));
    }
    std::fs::create_dir(&folder)
        .map_err(|error| format!("Cannot make the folder {}: {error}.", folder.display()))?;
    Ok(folder)
}

/// The path of a song's file of the given kind, named for its folder.
fn file_in(folder: &Path, name: &str, extension: &str) -> PathBuf {
    folder.join(format!("{name}.{extension}"))
}

/// Writes a file that must not be there already.
fn write_new(path: &Path, text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file: std::fs::File = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("Cannot write {}: {error}.", path.display()))?;
    file.write_all(text.as_bytes())
        .map_err(|error| format!("Cannot write {}: {error}.", path.display()))
}

/// Makes a new song in `folder`, named for the folder: `name.song`,
/// `name.studio` and `name.toml`. Gives the song's path.
///
/// # Errors
///
/// A sentence for the musician: the folder holds a song already, its name
/// is not allowed as a song's, or what the system said.
pub fn make_song_in_folder(folder: &Path) -> Result<PathBuf, String> {
    let name: String = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| String::from("Choose a folder with a name of its own."))?;
    let files: NewSongFiles = new_song_files(&name)?;
    let listing = std::fs::read_dir(folder)
        .map_err(|error| format!("Cannot read the folder {}: {error}.", folder.display()))?;
    for entry in listing.flatten() {
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "song")
        {
            return Err(String::from(
                "That folder already holds a song. Make or choose another folder.",
            ));
        }
    }
    write_new(&file_in(folder, &name, "song"), &files.song_text)?;
    write_new(&file_in(folder, &name, "studio"), &files.studio_text)?;
    write_new(&file_in(folder, &name, "toml"), &files.router_text)?;
    Ok(file_in(folder, &name, "song"))
}

/// Saves a song in `folder` under the folder's name: `name.song` from
/// `song_text`, `name.studio` from `studio_text` when there is one, and
/// `name.toml` copied from the old song's router file when it has one.
/// Nothing is overwritten. Gives the new song's path.
///
/// # Errors
///
/// A sentence for the musician: the folder holds a song already, its name is
/// not allowed as a song's, or what the system said.
pub fn save_song_as(
    folder: &Path,
    old_song_path: &Path,
    song_text: &str,
    studio_text: Option<&str>,
) -> Result<PathBuf, String> {
    let name: String = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| String::from("Choose a folder with a name of its own."))?;
    check_song_name(&name)?;
    let listing = std::fs::read_dir(folder)
        .map_err(|error| format!("Cannot read the folder {}: {error}.", folder.display()))?;
    for entry in listing.flatten() {
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "song")
        {
            return Err(String::from(
                "That folder already holds a song. Make or choose another folder.",
            ));
        }
    }
    write_new(&file_in(folder, &name, "song"), song_text)?;
    if let Some(text) = studio_text {
        write_new(&file_in(folder, &name, "studio"), text)?;
    }
    let old_router: PathBuf = old_song_path.with_extension("toml");
    if old_router.is_file() {
        let text: String = read_text(&old_router, "router file")?;
        write_new(&file_in(folder, &name, "toml"), &text)?;
    }
    Ok(file_in(folder, &name, "song"))
}

/// Copies the song folder `source` to a new folder `name` in `parent`:
/// each file named for the old song (`Old.song`, `Old.studio`, `Old.toml`)
/// is copied under the new name; other files keep their names. Folders
/// inside are left out. Gives the new song's path.
///
/// # Errors
///
/// A sentence for the musician: the source holds no song of its name, the
/// name is not allowed or taken, or what the system said.
pub fn copy_song(source: &Path, parent: &Path, name: &str) -> Result<PathBuf, String> {
    let old_name: String = source
        .file_name()
        .map(|old| old.to_string_lossy().into_owned())
        .ok_or_else(|| String::from("Choose a song folder to copy."))?;
    if !file_in(source, &old_name, "song").is_file() {
        return Err(format!("{old_name} holds no {old_name}.song to copy."));
    }
    let folder: PathBuf = make_folder(parent, name)?;
    let listing = std::fs::read_dir(source)
        .map_err(|error| format!("Cannot read the folder {}: {error}.", source.display()))?;
    for entry in listing.flatten() {
        let from: PathBuf = entry.path();
        if !from.is_file() {
            continue;
        }
        let file_name: String = entry.file_name().to_string_lossy().into_owned();
        let renamed: String = match file_name.strip_prefix(&format!("{old_name}.")) {
            Some(extension) => format!("{name}.{extension}"),
            None => file_name,
        };
        let to: PathBuf = folder.join(renamed);
        std::fs::copy(&from, &to).map_err(|error| {
            format!(
                "Cannot copy {} to {}: {error}.",
                from.display(),
                to.display()
            )
        })?;
    }
    Ok(file_in(&folder, name, "song"))
}

/// Where the browser starts: the `songs` folder beside where the program
/// runs, else the project's own.
#[must_use]
pub fn default_song_folder() -> PathBuf {
    let here: PathBuf = PathBuf::from("songs");
    if here.is_dir() {
        // If the full path cannot be found, the relative one lists the same
        // folder, so falling back to it is harmless.
        std::fs::canonicalize(&here).unwrap_or(here)
    } else {
        Path::new(PROJECT_DIR).join("songs")
    }
}

/// The song file on disk, and when it last changed, so a change made in
/// another editor can be taken up (EFS-5.8, EFS-5.11).
pub struct SongFile {
    pub path: PathBuf,
    modified: Option<SystemTime>,
    /// What the file is called in a message: "song file" or "studio file".
    what: &'static str,
}

impl SongFile {
    #[must_use]
    pub fn new(path: PathBuf) -> SongFile {
        SongFile::watching(path, "song file")
    }

    /// Watches a file of another kind the same way (the studio file).
    #[must_use]
    pub fn watching(path: PathBuf, what: &'static str) -> SongFile {
        let modified: Option<SystemTime> = modified_time(&path);
        SongFile {
            path,
            modified,
            what,
        }
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
        read_text(&self.path, self.what).map(Some)
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

#[cfg(test)]
mod new_song_tests {
    use super::{copy_song, load_song, make_folder, make_song_in_folder};
    use std::path::PathBuf;

    fn scratch_parent(what: &str) -> PathBuf {
        let parent: PathBuf =
            std::env::temp_dir().join(format!("lls_{what}_{}", std::process::id()));
        std::fs::create_dir_all(&parent).unwrap();
        parent
    }

    #[test]
    fn a_folder_is_made_then_holds_a_new_song_and_not_a_second() {
        let parent: PathBuf = scratch_parent("new_song");
        let folder: PathBuf = make_folder(&parent, "Blue Bossa").unwrap();
        assert!(
            make_folder(&parent, "Blue Bossa")
                .unwrap_err()
                .contains("already there")
        );
        let song_path: PathBuf = make_song_in_folder(&folder).unwrap();
        for extension in ["song", "studio", "toml"] {
            assert!(folder.join(format!("Blue Bossa.{extension}")).is_file());
        }
        assert!(load_song(&song_path, None).is_ok(), "the new song opens");
        let again: String = make_song_in_folder(&folder).unwrap_err();
        assert!(again.contains("already holds a song"), "{again}");
        std::fs::remove_dir_all(&parent).unwrap();
    }

    #[test]
    fn a_copy_renames_the_files_named_for_the_old_song() {
        let parent: PathBuf = scratch_parent("copy_song");
        let source: PathBuf = make_folder(&parent, "Old").unwrap();
        make_song_in_folder(&source).unwrap();
        std::fs::write(source.join("notes.txt"), "keep").unwrap();
        let copied: PathBuf = copy_song(&source, &parent, "New").unwrap();
        assert_eq!(copied, parent.join("New").join("New.song"));
        for kept in ["New.song", "New.studio", "New.toml", "notes.txt"] {
            assert!(parent.join("New").join(kept).is_file(), "{kept}");
        }
        assert!(!parent.join("New").join("Old.song").exists());
        assert!(load_song(&copied, None).is_ok());
        assert!(
            copy_song(&source, &parent, "New")
                .unwrap_err()
                .contains("already there")
        );
        std::fs::remove_dir_all(&parent).unwrap();
    }

    #[test]
    fn a_watched_file_reports_a_change_made_elsewhere_but_not_our_own_save() {
        let parent: PathBuf = scratch_parent("watch");
        let path: PathBuf = parent.join("W.studio");
        std::fs::write(&path, "one").unwrap();
        let mut watch = super::SongFile::watching(path.clone(), "studio file");
        assert_eq!(watch.changed_text().unwrap(), None);
        std::thread::sleep(std::time::Duration::from_millis(30));
        std::fs::write(&path, "two").unwrap();
        assert_eq!(watch.changed_text().unwrap(), Some(String::from("two")));
        assert_eq!(watch.changed_text().unwrap(), None, "reported once");
        std::thread::sleep(std::time::Duration::from_millis(30));
        watch.save("three").unwrap();
        assert_eq!(watch.changed_text().unwrap(), None, "our own write");
        std::fs::remove_dir_all(&parent).unwrap();
    }

    #[test]
    fn save_as_writes_the_given_texts_and_copies_the_router_file() {
        let parent: PathBuf = scratch_parent("save_as");
        let old: PathBuf = make_folder(&parent, "Old").unwrap();
        let old_song: PathBuf = make_song_in_folder(&old).unwrap();
        let target: PathBuf = make_folder(&parent, "Newer").unwrap();
        let saved: PathBuf = super::save_song_as(
            &target,
            &old_song,
            "edited song text",
            Some("edited studio text"),
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&saved).unwrap(), "edited song text");
        assert_eq!(
            std::fs::read_to_string(target.join("Newer.studio")).unwrap(),
            "edited studio text"
        );
        assert!(target.join("Newer.toml").is_file(), "router file copied");
        assert!(old.join("Old.song").is_file(), "the old song stays");
        let again: String = super::save_song_as(&target, &old_song, "x", None).unwrap_err();
        assert!(again.contains("already holds a song"), "{again}");
        std::fs::remove_dir_all(&parent).unwrap();
    }
}
