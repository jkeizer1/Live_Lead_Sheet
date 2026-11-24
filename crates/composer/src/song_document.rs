//! The song being composed: its text, which is what is saved (EFS-6.2),
//! and the song read from it. An edit gives new text; the text is read back
//! by the song file's reader, and only a text that reads cleanly replaces
//! the old one, so the document never holds a song that would not load
//! (EFS-6.3). Each accepted edit can be undone.

use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use vocabulary::song::Song;

use crate::song_source::SongSource;

/// Most edits kept for undo; older ones are let go.
const UNDO_DEPTH: usize = 200;

/// An edit that was refused, with what the reader said about the text it
/// would have made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditRefused {
    pub what: String,
    pub messages: Vec<String>,
}

impl std::fmt::Display for EditRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} refused: {}", self.what, self.messages.join("; "))
    }
}

/// An earlier text, and what the edit that left it did.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    text: String,
    what: String,
}

pub struct SongDocument {
    file_name: String,
    text: String,
    song: Song,
    source: SongSource,
    /// The text as last saved or loaded, to tell whether there are
    /// unsaved edits.
    saved_text: String,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

/// Reads a song's text, giving each mistake as its line and message.
///
/// # Errors
///
/// Every mistake the reader found.
pub fn read_song(file_name: &str, text: &str) -> Result<Song, Vec<String>> {
    parse_song(file_name, text).map_err(|errors: Vec<ParseError>| {
        errors.iter().map(ParseError::line_and_message).collect()
    })
}

impl SongDocument {
    /// # Errors
    ///
    /// Every mistake in the text.
    pub fn new(file_name: &str, text: String) -> Result<SongDocument, Vec<String>> {
        let song: Song = read_song(file_name, &text)?;
        let source: SongSource = SongSource::read(&text);
        Ok(SongDocument {
            file_name: file_name.to_string(),
            saved_text: text.clone(),
            text,
            song,
            source,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn song(&self) -> &Song {
        &self.song
    }

    #[must_use]
    pub fn source(&self) -> &SongSource {
        &self.source
    }

    #[must_use]
    pub fn has_unsaved_edits(&self) -> bool {
        self.text != self.saved_text
    }

    /// After the text has been written to its file.
    pub fn mark_saved(&mut self) {
        self.saved_text = self.text.clone();
    }

    /// Replaces the text with `new_text`, when it reads cleanly.
    ///
    /// # Errors
    ///
    /// When the reader finds a mistake in the new text; the document is
    /// then unchanged.
    pub fn replace(&mut self, new_text: String, what: &str) -> Result<(), EditRefused> {
        if new_text == self.text {
            return Ok(());
        }
        let song: Song = read_song(&self.file_name, &new_text).map_err(|messages| EditRefused {
            what: what.to_string(),
            messages,
        })?;
        let old_text: String = std::mem::replace(&mut self.text, new_text);
        self.undo.push(Snapshot {
            text: old_text,
            what: what.to_string(),
        });
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.song = song;
        self.source = SongSource::read(&self.text);
        Ok(())
    }

    /// The file changed outside the editor: take its text as the new
    /// starting point, which an undo can still go back from.
    ///
    /// # Errors
    ///
    /// When the new text has mistakes; the document keeps the old text.
    pub fn reload(&mut self, new_text: String) -> Result<(), EditRefused> {
        self.replace(new_text, "Reload from the file")?;
        self.mark_saved();
        Ok(())
    }

    /// Goes back one edit; says which edit was undone.
    pub fn undo(&mut self) -> Option<String> {
        let snapshot: Snapshot = self.undo.pop()?;
        Some(self.restore(snapshot, true))
    }

    /// Does again the last edit undone.
    pub fn redo(&mut self) -> Option<String> {
        let snapshot: Snapshot = self.redo.pop()?;
        Some(self.restore(snapshot, false))
    }

    fn restore(&mut self, snapshot: Snapshot, undoing: bool) -> String {
        // Every text on the stacks was read cleanly when it was current, so
        // it reads again; a failure here would be a bug, reported as such.
        let song: Song = match read_song(&self.file_name, &snapshot.text) {
            Ok(song) => song,
            Err(messages) => return format!("Cannot go back: {}", messages.join("; ")),
        };
        let current: Snapshot = Snapshot {
            text: std::mem::replace(&mut self.text, snapshot.text),
            what: snapshot.what.clone(),
        };
        if undoing {
            self.redo.push(current);
        } else {
            self.undo.push(current);
        }
        self.song = song;
        self.source = SongSource::read(&self.text);
        snapshot.what
    }
}
