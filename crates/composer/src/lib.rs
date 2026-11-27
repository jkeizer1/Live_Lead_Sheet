//! Song authoring (Architecture, "Capabilities"): the song being composed,
//! browsing the libraries, applying progressions and rhythms, moving notes
//! and chords by intervals, and arranging the composition. No window, no
//! files and no MIDI here; the app hands it keys and text, and the render
//! crate draws what it holds.

pub mod apply;
pub mod arrange_editor;
pub mod arrange_writer;
pub mod arrangement;
pub mod chord_moves;
pub mod chord_symbol;
pub mod composition_edits;
pub mod edits;
pub mod folder_browser;
pub mod key_signature;
pub mod library;
pub mod line_bars;
pub mod line_text;
pub mod new_song;
pub mod pitch_moves;
pub mod repair;
pub mod scroll_list;
pub mod session;
pub mod session_music;
pub mod settings_view;
pub mod shell;
pub mod song_document;
pub mod song_settings;
pub mod song_source;
pub mod studio_settings;
pub mod text_field;
pub mod timeline;
