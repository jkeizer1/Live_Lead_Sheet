//! What the sending loop saw of its own timing during one play, so a
//! missed or crowded note can be told from a fault in the score: an event
//! handed to the output after its instant is *late*, and a long pause
//! between two looks at the clock is the loop having been held up.

use vocabulary::duration::Duration;

/// Nanoseconds in a millisecond, to show durations in the unit the
/// musician knows.
const NANOSECONDS_PER_MILLISECOND: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayReport {
    /// Events handed to the output.
    pub events_sent: u64,
    /// Events handed over after their instant. For a note-off placed before
    /// the next note-on, the instant is where the output must send it, the
    /// gap earlier than that note-on.
    pub late_events: u64,
    /// The most any event was late.
    pub worst_lateness: Duration,
    /// The longest time between two looks at the clock. The loop looks
    /// every 5 ms; a long pause means the thread was not running.
    pub longest_pause: Duration,
    /// The first notes handed over (port, note, and how long after the play's
    /// first instant each is due), so a note missing from the start of a play
    /// shows here whether the sender ever had it.
    pub first_notes: Vec<String>,
}

/// How many of the first notes the report keeps.
const FIRST_NOTES_KEPT: usize = 6;

impl Default for PlayReport {
    /// Nothing seen yet.
    fn default() -> PlayReport {
        PlayReport {
            events_sent: 0,
            late_events: 0,
            worst_lateness: Duration::ZERO,
            longest_pause: Duration::ZERO,
            first_notes: Vec::new(),
        }
    }
}

impl PlayReport {
    /// One look at the clock, `pause` after the one before.
    pub(crate) fn note_pause(&mut self, pause: Duration) {
        if pause > self.longest_pause {
            self.longest_pause = pause;
        }
    }

    /// A note handed over, described; only the first few are kept.
    pub(crate) fn note_first(&mut self, note: Option<String>) {
        if let Some(note) = note
            && self.first_notes.len() < FIRST_NOTES_KEPT
        {
            self.first_notes.push(note);
        }
    }

    /// One event handed over; `lateness` is how late, when it was.
    pub(crate) fn note_event(&mut self, lateness: Option<Duration>) {
        self.events_sent += 1;
        if let Some(lateness) = lateness {
            self.late_events += 1;
            if lateness > self.worst_lateness {
                self.worst_lateness = lateness;
            }
        }
    }

    /// One sentence for the musician.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Sender: {} events handed over, {} late (worst {} ms); longest pause between looks at the clock {} ms. First notes: {}.",
            self.events_sent,
            self.late_events,
            self.worst_lateness.ns() / NANOSECONDS_PER_MILLISECOND,
            self.longest_pause.ns() / NANOSECONDS_PER_MILLISECOND,
            if self.first_notes.is_empty() {
                String::from("none")
            } else {
                self.first_notes.join(", ")
            },
        )
    }
}
