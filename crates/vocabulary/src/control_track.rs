//! Control tracks (D-10): what a song needs from the studio's other
//! devices — phrase cues. (A beat pulse is an ordinary mono voice.)

use std::fmt;

use crate::phrase_cue::PhraseCue;

/// A control track: any number of phrase cues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlTrack {
    name: String,
    cues: Vec<PhraseCue>,
}

impl ControlTrack {
    /// # Errors
    ///
    /// When the name is empty.
    pub fn new(name: &str, cues: Vec<PhraseCue>) -> Result<ControlTrack, ControlTrackError> {
        if name.trim().is_empty() {
            return Err(ControlTrackError::EmptyTrackName);
        }
        Ok(ControlTrack {
            name: name.to_string(),
            cues,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn cues(&self) -> &[PhraseCue] {
        &self.cues
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlTrackError {
    EmptyTrackName,
}

impl fmt::Display for ControlTrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlTrackError::EmptyTrackName => {
                write!(f, "a control track has no name. Give it one, e.g. Studio")
            }
        }
    }
}

impl std::error::Error for ControlTrackError {}
