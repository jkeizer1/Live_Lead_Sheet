//! The drawn folder browser that opens a song (no native file dialog, so no
//! GTK). It shows one folder at a time: its sub-folders, the song folders
//! among them (a folder holding a song of its own name, the usual layout),
//! and loose `.song` files. This crate touches no file, so the browser never
//! reads a folder itself: it asks (`BrowserRequest::List`), the app lists the
//! folder and hands the listing back (`set_listing`).

use std::path::{Path, PathBuf};

use crate::scroll_list::ScrollList;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserEntryKind {
    /// The row that goes up to the enclosing folder.
    ParentFolder,
    Folder,
    /// A folder holding `<its name>.song`: choosing it opens that song.
    SongFolder,
    SongFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserEntry {
    /// The file or folder's own name, as the listing found it.
    pub name: String,
    pub kind: BrowserEntryKind,
}

/// What the browser asks the app for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserRequest {
    /// List this folder and call `set_listing`.
    List(PathBuf),
    /// Open this song file.
    Open(PathBuf),
    /// Use this folder as the new song's folder.
    ChooseFolder(PathBuf),
    /// Ask for the name of a new folder inside the folder shown.
    NewFolder,
    /// Ask for the name of a copy of this song folder, made beside it.
    CopySong(PathBuf),
}

/// What the browser is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserPurpose {
    /// Choose a song to open.
    OpenSong,
    /// Choose the folder that becomes a new song's own folder, or make one
    /// (inside the folder shown) and choose that. A folder that already
    /// holds a song is not a choice, and song files are of no use.
    ChooseFolderForNewSong,
    /// The same, for the open song saved under a new name.
    ChooseFolderForSaveAs,
}

impl BrowserPurpose {
    /// Whether the browser chooses a folder (rather than a song to open).
    #[must_use]
    pub const fn chooses_folder(self) -> bool {
        !matches!(self, BrowserPurpose::OpenSong)
    }
}

/// How many rows the list shows at once. The screen draws exactly this many,
/// so the browser can keep the selected row in view without asking it.
pub const VISIBLE_ROWS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderBrowser {
    directory: PathBuf,
    entries: Vec<BrowserEntry>,
    /// The selection and the view over `entries`.
    list: ScrollList,
    /// Why the last listing failed, in the musician's terms.
    problem: Option<String>,
    purpose: BrowserPurpose,
}

impl FolderBrowser {
    /// A browser at `directory`, empty until the app hands in its listing.
    #[must_use]
    pub fn new(directory: PathBuf) -> FolderBrowser {
        FolderBrowser {
            directory,
            entries: Vec::new(),
            list: ScrollList::new(VISIBLE_ROWS),
            problem: None,
            purpose: BrowserPurpose::OpenSong,
        }
    }

    /// A browser that chooses the folder for a new song.
    #[must_use]
    pub fn for_new_song(directory: PathBuf) -> FolderBrowser {
        FolderBrowser::choosing_folder(directory, BrowserPurpose::ChooseFolderForNewSong)
    }

    /// A browser that chooses a folder for `purpose`.
    #[must_use]
    pub fn choosing_folder(directory: PathBuf, purpose: BrowserPurpose) -> FolderBrowser {
        FolderBrowser {
            purpose,
            ..FolderBrowser::new(directory)
        }
    }

    #[must_use]
    pub fn purpose(&self) -> BrowserPurpose {
        self.purpose
    }

    /// Ask for a new folder in the folder shown (N, when choosing a folder).
    #[must_use]
    pub fn new_folder_here(&self) -> Option<BrowserRequest> {
        (self.purpose == BrowserPurpose::ChooseFolderForNewSong)
            .then_some(BrowserRequest::NewFolder)
    }

    /// Copy the chosen song folder (C, when choosing a folder).
    #[must_use]
    pub fn copy_selected(&self) -> Option<BrowserRequest> {
        if self.purpose != BrowserPurpose::ChooseFolderForNewSong {
            return None;
        }
        let entry: &BrowserEntry = self.entries.get(self.list.selected())?;
        (entry.kind == BrowserEntryKind::SongFolder)
            .then(|| BrowserRequest::CopySong(self.directory.join(&entry.name)))
    }

