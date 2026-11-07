//! Failures of the ALSA output. Each says what was being done and with
//! what input, not only which call failed.

use std::fmt;

#[derive(Debug)]
pub enum AlsaOutputError {
    /// An ALSA call failed while doing `doing`.
    Alsa { doing: String, source: alsa::Error },
    /// A name given to ALSA contains a NUL character.
    NameHasNul { what: &'static str, name: String },
    /// The queue's real time does not fit the instant's 64 bits.
    ClockOutOfRange { seconds: u64, nanoseconds: u32 },
    /// A MIDI value is outside what a MIDI message can carry.
    MidiValueOutOfRange {
        what: &'static str,
        value: u8,
        largest: u8,
    },
    /// An instant cannot be turned into a queue timestamp.
    InstantOutOfRange { instant_ns: u64 },
    /// Our own output buffer is full: events were queued while the kernel's
    /// output pool stayed full.
    OutputBufferFull { doing: String },
    /// A note that touches the next one is not longer than the lead its
    /// note-off needs.
    NoteNotLongerThanLead {
        channel: u8,
        note: u8,
        note_length_ns: u64,
        lead_ns: u64,
    },
}

impl AlsaOutputError {
    pub(crate) fn alsa(doing: impl Into<String>, source: alsa::Error) -> AlsaOutputError {
        AlsaOutputError::Alsa {
            doing: doing.into(),
            source,
        }
    }
}

impl fmt::Display for AlsaOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlsaOutputError::Alsa { doing, source } => {
                write!(f, "MIDI output failed while {doing}: {source}")
            }
            AlsaOutputError::NameHasNul { what, name } => {
                write!(f, "the {what} {name:?} contains a NUL character")
            }
            AlsaOutputError::ClockOutOfRange {
                seconds,
                nanoseconds,
            } => write!(
                f,
                "reading the queue's real time: {seconds} s {nanoseconds} ns \
                 does not fit in 64 bits of nanoseconds"
            ),
            AlsaOutputError::MidiValueOutOfRange {
                what,
                value,
                largest,
            } => write!(
                f,
                "MIDI {what} is {value}; the largest allowed is {largest}"
            ),
            AlsaOutputError::OutputBufferFull { doing } => write!(
                f,
                "MIDI output full while {doing}: events were queued faster than the kernel \
                 delivers them. Lower the tempo"
            ),
            AlsaOutputError::NoteNotLongerThanLead {
                channel,
                note,
                note_length_ns,
                lead_ns,
            } => write!(
                f,
                "note {note} on channel {channel} lasts {note_length_ns} ns, which is not longer \
                 than the {lead_ns} ns its note-off must come before the next note. Use a longer \
                 note or a slower tempo"
            ),
            AlsaOutputError::InstantOutOfRange { instant_ns } => write!(
                f,
                "stamping an event at instant {instant_ns} ns: not a valid queue time"
            ),
        }
    }
}

impl std::error::Error for AlsaOutputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AlsaOutputError::Alsa { source, .. } => Some(source),
            _ => None,
        }
    }
}
