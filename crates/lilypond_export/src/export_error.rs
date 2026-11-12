//! Why a song could not be written as LilyPond.

use std::fmt;

use note_spelling::spelling_error::SpellingError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// A place in the song that note spelling cannot write yet.
    Spelling(SpellingError),
    /// A song with a `sample_source` line: how to draw one is not designed.
    SampleSourceNotDrawn { track_name: String },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Spelling(error) => write!(f, "LilyPond view: {error}"),
            ExportError::SampleSourceNotDrawn { track_name } => write!(
                f,
                "LilyPond view: track \"{track_name}\" has a sample_source line, which is not \
                 drawn yet. Leave it out, or write the track as plain notes"
            ),
        }
    }
}

impl std::error::Error for ExportError {}
