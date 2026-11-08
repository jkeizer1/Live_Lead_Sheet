//! One line of a section, spelled: its bars, each a list of written notes
//! and rests, with triplets grouped. The places and lengths come from the
//! vocabulary in ticks; the note values, dots and groups are worked out
//! here, once, for every converter.

use vocabulary::key::Key;
use vocabulary::pitch::Pitch;
use vocabulary::rhythm_pattern::{ElementKind, ElementMarks, RhythmElement};
use vocabulary::roman_chord::RomanChord;
use vocabulary::song::{ElementContents, Line, LineElement, Section, Track, TrackKind};

use crate::chord_pitches::chord_pitches;
use crate::note_value::{BaseValue, NoteValue};
use crate::spelling_error::{SpellingError, SpellingProblem};

/// What a written element sounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sound {
    Rest,
    /// One note, or the notes of a chord, lowest first as written.
    Pitches(Vec<Pitch>),
    /// A cue's name, on a cue control track.
    Cue(String),
}

/// One written note, chord or rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpelledElement {
    /// Ticks from the start of its bar.
    pub start_ticks: u64,
    /// The written value; inside a triplet, the value before the triplet
    /// squeezes it (a triplet quarter is written as a quarter).
    pub value: NoteValue,
    pub sound: Sound,
    /// The chord as written in the song, when the element names one.
    pub chord: Option<RomanChord>,
    pub marks: ElementMarks,
    pub tied_to_next: bool,
}

/// An element on its own, or a triplet: three in the time of two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpelledItem {
    Single(SpelledElement),
    Triplet(Vec<SpelledElement>),
}

/// One bar of a spelled line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpelledBar {
    pub items: Vec<SpelledItem>,
}

/// Spells `line`, the line of `track` in `section`.
///
/// # Errors
///
/// When a length is neither a plain or dotted value nor part of a complete
/// triplet in its bar, or a chord cannot be voiced.
pub fn spell_line(
    track: &Track,
    section: &Section,
    line: &Line,
) -> Result<Vec<SpelledBar>, SpellingError> {
    let bar_length_ticks: u64 = section.time_signature().bar_length().ticks();
    let home_octave: Option<i8> = match track.kind() {
        TrackKind::Voice {
            clef, home_octave, ..
        } => Some(home_octave.unwrap_or_else(|| clef.default_home_octave())),
        TrackKind::ClockAndTransport | TrackKind::CueControl { .. } => None,
    };
    let error_at = |bar_index: usize, problem: SpellingProblem| SpellingError {
        section_name: section.name().to_string(),
        track_name: track.name().to_string(),
        bar_number: bar_index + 1,
        problem,
    };

    let bar_count: usize = usize::try_from(line.bar_count()).unwrap_or(usize::MAX);
    let mut bars: Vec<SpelledBar> = Vec::with_capacity(bar_count);
    let mut triplet: Vec<SpelledElement> = Vec::new();
    // The triplet's length as played, so far.
    let mut triplet_ticks: u64 = 0;
    let mut current_bar_index: usize = 0;
    let mut items: Vec<SpelledItem> = Vec::new();

    for line_element in line.elements() {
        let element: &RhythmElement = line_element.element();
        // Elements never cross a barline (the song file refuses it), so an
        // element's bar is the bar its start lies in.
        let bar_index: usize =
            usize::try_from(element.start().ticks() / bar_length_ticks).unwrap_or(usize::MAX);
        if bar_index != current_bar_index {
            if !triplet.is_empty() {
                return Err(error_at(
                    current_bar_index,
                    SpellingProblem::TripletNotComplete,
                ));
            }
            bars.push(SpelledBar {
                items: std::mem::take(&mut items),
            });
            current_bar_index = bar_index;
        }
        let start_ticks: u64 = element.start().ticks() % bar_length_ticks;
        let length_ticks: u64 = element.length().ticks();
        let (sound, chord): (Sound, Option<RomanChord>) =
            sound_of(line_element, section.key(), home_octave)
                .map_err(|problem| error_at(bar_index, problem))?;

        let plain: Option<NoteValue> = NoteValue::from_ticks(length_ticks);
        // A triplet note plays 2/3 of its written value: written = played × 3/2.
        let triplet_value: Option<NoteValue> = if (length_ticks * 3).is_multiple_of(2) {
            NoteValue::from_ticks(length_ticks * 3 / 2)
        } else {
            None
        };
        let spelled = |value: NoteValue| SpelledElement {
            start_ticks,
            value,
            sound: sound.clone(),
            chord,
            marks: element.marks(),
            tied_to_next: element.tied_to_next(),
        };
        match (triplet.is_empty(), plain, triplet_value) {
            (true, Some(value), _) => items.push(SpelledItem::Single(spelled(value))),
            (_, _, Some(value)) => {
                triplet.push(spelled(value));
                triplet_ticks += length_ticks;
                // A triplet is complete when what it plays is one plain
                // value: three triplet eighths are a quarter.
                if is_undotted_value(triplet_ticks) {
                    items.push(SpelledItem::Triplet(std::mem::take(&mut triplet)));
                    triplet_ticks = 0;
                }
            }
            (false, Some(_), None) => {
                return Err(error_at(bar_index, SpellingProblem::TripletNotComplete));
            }
            (_, None, None) => {
                return Err(error_at(
                    bar_index,
                    SpellingProblem::LengthNotSpelled { length_ticks },
                ));
            }
        }
    }
    if !triplet.is_empty() {
        return Err(error_at(
            current_bar_index,
            SpellingProblem::TripletNotComplete,
        ));
    }
    bars.push(SpelledBar { items });
    Ok(bars)
}

