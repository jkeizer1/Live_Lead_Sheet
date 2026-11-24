//! Apply (Song Notation, section 7): a rhythm laid over bars, with the
//! harmony in effect at each element. The rhythm may be a library pattern
//! or the rhythm a line already has, and the harmony a library progression
//! or the chords a line already has, so either can be changed while the
//! other is kept: the composer swaps the groove under a progression, or the
//! progression over a groove.
//!
//! The rules, as the notation states them:
//! 1. The bar's first chord goes on the bar's first sounding element, and
//!    the later elements carry it on; the rhythm's marks are copied.
//! 2. Each later sounding element takes the chord in effect when it starts;
//!    a tie keeps the chord of the element it continues.
//! 3. A bass line takes the root of the chord in effect, in the track's
//!    home octave.
//! 4. A bar with more chords than sounding elements is reported.

use std::fmt;

use note_spelling::chord_pitches::{chord_pitches, inverted_chord_pitches};
use vocabulary::bar_meters::BarMeters;
use vocabulary::key::Key;
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::pitch::Pitch;
use vocabulary::rhythm_pattern::{ElementKind, ElementMarks, RhythmPattern};
use vocabulary::song::ElementContents;
use vocabulary::time_signature::TimeSignature;

use crate::line_bars::BarHarmony;
use crate::line_text::{WrittenBar, WrittenElement, WrittenSound};

/// One element of a rhythm, placed in its bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RhythmSlot {
    pub offset_ticks: u64,
    pub length_ticks: u64,
    pub sounding: bool,
    pub marks: ElementMarks,
    pub tied_to_next: bool,
}

/// A rhythm as bars of slots, with the meter of each bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhythmBars {
    pub bars: Vec<Vec<RhythmSlot>>,
    pub meters: Vec<TimeSignature>,
}

impl RhythmBars {
    /// A library pattern's bars.
    #[must_use]
    pub fn of_pattern(pattern: &RhythmPattern) -> RhythmBars {
        let meters: BarMeters = BarMeters::uniform(pattern.time_signature());
        let bar_count: usize = usize::from(pattern.bar_count());
        let mut bars: Vec<Vec<RhythmSlot>> = vec![Vec::new(); bar_count];
        for element in pattern.elements() {
            let (bar_index, offset_ticks): (usize, u64) = meters.bar_at(element.start().ticks());
            if let Some(bar) = bars.get_mut(bar_index) {
                bar.push(RhythmSlot {
                    offset_ticks,
                    length_ticks: element.length().ticks(),
                    sounding: element.kind() == ElementKind::Sounding,
                    marks: element.marks(),
                    tied_to_next: element.tied_to_next(),
                });
            }
        }
        RhythmBars {
            bars,
            meters: vec![pattern.time_signature(); bar_count],
        }
    }

    /// The rhythm of bars a line already has.
    #[must_use]
    pub fn of_written(bars: &[WrittenBar], meters: Vec<TimeSignature>) -> RhythmBars {
        let bars: Vec<Vec<RhythmSlot>> = bars
            .iter()
            .map(|bar: &WrittenBar| {
                let mut offset_ticks: u64 = 0;
                bar.elements
                    .iter()
                    .map(|element: &WrittenElement| {
                        let slot: RhythmSlot = RhythmSlot {
                            offset_ticks,
                            length_ticks: element.length_ticks,
                            sounding: element.sound != WrittenSound::Rest,
                            marks: element.marks,
                            tied_to_next: element.tied_to_next,
                        };
                        offset_ticks += element.length_ticks;
                        slot
                    })
                    .collect()
            })
            .collect();
        RhythmBars { bars, meters }
    }
}

/// What the applied line plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyTarget {
    /// The chords themselves: a harmony track.
    Chords,
    /// The root of each chord in the home octave: a bass line.
    Roots { home_octave: i8 },
    /// Nothing before the colon: a rhythm line that takes its chords from
    /// the track's sample source.
    FromSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// The rhythm's meter is not the bar's.
    MeterDiffers {
        bar_number: usize,
        rhythm_meter: TimeSignature,
        bar_meter: TimeSignature,
    },
    /// A bass root cannot be found: the chord lies outside MIDI, or the
    /// section has no key.
    NoRoot { bar_number: usize, why: String },
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplyError::MeterDiffers {
                bar_number,
                rhythm_meter,
                bar_meter,
            } => write!(
                f,
                "the rhythm is in {}/{} and bar {bar_number} is in {}/{}. Choose a rhythm in the bar's meter",
                rhythm_meter.pulses_per_bar(),
                rhythm_meter.pulses_per_whole_note(),
                bar_meter.pulses_per_bar(),
                bar_meter.pulses_per_whole_note()
            ),
            ApplyError::NoRoot { bar_number, why } => {
                write!(f, "bar {bar_number}: no bass note for the chord: {why}")
            }
        }
    }
}

