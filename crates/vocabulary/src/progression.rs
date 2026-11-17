//! A library progression (Song Notation, section 7): the chords of a
//! sequence of bars in Roman numerals. It has no key and carries no rhythm;
//! the chords of a bar share the bar evenly.

use std::fmt;

use crate::key::Mode;
use crate::roman_chord::RomanChord;
use crate::time_signature::TimeSignature;

/// A named progression with the place it is listed under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progression {
    name: String,
    idiom: String,
    kind: String,
    mode: Mode,
    time_signature: TimeSignature,
    /// The chords of each bar, in order; never empty, and no bar is empty.
    bars: Vec<Vec<RomanChord>>,
}

impl Progression {
    /// # Errors
    ///
    /// When the name, idiom or kind is empty, there are no bars, or a bar
    /// has no chord.
    pub fn new(
        name: &str,
        idiom: &str,
        kind: &str,
        mode: Mode,
        time_signature: TimeSignature,
        bars: Vec<Vec<RomanChord>>,
    ) -> Result<Progression, ProgressionError> {
        for (what, text) in [("name", name), ("idiom", idiom), ("kind", kind)] {
            if text.trim().is_empty() {
                return Err(ProgressionError::EmptyText { what });
            }
        }
        if bars.is_empty() {
            return Err(ProgressionError::NoBars);
        }
        if let Some(index) = bars.iter().position(Vec::is_empty) {
            return Err(ProgressionError::EmptyBar {
                bar_number: index + 1,
            });
        }
        Ok(Progression {
            name: name.to_string(),
            idiom: idiom.to_string(),
            kind: kind.to_string(),
            mode,
            time_signature,
            bars,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The first group name, e.g. "Blues".
    #[must_use]
    pub fn idiom(&self) -> &str {
        &self.idiom
    }

    /// The second group name, e.g. "Full form".
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Whether the numerals are written for a major or a minor key. A
    /// label for browsing and warnings; it does not change what plays.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    #[must_use]
    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    #[must_use]
    pub fn bars(&self) -> &[Vec<RomanChord>] {
        &self.bars
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressionError {
    EmptyText { what: &'static str },
    NoBars,
    EmptyBar { bar_number: usize },
}

impl fmt::Display for ProgressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProgressionError::EmptyText { what } => {
                write!(f, "a progression's {what} cannot be empty")
            }
            ProgressionError::NoBars => write!(f, "a progression needs at least one bar"),
            ProgressionError::EmptyBar { bar_number } => {
                write!(
                    f,
                    "bar {bar_number} has no chord. Write at least one chord in it"
                )
            }
        }
    }
}

impl std::error::Error for ProgressionError {}
