//! The plain-text notation for songs, studio configurations and the two
//! libraries (EFS-6; docs/Architecture/Song Notation.md). An adapter at the
//! edge (A-3): it translates text to and from the vocabulary and uses no
//! external crate.
//!
//! So far: rhythm patterns and the rhythm library file.

pub mod length_words;
pub mod parse_error;
pub mod rhythm_bar;
pub mod rhythm_library;