impl std::error::Error for ApplyError {}

/// The bars a rhythm and a harmony make, and the bars where chords were
/// left over (rule 4), numbered as in the section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub bars: Vec<WrittenBar>,
    pub bars_with_unplaced_chords: Vec<usize>,
}

/// Lays `rhythm` over the bars `first_bar_index` onwards of a section, one
/// for each bar of `harmony`; the rhythm repeats from its first bar.
///
/// # Errors
///
/// When the rhythm's meter is not a bar's, or a root cannot be found.
pub fn apply(
    rhythm: &RhythmBars,
    harmony: &[BarHarmony],
    bar_meters: &BarMeters,
    first_bar_index: usize,
    target: ApplyTarget,
    key: Option<Key>,
    numeral_standard: Option<NumeralStandard>,
) -> Result<Applied, ApplyError> {
    let mut bars: Vec<WrittenBar> = Vec::with_capacity(harmony.len());
    let mut bars_with_unplaced_chords: Vec<usize> = Vec::new();
    let mut tied_from: Option<ElementContents> = None;
    for (offset, bar_harmony) in harmony.iter().enumerate() {
        let bar_number: usize = first_bar_index + offset + 1;
        let rhythm_index: usize = offset % rhythm.bars.len().max(1);
        let (Some(slots), Some(rhythm_meter)) = (
            rhythm.bars.get(rhythm_index),
            rhythm.meters.get(rhythm_index),
        ) else {
            bars.push(WrittenBar::default());
            continue;
        };
        let bar_meter: TimeSignature = bar_meters.meter_of_bar(first_bar_index + offset);
        if *rhythm_meter != bar_meter {
            return Err(ApplyError::MeterDiffers {
                bar_number,
                rhythm_meter: *rhythm_meter,
                bar_meter,
            });
        }
        let mut elements: Vec<WrittenElement> = Vec::with_capacity(slots.len());
        let mut first_sounding: bool = true;
        let mut placed_changes: usize = 0;
        for slot in slots {
            let sound: WrittenSound = if !slot.sounding {
                WrittenSound::Rest
            } else if target == ApplyTarget::FromSource {
                WrittenSound::FromSource
            } else {
                // Rule 2's tie: the continuation plays what it continues.
                let contents: Option<ElementContents> = match tied_from.take() {
                    Some(continued) => Some(continued),
                    None if first_sounding => bar_harmony
                        .changes
                        .first()
                        .map(|change| change.contents.clone()),
                    None => bar_harmony.at(slot.offset_ticks).cloned(),
                };
                match contents {
                    Some(contents) => WrittenSound::Plays(played(
                        &contents,
                        target,
                        key,
                        numeral_standard,
                        bar_number,
                    )?),
                    None => WrittenSound::Rest,
                }
            };
            if slot.sounding {
                let changes_started: usize = bar_harmony
                    .changes
                    .iter()
                    .filter(|change| change.offset_ticks <= slot.offset_ticks)
                    .count()
                    .max(1);
                placed_changes = placed_changes.max(changes_started);
                first_sounding = false;
                if slot.tied_to_next
                    && let WrittenSound::Plays(contents) = &sound
                {
                    tied_from = Some(contents.clone());
                }
            }
            // A rest cannot carry note marks or a tie.
            let (marks, tied_to_next): (ElementMarks, bool) = if sound == WrittenSound::Rest {
                (ElementMarks::NONE, false)
            } else {
                (slot.marks, slot.tied_to_next)
            };
            elements.push(WrittenElement {
                sound,
                length_ticks: slot.length_ticks,
                marks,
                tied_to_next,
                octave_shift_octaves: 0,
            });
        }
        if target != ApplyTarget::FromSource && placed_changes < bar_harmony.changes.len() {
            bars_with_unplaced_chords.push(bar_number);
        }
        bars.push(WrittenBar { elements });
    }
    Ok(Applied {
        bars,
        bars_with_unplaced_chords,
    })
}

