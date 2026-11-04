//! Phrase cues (D-10, EFS-4): markers at score positions that send one
//! plain MIDI message to the studio — a note (a short one is a trigger, a
//! held one a gate), a control change, a program change, or, on a control
//! track only, Start, Stop or Continue.

use std::fmt;

use crate::midi_values::{
    ControllerNumber, ControllerValue, MidiChannel, NoteNumber, ProgramNumber, Velocity,
};
use crate::score_length::ScoreLength;
use crate::score_position::ScorePosition;

/// The name of one of the program's output ports (EFS-3.1), e.g. `Clock`
/// or `SQ-1`. Which devices hang off it is the studio's connection, not
/// the song's.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputPortName {
    name: String,
}

impl OutputPortName {
    /// # Errors
    ///
    /// When the name is empty or only spaces.
    pub fn new(name: &str) -> Result<OutputPortName, PhraseCueError> {
        if name.trim().is_empty() {
            return Err(PhraseCueError::EmptyPortName);
        }
        Ok(OutputPortName {
            name: name.to_string(),
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

/// MIDI System Real-Time transport. These have no channel: they reach
/// every device connected to the port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportMessage {
    Start,
    Stop,
    Continue,
}

/// A message for one channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelMessage {
    /// Note on at the cue, note off `length` later.
    Note {
        note: NoteNumber,
        velocity: Velocity,
        length: ScoreLength,
    },
    ControlChange {
        controller: ControllerNumber,
        value: ControllerValue,
    },
    ProgramChange {
        program: ProgramNumber,
    },
}

/// What a cue on a control track sends: a channel message, or transport.
/// Transport carries no channel, so a "Stop on channel 3" cannot be
/// written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlCueMessage {
    Channel {
        channel: MidiChannel,
        message: ChannelMessage,
    },
    Transport(TransportMessage),
}

/// A cue on a control track: it names its own output port (D-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhraseCue {
    name: String,
    position: ScorePosition,
    port: OutputPortName,
    message: ControlCueMessage,
}

impl PhraseCue {
    /// # Errors
    ///
    /// When the name is empty, or a note's length is zero (its note off
    /// would arrive with its note on, and many devices would miss it).
    pub fn new(
        name: &str,
        position: ScorePosition,
        port: OutputPortName,
        message: ControlCueMessage,
    ) -> Result<PhraseCue, PhraseCueError> {
        if name.trim().is_empty() {
            return Err(PhraseCueError::EmptyCueName);
        }
        let note_length: Option<ScoreLength> = match message {
            ControlCueMessage::Channel {
                message: ChannelMessage::Note { length, .. },
                ..
            } => Some(length),
            ControlCueMessage::Channel {
                message: ChannelMessage::ControlChange { .. } | ChannelMessage::ProgramChange { .. },
                ..
            }
            | ControlCueMessage::Transport(_) => None,
        };
        if note_length == Some(ScoreLength::ZERO) {
            return Err(PhraseCueError::NoteWithoutLength {
                cue_name: name.to_string(),
            });
        }
        Ok(PhraseCue {
            name: name.to_string(),
            position,
            port,
            message,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn position(&self) -> ScorePosition {
        self.position
    }

    #[must_use]
    pub fn port(&self) -> &OutputPortName {
        &self.port
    }

    #[must_use]
    pub fn message(&self) -> ControlCueMessage {
        self.message
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhraseCueError {
    EmptyPortName,
    EmptyCueName,
    NoteWithoutLength { cue_name: String },
}

impl fmt::Display for PhraseCueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhraseCueError::EmptyPortName => {
                write!(f, "an output port has no name. Give it one, e.g. SQ-1")
            }
            PhraseCueError::EmptyCueName => {
                write!(f, "a cue has no name. Give it one, e.g. Start SQ-1")
            }
            PhraseCueError::NoteWithoutLength { cue_name } => write!(
                f,
                "cue {cue_name:?} sends a note of length 0. Give it a length, e.g. a 32nd"
            ),
        }
    }
}

impl std::error::Error for PhraseCueError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn port() -> OutputPortName {
        OutputPortName::new("SQ-1").unwrap()
    }

    #[test]
    fn transport_cue_is_made() {
        let cue: PhraseCue = PhraseCue::new(
            "Start SQ-1",
            ScorePosition::from_ticks(20_160),
            port(),
            ControlCueMessage::Transport(TransportMessage::Start),
        )
        .unwrap();
        assert_eq!(cue.position().ticks(), 20_160);
        assert_eq!(cue.port().as_str(), "SQ-1");
    }

    #[test]
    fn note_cue_of_length_zero_is_refused() {
        let message: ControlCueMessage = ControlCueMessage::Channel {
            channel: MidiChannel::new(16).unwrap(),
            message: ChannelMessage::Note {
                note: NoteNumber::new(60).unwrap(),
                velocity: Velocity::new(127).unwrap(),
                length: ScoreLength::ZERO,
            },
        };
        assert_eq!(
            PhraseCue::new("Reset", ScorePosition::ZERO, port(), message),
            Err(PhraseCueError::NoteWithoutLength {
                cue_name: "Reset".to_string()
            })
        );
    }

    #[test]
    fn empty_names_are_refused() {
        assert_eq!(OutputPortName::new(" "), Err(PhraseCueError::EmptyPortName));
        assert_eq!(
            PhraseCue::new(
                "",
                ScorePosition::ZERO,
                port(),
                ControlCueMessage::Transport(TransportMessage::Stop)
            ),
            Err(PhraseCueError::EmptyCueName)
        );
    }
}
