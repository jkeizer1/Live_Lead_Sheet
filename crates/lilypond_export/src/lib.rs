//! The LilyPond view of a song: an adapter at the edge (A-3) that writes a
//! song as a LilyPond text file, which the `lilypond` program engraves as
//! sheet music (PDF). For reading songs as music while the screen is being
//! built; it sends nothing and changes nothing.
//!
//! What the notes are (values, triplets, chord notes) comes from
//! `note_spelling`; this crate only writes LilyPond's words for them.

pub mod export_error;
pub mod lilypond_words;
pub mod score;
