//! Notes for one track: the notes a voice plays on one port and channel,
//! with their places, gate lengths and velocities settled.
//!
//! Every voice is the same to the schedule: a note-on with a velocity at the
//! start of an element and a note-off at its end. What the receiver does
//! with them (a gate and a pitch CV on the FH-2, a velocity gate on the
//! Disting NT) is the receiver's business. A click or a trigger for a clock
//! divider is a mono voice that plays one fixed note (Song Notation,
//! section 5).

use std::fmt;

use vocabulary::duration::Duration;
use vocabulary::midi_values::{MidiChannel, NoteNumber, Velocity};
use vocabulary::phrase_cue::OutputPortName;
use vocabulary::rhythm_pattern::{ElementKind, RhythmPattern};
use vocabulary::score_length::ScoreLength;

use crate::studio_configuration::VelocityLevels;

/// A staccato note sounds for this fraction of its written length (Song
/// Notation, section 6): 1 of 2.
const STACCATO_DIVISOR: u64 = 2;

/// One note to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedNote {
    pub start_ticks: u64,
    /// How long the note is held: the gate length at a CV and gate output.
    pub length_ticks: u64,
    pub note: NoteNumber,
    pub velocity: Velocity,
    /// True when another note of the track starts where this one ends and
    /// this note's note-off is sent after that note-on, so a mono receiver
    /// set to legato keeps its gate high (Song Notation, section 3, "How
    /// legato works in MIDI"). False: the note ends as written, and a note
    /// touching the next one is shortened (Q11).
    pub legato_to_next: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTrack {
    track_name: String,
    port: OutputPortName,
    channel: MidiChannel,
    notes: Vec<TimedNote>,
    /// The studio's `retrigger_gap` for this track's output.
    retrigger_gap: Duration,
}

impl NoteTrack {
    /// # Errors
    ///
    /// When the name is empty or a note has no length.
    pub fn new(
        track_name: &str,
        port: OutputPortName,
        channel: MidiChannel,
        notes: Vec<TimedNote>,
    ) -> Result<NoteTrack, NoteTrackError> {
        if track_name.trim().is_empty() {
            return Err(NoteTrackError::EmptyTrackName);
        }
        if let Some(note) = notes
            .iter()
            .find(|note: &&TimedNote| note.length_ticks == 0)
        {
            return Err(NoteTrackError::NoteWithoutLength {
                track_name: track_name.to_string(),
                start_ticks: note.start_ticks,
            });
        }
        Ok(NoteTrack {
            track_name: track_name.to_string(),
            port,
            channel,
            notes,
            // Zero is harmless: the ALSA output's own lead floor still applies.
            retrigger_gap: Duration::ZERO,
        })
    }

    /// The track with the studio's `retrigger_gap` for its output: a stopgap
    /// that raises the lead of a touching note-off above the ALSA output's
    /// floor.
    #[must_use]
    pub fn with_retrigger_gap(self, retrigger_gap: Duration) -> NoteTrack {
        NoteTrack {
            retrigger_gap,
            ..self
        }
    }

    #[must_use]
    pub fn retrigger_gap(&self) -> Duration {
        self.retrigger_gap
    }