fn is_undotted_value(length_ticks: u64) -> bool {
    [
        BaseValue::Whole,
        BaseValue::Half,
        BaseValue::Quarter,
        BaseValue::Eighth,
        BaseValue::Sixteenth,
        BaseValue::ThirtySecond,
    ]
    .iter()
    .any(|base: &BaseValue| base.ticks() == length_ticks)
}

/// What an element sounds, and the chord it names if any.
fn sound_of(
    line_element: &LineElement,
    key: Option<Key>,
    home_octave: Option<i8>,
) -> Result<(Sound, Option<RomanChord>), SpellingProblem> {
    if line_element.element().kind() == ElementKind::Rest {
        return Ok((Sound::Rest, None));
    }
    match line_element.contents() {
        None => Err(SpellingProblem::NothingWritten),
        Some(ElementContents::Notes(pitches)) => Ok((Sound::Pitches(pitches.clone()), None)),
        Some(ElementContents::ChordWithNotes { chord, notes }) => {
            Ok((Sound::Pitches(notes.clone()), Some(*chord)))
        }
        Some(ElementContents::Cue(name)) => Ok((Sound::Cue(name.clone()), None)),
        Some(ElementContents::Chord(chord)) => {
            let key: Key = key.ok_or(SpellingProblem::ChordWithoutKey)?;
            // A chord on a track with no home octave: the song file only
            // lets voice tracks carry chords, so this is reported, not guessed.
            let home_octave: i8 = home_octave.ok_or(SpellingProblem::NothingWritten)?;
            let pitches: Vec<Pitch> =
                chord_pitches(*chord, key, home_octave).map_err(SpellingProblem::Voicing)?;
            Ok((Sound::Pitches(pitches), Some(*chord)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use song_text::song_file::parse_song;
    use vocabulary::song::Song;

    /// A song with a poly voice "Keys" and one section whose Keys line is
    /// `bars`, in C major 4/4.
    fn song_of(bars: &str) -> Song {
        let text: String = format!(
            "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
             track \"Keys\"\n  type poly_voice\n  clef grand\n  lead\n\
             section \"A\"\n  key C major  meter 4/4  tempo 120 beats/min\n\
             \x20 line \"Keys\"  {bars}\n"
        );
        parse_song("test.song", &text).unwrap()
    }

    fn spelled(bars: &str) -> Result<Vec<SpelledBar>, SpellingError> {
        let song: Song = song_of(bars);
        let section: &Section = &song.sections()[0];
        spell_line(&song.tracks()[1], section, &section.lines()[0])
    }

    fn values(item: &SpelledItem) -> Vec<(u64, u8)> {
        let of = |element: &SpelledElement| {
            (element.value.base().per_whole_note(), element.value.dots())
        };
        match item {
            SpelledItem::Single(element) => vec![of(element)],
            SpelledItem::Triplet(elements) => elements.iter().map(of).collect(),
        }
    }

    #[test]
    fn plain_and_dotted_values_are_spelled_bar_by_bar() {
        let bars: Vec<SpelledBar> = spelled("| C4:q D4:e. E4:sx F4:h | G4:w |").unwrap();
        assert_eq!(bars.len(), 2);
        let first: Vec<Vec<(u64, u8)>> = bars[0].items.iter().map(values).collect();
        assert_eq!(
            first,
            vec![vec![(4, 0)], vec![(8, 1)], vec![(16, 0)], vec![(2, 0)]]
        );
        let SpelledItem::Single(f4) = &bars[0].items[3] else {
            panic!("F4 is a single note");
        };
        assert_eq!(f4.start_ticks, 10_080);
        assert_eq!(bars[1].items.len(), 1);
    }

    #[test]
    fn triplets_are_grouped_and_written_at_their_unsqueezed_value() {
        let bars: Vec<SpelledBar> = spelled("| trp (C4:q D4 E4) trp (F4:e G4 A4) B4:q |").unwrap();
        assert_eq!(bars[0].items.len(), 3);
        assert!(matches!(&bars[0].items[0], SpelledItem::Triplet(notes) if notes.len() == 3));
        assert_eq!(values(&bars[0].items[0]), vec![(4, 0); 3]);
        assert_eq!(values(&bars[0].items[1]), vec![(8, 0); 3]);
        assert_eq!(values(&bars[0].items[2]), vec![(4, 0)]);
    }

    #[test]
    fn five_in_a_quarter_is_reported_in_musical_terms() {
        let error: SpellingError = spelled("| fitQ(C4:e D4 E4 F4 G4) A4:q B4:h |").unwrap_err();
        assert_eq!(
            error.to_string(),
            "section \"A\", track \"Keys\", bar 1: a note 1/5 of a quarter note long cannot \
             be drawn yet. The view draws plain and dotted values and triplets; the song still \
             plays"
        );
    }

    #[test]
    fn a_chord_is_spelled_as_its_voiced_notes_and_keeps_its_name() {
        let bars: Vec<SpelledBar> = spelled("| I:h ii7 |").unwrap();
        let SpelledItem::Single(second) = &bars[0].items[1] else {
            panic!("ii7 is a single chord");
        };
        assert_eq!(
            second.chord.map(|chord| chord.to_string()),
            Some("ii7".to_string())
        );
        let Sound::Pitches(pitches) = &second.sound else {
            panic!("a chord sounds pitches");
        };
        let names: Vec<String> = pitches.iter().map(ToString::to_string).collect();
        assert_eq!(names, vec!["D4", "F4", "A4", "C5"]);
    }

    #[test]
    fn rests_marks_and_ties_are_kept() {
        let bars: Vec<SpelledBar> = spelled("| r:q C4:q> D4~ D4^ |").unwrap();
        let elements: Vec<&SpelledElement> = bars[0]
            .items
            .iter()
            .map(|item: &SpelledItem| match item {
                SpelledItem::Single(element) => element,
                SpelledItem::Triplet(_) => panic!("no triplets here"),
            })
            .collect();
        assert_eq!(elements[0].sound, Sound::Rest);
        assert_eq!(
            elements[1].marks.accent(),
            vocabulary::rhythm_pattern::AccentLevel::Accent
        );
        assert!(elements[2].tied_to_next);
        assert!(elements[3].marks.fermata());
    }
}
