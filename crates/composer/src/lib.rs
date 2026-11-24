//! Song authoring (Architecture, "Capabilities"): the song being composed,
//! browsing the libraries, applying progressions and rhythms, moving notes
//! and chords by intervals, and arranging the composition. No window, no
//! files and no MIDI here; the app hands it keys and text, and the render
//! crate draws what it holds.

pub mod apply;
pub mod chord_moves;
pub mod chord_symbol;
pub mod edits;
pub mod key_signature;
pub mod library;
pub mod line_bars;
pub mod line_text;
pub mod pitch_moves;
pub mod session;
pub mod session_music;
pub mod song_document;
pub mod song_source;
pub mod timeline;