    /// A mono voice playing one fixed `note` on the notes of `rhythm`,
    /// which repeats from its first bar to fill the stretch and is cut where
    /// the stretch ends (EFS-5.4).
    ///
    /// A note begins at a sounding element that no tie leads into and lasts
    /// the written length of that element and those tied after it, or half
    /// of that when the element is staccato. Its velocity is the level of
    /// the element's accent mark.
    ///
    /// # Errors
    ///
    /// When the name is empty, a note's accent level has no velocity in
    /// `levels`, or the stretch is longer than can be counted.
    pub fn from_rhythm(
        track_name: &str,
        port: OutputPortName,
        channel: MidiChannel,
        note: NoteNumber,
        levels: VelocityLevels,
        rhythm: &RhythmPattern,
        stretch_length: ScoreLength,
    ) -> Result<NoteTrack, NoteTrackError> {
        // bars × ticks/bar → ticks. At most 8 bars.
        let pattern_length_ticks: u64 =
            u64::from(rhythm.bar_count()) * rhythm.time_signature().bar_length().ticks();
        let elements = rhythm.elements();
        let mut notes: Vec<TimedNote> = Vec::new();
        let mut pattern_start_ticks: u64 = 0;
        while pattern_start_ticks < stretch_length.ticks() {
            for (index, element) in elements.iter().enumerate() {
                let continues_a_tie: bool = index > 0 && elements[index - 1].tied_to_next();
                if element.kind() != ElementKind::Sounding || continues_a_tie {
                    continue;
                }
                // Within the stretch plus one pattern, so no overflow.
                let start_ticks: u64 = pattern_start_ticks + element.start().ticks();
                if start_ticks >= stretch_length.ticks() {
                    break;
                }
                // The written length: this element and those tied after it.
                let mut written_ticks: u64 = element.length().ticks();
                let mut last: usize = index;
                while elements[last].tied_to_next() {
                    let Some(next) = elements.get(last + 1) else {
                        break;
                    };
                    written_ticks += next.length().ticks();
                    last += 1;
                }
                let sounding_ticks: u64 = if element.marks().staccato() {
                    (written_ticks / STACCATO_DIVISOR).max(1)
                } else {
                    written_ticks
                };
                let velocity: Velocity = levels.for_level(element.marks().accent());
                notes.push(TimedNote {
                    start_ticks,
                    length_ticks: sounding_ticks.min(stretch_length.ticks() - start_ticks),
                    note,
                    velocity,
                    legato_to_next: false,
                });
            }
            pattern_start_ticks = pattern_start_ticks
                .checked_add(pattern_length_ticks)
                .ok_or(NoteTrackError::StretchTooLong)?;
        }
        NoteTrack::new(track_name, port, channel, notes)
    }

    #[must_use]
    pub fn track_name(&self) -> &str {
        &self.track_name
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
    pub fn notes(&self) -> &[TimedNote] {
        &self.notes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteTrackError {
    EmptyTrackName,
    NoteWithoutLength {
        track_name: String,
        start_ticks: u64,
    },
    StretchTooLong,
}

impl fmt::Display for NoteTrackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteTrackError::EmptyTrackName => {
                write!(f, "a track has no name. Give it one, e.g. Beat")
            }
            NoteTrackError::NoteWithoutLength {
                track_name,
                start_ticks,
            } => write!(
                f,
                "track {track_name:?}: the note at {start_ticks} ticks has no length"
            ),
            NoteTrackError::StretchTooLong => {
                write!(
                    f,
                    "the stretch is longer than score positions can be counted"
                )
            }
        }
    }
}

