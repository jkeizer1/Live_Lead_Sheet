//! Why a song could not be written as LilyPond.

use std::fmt;

use note_spelling::spelling_error::SpellingError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// A place in the song that note spelling cannot write yet.
    Spelling(SpellingError),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Spelling(error) => write!(f, "LilyPond view: {error}"),
        }
    }
}

impl std::error::Error for ExportError {}