/// What one element plays for the target: the chord, or its root.
///
/// # Errors
///
/// When the target wants a root and the contents give none.
pub(crate) fn played(
    contents: &ElementContents,
    target: ApplyTarget,
    key: Option<Key>,
    numeral_standard: Option<NumeralStandard>,
    bar_number: usize,
) -> Result<ElementContents, ApplyError> {
    let ApplyTarget::Roots { home_octave } = target else {
        return Ok(contents.clone());
    };
    let no_key = || ApplyError::NoRoot {
        bar_number,
        why: String::from("the section has no key"),
    };
    let root: Option<Pitch> = match contents {
        ElementContents::Chord(chord) | ElementContents::ChordWithNotes { chord, .. } => {
            let key: Key = key.ok_or_else(no_key)?;
            chord_pitches(*chord, key, numeral_standard, home_octave)
                .map_err(|error| ApplyError::NoRoot {
                    bar_number,
                    why: error.to_string(),
                })?
                .first()
                .copied()
        }
        ElementContents::InvertedChord { chord, inversion } => {
            // The bass of an inversion is its lowest note, as a bass player
            // reads C/E.
            let key: Key = key.ok_or_else(no_key)?;
            inverted_chord_pitches(*chord, key, numeral_standard, home_octave, *inversion)
                .map_err(|error| ApplyError::NoRoot {
                    bar_number,
                    why: error.to_string(),
                })?
                .first()
                .copied()
        }
        ElementContents::Notes(notes) => notes.first().copied(),
        ElementContents::Dynamic(_) | ElementContents::Cue(_) => None,
    };
    root.map(|pitch: Pitch| ElementContents::Notes(vec![pitch]))
        .ok_or_else(|| ApplyError::NoRoot {
            bar_number,
            why: String::from("it holds no chord or note"),
        })
}

/// The chords of each bar, for a sample source line (`| vi | IV V |`):
/// one entry per change, which the source shares evenly over the bar.
#[must_use]
pub fn source_bars(
    harmony: &[BarHarmony],
    target: ApplyTarget,
    key: Option<Key>,
    numeral_standard: Option<NumeralStandard>,
) -> Vec<Vec<ElementContents>> {
    harmony
        .iter()
        .enumerate()
        .map(|(index, bar)| {
            bar.changes
                .iter()
                .filter_map(|change| {
                    played(&change.contents, target, key, numeral_standard, index + 1).ok()
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::line_bars::harmony_of_progression;
    use crate::line_text::line_text;
    use song_text::contents_text::parse_chord;
    use song_text::rhythm_library::{RhythmLibraryEntry, parse_rhythm_library};
    use vocabulary::key::Mode;
    use vocabulary::pitch::{Accidental, NoteLetter, NoteName};

    const LIBRARY: &str =
        include_str!("../../../songs_canonical/composer/composer_rhythms.library");

    fn rhythm(name: &str) -> RhythmBars {
        let entries: Vec<RhythmLibraryEntry> =
            parse_rhythm_library("composer_rhythms.library", LIBRARY).unwrap();
        let entry: &RhythmLibraryEntry = entries
            .iter()
            .find(|entry| entry.pattern().name() == name)
            .unwrap();
        RhythmBars::of_pattern(entry.pattern())
    }

    fn ii_v() -> Vec<Vec<vocabulary::roman_chord::RomanChord>> {
        vec![vec![
            parse_chord("ii7").unwrap(),
            parse_chord("V7").unwrap(),
        ]]
    }

    fn c_major() -> Key {
        Key::new(
            NoteName::new(NoteLetter::C, Accidental::Natural),
            Mode::Major,
        )
    }

    fn written(target: ApplyTarget, rhythm_name: &str) -> String {
        let meters: BarMeters = BarMeters::uniform(TimeSignature::new(4, 4).unwrap());
        let harmony: Vec<BarHarmony> = harmony_of_progression(&ii_v(), &meters, 0, 1);
        let applied: Applied = apply(
            &rhythm(rhythm_name),
            &harmony,
            &meters,
            0,
            target,
            Some(c_major()),
            None,
        )
        .unwrap();
        line_text("", "line \"X\"", &applied.bars)
            .unwrap()
            .join("\n")
    }

    #[test]
    fn the_notation_examples_come_out_as_written() {
        // Song Notation, section 7: ii-V under "Rest, half, quarter".
        assert_eq!(
            written(ApplyTarget::Chords, "Rest, half, quarter"),
            "line \"X\"  | r:q ii7:h V7:q |"
        );
        assert_eq!(
            written(ApplyTarget::Roots { home_octave: 2 }, "Rest, half, quarter"),
            "line \"X\"  | r:q D2:h G2:q |"
        );
        assert_eq!(
            written(ApplyTarget::Chords, "Quarter-note pulses"),
            "line \"X\"  | ii7:q> / V7 / |"
        );
        assert_eq!(
            written(ApplyTarget::FromSource, "Quarter-note pulses"),
            "line \"X\"  | :q> :q :q :q |"
        );
    }

    #[test]
    fn a_chord_with_no_element_is_reported() {
        let meters: BarMeters = BarMeters::uniform(TimeSignature::new(4, 4).unwrap());
        let harmony: Vec<BarHarmony> = harmony_of_progression(&ii_v(), &meters, 0, 1);
        let applied: Applied = apply(
            &rhythm("Whole-note pulse"),
            &harmony,
            &meters,
            0,
            ApplyTarget::Chords,
            Some(c_major()),
            None,
        )
        .unwrap();
        assert_eq!(applied.bars_with_unplaced_chords, vec![1]);
    }
}
