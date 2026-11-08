//! Note spelling: from the vocabulary's exact lengths and places (ticks)
//! back to written music, for every converter that writes notation (the
//! the LilyPond view now; MusicXML and the screen later). It does the
//! arithmetic once: which note value and dots an element is, which elements
//! form a triplet, and which written pitches a chord's notes are.
//!
//! V1 spells plain and dotted values and triplets. Anything else, such as
//! five in the time of a quarter, is reported, not guessed.

pub mod chord_pitches;
pub mod note_value;
pub mod spelled_line;
pub mod spelling_error;