impl std::error::Error for NoteTrackError {}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::rhythm_pattern::{AccentLevel, ElementMarks, RhythmElement};
    use vocabulary::time_signature::TimeSignature;

    fn four_four() -> TimeSignature {
        TimeSignature::new(4, 4).unwrap()
    }

    fn levels() -> VelocityLevels {
        VelocityLevels::new(
            Velocity::new(64).unwrap(),
            Some(Velocity::new(127).unwrap()),
            None,
        )
    }

    fn element(
        start: u64,
        length: u64,
        kind: ElementKind,
        marks: ElementMarks,
        tied: bool,
    ) -> RhythmElement {
        RhythmElement::new(
            ScoreLength::from_ticks(start),
            ScoreLength::from_ticks(length),
            kind,
            marks,
            tied,
        )
        .unwrap()
    }

    fn track_of(rhythm: &RhythmPattern, stretch_ticks: u64) -> Result<NoteTrack, NoteTrackError> {
        NoteTrack::from_rhythm(
            "Beat",
            OutputPortName::new("Beat").unwrap(),
            MidiChannel::new(15).unwrap(),
            NoteNumber::new(60).unwrap(),
            levels(),
            rhythm,
            ScoreLength::from_ticks(stretch_ticks),
        )
    }

    fn starts_lengths_velocities(track: &NoteTrack) -> Vec<(u64, u64, u8)> {
        track
            .notes()
            .iter()
            .map(|note: &TimedNote| (note.start_ticks, note.length_ticks, note.velocity.value()))
            .collect()
    }

    #[test]
    fn a_rhythm_repeats_to_fill_the_stretch_and_its_marks_set_the_velocity() {
        // Charleston with an accent on beat 1: dotted quarter, eighth, half rest.
        let accent: ElementMarks = ElementMarks::new(AccentLevel::Accent, false, false, false);
        let charleston: RhythmPattern = RhythmPattern::new(
            "Charleston",
            four_four(),
            1,
            vec![
                element(0, 7560, ElementKind::Sounding, accent, false),
                element(7560, 2520, ElementKind::Sounding, ElementMarks::NONE, false),
                element(10_080, 10_080, ElementKind::Rest, ElementMarks::NONE, false),
            ],
        )
        .unwrap();
        let track: NoteTrack = track_of(&charleston, 40_320).unwrap();
        // The written lengths are the gate lengths.
        assert_eq!(
            starts_lengths_velocities(&track),
            vec![
                (0, 7560, 127),
                (7560, 2520, 64),
                (20_160, 7560, 127),
                (27_720, 2520, 64)
            ]
        );
    }

    #[test]
    fn tied_elements_make_one_note_as_long_as_both() {
        // Anticipation: an eighth tied across the barline into a quarter.
        let anticipation: RhythmPattern = RhythmPattern::new(
            "Anticipation",
            four_four(),
            2,
            vec![
                element(0, 17_640, ElementKind::Rest, ElementMarks::NONE, false),
                element(
                    17_640,
                    2520,
                    ElementKind::Sounding,
                    ElementMarks::NONE,
                    true,
                ),
                element(
                    20_160,
                    5040,
                    ElementKind::Sounding,
                    ElementMarks::NONE,
                    false,
                ),
                element(25_200, 15_120, ElementKind::Rest, ElementMarks::NONE, false),
            ],
        )
        .unwrap();
        let track: NoteTrack = track_of(&anticipation, 40_320).unwrap();
        // 2520 ticks + 5040 ticks = 7560 ticks, one note.
        assert_eq!(starts_lengths_velocities(&track), vec![(17_640, 7560, 64)]);
    }

    #[test]
    fn staccato_sounds_for_half_the_written_length_and_the_last_note_is_cut_at_the_end() {
        let staccato: ElementMarks = ElementMarks::new(AccentLevel::Normal, true, false, false);
        let notes: RhythmPattern = RhythmPattern::new(
            "Short",
            four_four(),
            1,
            vec![
                element(0, 10_080, ElementKind::Sounding, staccato, false),
                element(
                    10_080,
                    10_080,
                    ElementKind::Sounding,
                    ElementMarks::NONE,
                    false,
                ),
            ],
        )
        .unwrap();
        // A stretch of 15000 ticks cuts the second note at 15000 - 10080 = 4920 ticks.
        let track: NoteTrack = track_of(&notes, 15_000).unwrap();
        assert_eq!(
            starts_lengths_velocities(&track),
            vec![(0, 5040, 64), (10_080, 4920, 64)]
        );
    }

    #[test]
    fn a_mark_without_a_studio_velocity_is_relative_to_normal() {
        // levels(): normal 64, accent 127 (absolute), no marcato. Marcato is
        // 3/4 of the way from 64 to 127 (111), and the soft marks are 3/8 and
        // 3/4 of the way from 64 down to 1 (40 and 17).
        let marked = |level: AccentLevel| ElementMarks::new(level, false, false, false);
        let rhythm: RhythmPattern = RhythmPattern::new(
            "Levels",
            four_four(),
            1,
            vec![
                element(
                    0,
                    5040,
                    ElementKind::Sounding,
                    marked(AccentLevel::Accent),
                    false,
                ),
                element(
                    5040,
                    5040,
                    ElementKind::Sounding,
                    marked(AccentLevel::Marcato),
                    false,
                ),
                element(
                    10_080,
                    5040,
                    ElementKind::Sounding,
                    marked(AccentLevel::Soft),
                    false,
                ),
                element(
                    15_120,
                    5040,
                    ElementKind::Sounding,
                    marked(AccentLevel::Softer),
                    false,
                ),
            ],
        )
        .unwrap();
        let track: NoteTrack = track_of(&rhythm, 20_160).unwrap();
        assert_eq!(
            starts_lengths_velocities(&track),
            vec![
                (0, 5040, 127),
                (5040, 5040, 111),
                (10_080, 5040, 40),
                (15_120, 5040, 17),
            ]
        );
    }
}
