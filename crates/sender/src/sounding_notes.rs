//! The notes the sending loop has started and not yet seen end: step 2 of
//! Stop sends each of them a Note Off directly (Musical Time and Clock,
//! section 8), because removing the queued events also removes their own
//! Note Offs.
//!
//! A note counts as sounding from the moment its Note On is handed over
//! until its Note Off is due, even if the Note On itself is still waiting
//! in the queue: a Note Off for a note that is not sounding is harmless,
//! a note left sounding is not.

use vocabulary::instant::Instant;
use vocabulary::midi_values::{MidiChannel, NoteNumber};
use vocabulary::phrase_cue::OutputPortName;

/// One note on one port and channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SoundingNote {
    pub(crate) port: OutputPortName,
    pub(crate) channel: MidiChannel,
    pub(crate) note: NoteNumber,
    /// When its Note Off is due, once that has been handed over.
    note_off_due: Option<Instant>,
}

#[derive(Debug, Default)]
pub(crate) struct SoundingNotes {
    notes: Vec<SoundingNote>,
}

impl SoundingNotes {
    /// A Note On was handed over.
    pub(crate) fn started(
        &mut self,
        port: &OutputPortName,
        channel: MidiChannel,
        note: NoteNumber,
    ) {
        match self.find(port, channel, note) {
            // The same note again before the earlier one ended: one Note Off
            // ends both at the receiver, so one entry is enough.
            Some(index) => self.notes[index].note_off_due = None,
            None => self.notes.push(SoundingNote {
                port: port.clone(),
                channel,
                note,
                note_off_due: None,
            }),
        }
    }

    /// The note's Note Off was handed over, due at `due`. A Note Off for a
    /// note not started in this play (held across the start) has no entry.
    pub(crate) fn ending(
        &mut self,
        port: &OutputPortName,
        channel: MidiChannel,
        note: NoteNumber,
        due: Instant,
    ) {
        if let Some(index) = self.find(port, channel, note) {
            self.notes[index].note_off_due = Some(due);
        }
    }

    /// Drops the notes whose Note Off is due by `now`: the kernel has
    /// delivered it.
    pub(crate) fn forget_ended(&mut self, now: Instant) {
        self.notes
            .retain(|sounding: &SoundingNote| sounding.note_off_due.is_none_or(|due| due > now));
    }

    /// The notes still sounding at `now`, and forgets them all.
    pub(crate) fn take_sounding_at(&mut self, now: Instant) -> Vec<SoundingNote> {
        self.forget_ended(now);
        std::mem::take(&mut self.notes)
    }

    fn find(&self, port: &OutputPortName, channel: MidiChannel, note: NoteNumber) -> Option<usize> {
        self.notes.iter().position(|sounding: &SoundingNote| {
            sounding.port == *port && sounding.channel == channel && sounding.note == note
        })
    }
}
