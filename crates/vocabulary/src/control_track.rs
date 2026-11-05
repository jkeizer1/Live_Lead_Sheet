//! Control tracks (D-10): what a song needs from the studio's other
//! devices — the beat pulse and phrase cues.

use std::fmt;

use crate::midi_values::{MidiChannel, NoteNumber, Velocity};
use crate::phrase_cue::{OutputPortName, PhraseCue};
use crate::rhythm_pattern::RhythmPattern;
use crate::score_length::ScoreLength;

/// One note per count (EFS-2.6), the first beat of each bar at the accent
/// velocity. Without a rhythm it pulses every count; with one, it pulses on
/// each note of the rhythm (D-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatPulse {
    port: OutputPortName,
    channel: MidiChannel,
    note: NoteNumber,
    accent_velocity: Velocity,
    normal_velocity: Velocity,
    /// How long each pulse note is held: the gate length an FH-2 output
    /// gives it.
    length: ScoreLength,
    /// The rhythm to pulse; `None`: every count.
    rhythm: Option<RhythmPattern>,
}

impl BeatPulse {
    /// # Errors
    ///
    /// When `length` is zero.
    pub fn new(
        port: OutputPortName,
        channel: MidiChannel,
        note: NoteNumber,
        accent_velocity: Velocity,
        normal_velocity: Velocity,
        length: ScoreLength,
    ) -> Result<BeatPulse, ControlTrackError> {
        if length == ScoreLength::ZERO {
            return Err(ControlTrackError::BeatPulseWithoutLength);
        }
        Ok(BeatPulse {
            port,
            channel,
            note,
            accent_velocity,
            normal_velocity,
            length,
            rhythm: None,
        })
    }

    /// The same pulse, on the notes of `rhythm` instead of every count.
    #[must_use]
    pub fn with_rhythm(self, rhythm: RhythmPattern) -> BeatPulse {
        BeatPulse {
            rhythm: Some(rhythm),
            ..self
        }
    }

    #[must_use]
    pub fn rhythm(&self) -> Option<&RhythmPattern> {
        self.rhythm.as_ref()
    }

    #[must_use]
    pub fn port(&self) -> &OutputPortName {
        &self.port
    }

    #[must_use]
    pub fn channel(&self) -> MidiChannel {
        self.channel
    }

    #[must_use]
    pub fn note(&self) -> NoteNumber {
        self.note
    }

    #[must_use]
    pub fn accent_velocity(&self) -> Velocity {
        self.accent_velocity
    }

    #[must_use]
    pub fn normal_velocity(&self) -> Velocity {
        self.normal_velocity
    }

    #[must_use]
    pub fn length(&self) -> ScoreLength {
        self.length
    }
}

/// A control track: an optional beat pulse and any number of phrase cues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlTrack {
    name: String,
    beat_pulse: Option<BeatPulse>,
    cues: Vec<PhraseCue>,
}

impl ControlTrack {
    /// # Errors
    ///
    /// When the name is empty.
    pub fn new(
        name: &str,
        beat_pulse: Option<BeatPulse>,
        cues: Vec<PhraseCue>,
    ) -> Result<ControlTrack, ControlTrackError> {
        if name.trim().is_empty() {
            return Err(ControlTrackError::EmptyTrackName);
        }
        Ok(ControlTrack {
            name: name.to_string(),
            beat_pulse,
            cues,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn beat_pulse(&self) -> Option<&BeatPulse> {
        self.beat_pulse.as_ref()
    }

    #[must_use]
    pub fn cues(&self) -> &[PhraseCue] {
        &self.cues
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlTrackError {
    EmptyTrackName,
    BeatPulseWithoutLength,
}

impl fmt::Display for ControlTrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlTrackError::EmptyTrackName => {
                write!(f, "a control track has no name. Give it one, e.g. Studio")
            }
            ControlTrackError::BeatPulseWithoutLength => write!(
                f,
                "the beat pulse has length 0. Give it a length, e.g. a 16th"
            ),
        }
    }
}

impl std::error::Error for ControlTrackError {}