    /// The folder being shown is the one chosen (U, when choosing a folder).
    #[must_use]
    pub fn choose_here(&self) -> Option<BrowserRequest> {
        self.purpose
            .chooses_folder()
            .then(|| BrowserRequest::ChooseFolder(self.directory.clone()))
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    #[must_use]
    pub fn entries(&self) -> &[BrowserEntry] {
        &self.entries
    }

    #[must_use]
    pub fn selected(&self) -> usize {
        self.list.selected()
    }

    #[must_use]
    pub fn list(&self) -> &ScrollList {
        &self.list
    }

    /// The first row drawn.
    #[must_use]
    pub fn top(&self) -> usize {
        self.list.top()
    }

    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    /// The folder's contents. Folders come first, then song files, each
    /// group by name without regard to case; the row going up leads when
    /// there is a folder above.
    pub fn set_listing(&mut self, directory: PathBuf, mut found: Vec<BrowserEntry>) {
        found.sort_by_key(|entry: &BrowserEntry| {
            (
                entry.kind == BrowserEntryKind::SongFile,
                entry.name.to_lowercase(),
            )
        });
        let mut entries: Vec<BrowserEntry> = Vec::with_capacity(found.len() + 1);
        if directory.parent().is_some() {
            entries.push(BrowserEntry {
                name: String::from(".."),
                kind: BrowserEntryKind::ParentFolder,
            });
        }
        entries.extend(found);
        self.directory = directory;
        self.list.reset(entries.len());
        self.entries = entries;
        self.problem = None;
    }

    /// The folder could not be listed; the browser stays where it was.
    pub fn set_problem(&mut self, problem: String) {
        self.problem = Some(problem);
    }

    pub fn up(&mut self) {
        self.list.up();
    }

    pub fn down(&mut self) {
        self.list.down();
    }

    pub fn page_up(&mut self) {
        self.list.page_up();
    }

    pub fn page_down(&mut self) {
        self.list.page_down();
    }

    pub fn first(&mut self) {
        self.list.first();
    }

    pub fn last(&mut self) {
        self.list.last();
    }

    pub fn select(&mut self, index: usize) {
        self.list.select(index);
    }

    /// Scrolls the view by `rows` (negative: toward the top) and leaves the
    /// selection where it is, as the mouse wheel does.
    pub fn scroll_by(&mut self, rows: isize) {
        self.list.scroll_by(rows);
    }

    /// One page up or down, as a click on the scroll bar asks.
    pub fn scroll_page(&mut self, up: bool) {
        self.list.scroll_page(up);
    }

    /// Opens the selected row: into a folder, up a folder, or a song.
    #[must_use]
    pub fn enter(&self) -> Option<BrowserRequest> {
        let entry: &BrowserEntry = self.entries.get(self.list.selected())?;
        match entry.kind {
            BrowserEntryKind::ParentFolder => self.parent(),
            BrowserEntryKind::Folder => {
                Some(BrowserRequest::List(self.directory.join(&entry.name)))
            }
            // A folder that holds a song is no place for another.
            BrowserEntryKind::SongFolder | BrowserEntryKind::SongFile
                if self.purpose.chooses_folder() =>
            {
                None
            }
            BrowserEntryKind::SongFolder => Some(BrowserRequest::Open(
                self.directory
                    .join(&entry.name)
                    .join(format!("{}.song", entry.name)),
            )),
            BrowserEntryKind::SongFile => {
                Some(BrowserRequest::Open(self.directory.join(&entry.name)))
            }
        }
    }

    /// Up to the enclosing folder (Backspace), if there is one.
    #[must_use]
    pub fn parent(&self) -> Option<BrowserRequest> {
        self.directory
            .parent()
            .map(|parent: &Path| BrowserRequest::List(parent.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use super::{BrowserEntry, BrowserEntryKind, BrowserRequest, FolderBrowser, VISIBLE_ROWS};
    use std::path::PathBuf;

    fn entry(name: &str, kind: BrowserEntryKind) -> BrowserEntry {
        BrowserEntry {
            name: name.to_string(),
            kind,
        }
    }

    fn browser() -> FolderBrowser {
        let mut browser: FolderBrowser = FolderBrowser::new(PathBuf::from("/music/songs"));
        browser.set_listing(
            PathBuf::from("/music/songs"),
            vec![
                entry("old.song", BrowserEntryKind::SongFile),
                entry("Tempo_marks", BrowserEntryKind::SongFolder),
                entry("archive", BrowserEntryKind::Folder),
            ],
        );
        browser
    }

    #[test]
    fn folders_come_first_then_song_files_and_the_way_up_leads() {
        let browser: FolderBrowser = browser();
        let names: Vec<&str> = browser.entries().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["..", "archive", "Tempo_marks", "old.song"]);
    }

    #[test]
    fn a_song_folder_opens_the_song_of_its_own_name() {
        let mut browser: FolderBrowser = browser();
        browser.select(2);
        assert_eq!(
            browser.enter(),
            Some(BrowserRequest::Open(PathBuf::from(
                "/music/songs/Tempo_marks/Tempo_marks.song"
            )))
        );
    }

    #[test]
    fn a_folder_is_entered_and_the_parent_row_goes_up() {
        let mut browser: FolderBrowser = browser();
        browser.select(1);
        assert_eq!(
            browser.enter(),
            Some(BrowserRequest::List(PathBuf::from("/music/songs/archive")))
        );
        browser.select(0);
        assert_eq!(
            browser.enter(),
            Some(BrowserRequest::List(PathBuf::from("/music")))
        );
    }

    #[test]
    fn a_loose_song_file_opens() {
        let mut browser: FolderBrowser = browser();
        browser.select(3);
        assert_eq!(
            browser.enter(),
            Some(BrowserRequest::Open(PathBuf::from("/music/songs/old.song")))
        );
    }

    #[test]
    fn the_selection_stays_inside_the_list() {
        let mut browser: FolderBrowser = browser();
        browser.up();
        assert_eq!(browser.selected(), 0);
        for _ in 0..10 {
            browser.down();
        }
        assert_eq!(browser.selected(), 3);
        browser.select(9);
        assert_eq!(browser.selected(), 3, "past the end selects the last row");
    }

    fn long_listing() -> FolderBrowser {
        let mut browser: FolderBrowser = FolderBrowser::new(PathBuf::from("/music/songs"));
        let found: Vec<BrowserEntry> = (0..40)
            .map(|number: u32| entry(&format!("folder_{number:02}"), BrowserEntryKind::Folder))
            .collect();
        browser.set_listing(PathBuf::from("/music/songs"), found);
        browser
    }

    #[test]
    fn moving_down_scrolls_only_when_the_selection_leaves_the_view() {
        let mut browser: FolderBrowser = long_listing();
        for _ in 0..(VISIBLE_ROWS - 1) {
            browser.down();
        }
        assert_eq!(browser.top(), 0, "the last visible row needs no scroll");
        browser.down();
        assert_eq!(browser.top(), 1);
    }

    #[test]
    fn moving_back_up_does_not_jump_the_view() {
        let mut browser: FolderBrowser = long_listing();
        for _ in 0..25 {
            browser.down();
        }
        let top_after_down: usize = browser.top();
        browser.up();
        browser.up();
        assert_eq!(browser.top(), top_after_down);
        for _ in 0..30 {
            browser.up();
        }
        assert_eq!(browser.selected(), 0);
        assert_eq!(browser.top(), 0);
    }

    #[test]
    fn the_wheel_scrolls_the_view_and_keeps_the_selection() {
        let mut browser: FolderBrowser = long_listing();
        browser.scroll_by(10);
        assert_eq!(browser.top(), 10);
        assert_eq!(browser.selected(), 0);
        browser.scroll_by(1000);
        assert_eq!(
            browser.top(),
            41 - VISIBLE_ROWS,
            "41 rows: the way up and 40 folders"
        );
        browser.scroll_by(-1000);
        assert_eq!(browser.top(), 0);
    }

    #[test]
    fn page_keys_home_and_end_keep_the_selection_in_view() {
        let mut browser: FolderBrowser = long_listing();
        browser.page_down();
        assert_eq!(browser.selected(), VISIBLE_ROWS);
        browser.last();
        assert_eq!(browser.selected(), 40);
        assert_eq!(browser.top(), 41 - VISIBLE_ROWS);
        browser.page_up();
        assert_eq!(browser.selected(), 40 - VISIBLE_ROWS);
        assert!(browser.selected() >= browser.top());
        browser.first();
        assert_eq!((browser.selected(), browser.top()), (0, 0));
    }

    #[test]
    fn the_root_has_no_way_up() {
        let mut browser: FolderBrowser = FolderBrowser::new(PathBuf::from("/"));
        browser.set_listing(
            PathBuf::from("/"),
            vec![entry("home", BrowserEntryKind::Folder)],
        );
        assert_eq!(browser.entries().len(), 1);
        assert_eq!(browser.parent(), None);
    }
}
