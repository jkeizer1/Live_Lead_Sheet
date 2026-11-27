//! The musical operations of the editor, each giving the song's new text:
//! apply a progression or a rhythm to bars of a track, move chords and
//! notes by intervals, transpose a section with its written notes, give a
//! bass line the roots of the lead's chords, and start a new section from a
//! progression. The session reads the text back before it keeps it.

use note_spelling::chord_pitches::chord_pitches;
use vocabulary::bar_meters::BarMeters;
use vocabulary::key::{Key, Mode};
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::pitch::{NoteName, Pitch};
use vocabulary::progression::Progression;
use vocabulary::rhythm_pattern::{ElementKind, ElementMarks};
use vocabulary::roman_chord::RomanChord;
use vocabulary::song::{
    ElementContents, Line, LineRole, Section, Song, Track, TrackKind, TrackNotes, VoiceKind,
    VoiceLayerNumber,
};
use vocabulary::time_signature::TimeSignature;

use crate::apply::{Applied, ApplyTarget, RhythmBars, RhythmSlot, apply, played};
use crate::chord_moves::{chromatic_chord_step, diatonic_chord_step};
use crate::edits::{
    LineChoice, add_section, line_indent, remove_line, replace_line, with_numeral_standard,
};
use crate::line_bars::{
    BarHarmony, PlacedContents, harmony_of_line, harmony_of_progression, rest_bar,
    written_bars_of_line,
};
use vocabulary::interval::nearest_tonic;

use crate::line_text::{
    NoteSpelling, WrittenBar, WrittenElement, WrittenSound, contents_text, line_text, source_text,
    tonic_octave_for,
};
use crate::pitch_moves::{chord_tone_step, chromatic_step, diatonic_step, octave_step};
use crate::song_source::SongSource;
use crate::timeline::harmony_line;

/// A move by an interval, as the composer thinks of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Steps of the key's scale: a note to the next scale note, a chord to
    /// the next degree's chord.
    Diatonic(i16),
    /// Semitones.
    Chromatic(i16),
    /// Octaves: notes move; a chord is written an octave up or down.
    Octave(i8),
}

impl Step {
    /// The move in words, for the message line.
    #[must_use]
    pub fn describe(self) -> String {
        let (count, unit_one, unit_many): (i16, &str, &str) = match self {
            Step::Diatonic(steps) => (steps, "scale step", "scale steps"),
            Step::Chromatic(semitones) => (semitones, "semitone", "semitones"),
            Step::Octave(octaves) => (i16::from(octaves), "octave", "octaves"),
        };
        let direction: &str = if count >= 0 { "up" } else { "down" };
        let unit: &str = if count.unsigned_abs() == 1 {
            unit_one
        } else {
            unit_many
        };
        format!("{direction} {} {unit}", count.unsigned_abs())
    }
}

/// Where in the song an operation works: a section (by its definition's
/// name), bars of it, and a track.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub song: &'a Song,
    pub text: &'a str,
    pub source: &'a SongSource,
    /// The section as the selected entry plays it (its key is the entry's).
    pub section: &'a Section,
    pub first_bar: usize,
    /// Bars from `first_bar`, at least 1.
    pub bar_count: usize,
    pub track: &'a Track,
}

/// What a track's line holds, as the editor rewrites it.
enum LineBars {
    Rhythm(Vec<WrittenBar>),
    Source(Vec<Vec<ElementContents>>),
}

fn line_of<'a>(section: &'a Section, track_name: &str, role: LineRole) -> Option<&'a Line> {
    section
        .lines()
        .iter()
        .find(|line: &&Line| line.track_name() == track_name && line.role() == role)
}

const FIRST_LAYER: LineRole = LineRole::VoiceLayer(VoiceLayerNumber::FIRST);
const FIRST_SOURCE: LineRole = LineRole::SampleSource(VoiceLayerNumber::FIRST);

fn voice_of(track: &Track) -> Option<(VoiceKind, i8)> {
    match track.kind() {
        TrackKind::Voice { voice, clef, .. } => Some((*voice, clef.default_home_octave())),
        _ => None,
    }
}

fn lead_track(song: &Song) -> Option<&Track> {
    song.tracks()
        .iter()
        .find(|track: &&Track| matches!(track.kind(), TrackKind::Voice { lead: true, .. }))
}

/// The track's rhythm line as written bars, or a bar of rests for each bar
/// when the section has no line for it.
fn rhythm_bars(section: &Section, track_name: &str) -> Vec<WrittenBar> {
    let meters: &BarMeters = section.bar_meters();
    line_of(section, track_name, FIRST_LAYER).map_or_else(
        || {
            (0..usize::try_from(section.bar_count()).unwrap_or(0))
                .map(|bar_index: usize| rest_bar(meters.bar_length_ticks(bar_index)))
                .collect()
        },
        |line: &Line| written_bars_of_line(line, meters),
    )
}

/// The source's chords bar by bar, when the track has a sample source.
fn source_bars(section: &Section, track_name: &str) -> Option<Vec<Vec<ElementContents>>> {
    let line: &Line = line_of(section, track_name, FIRST_SOURCE)?;
    let bars: Vec<WrittenBar> = written_bars_of_line(line, section.bar_meters());
    Some(
        bars.into_iter()
            .map(|bar: WrittenBar| {
                bar.elements
                    .into_iter()
                    .filter_map(|element: WrittenElement| match element.sound {
                        WrittenSound::Plays(contents) => Some(contents),
                        WrittenSound::Rest | WrittenSound::FromSource => None,
                    })
                    .collect()
            })
            .collect(),
    )
}

/// The lowest note among `contents`, for choosing a new line's `tonic_octave`.
fn lowest_pitch<'a>(contents: impl Iterator<Item = &'a ElementContents>) -> Option<Pitch> {
    contents
        .flat_map(|contents: &ElementContents| match contents {
            ElementContents::Notes(notes) | ElementContents::ChordWithNotes { notes, .. } => {
                notes.as_slice()
            }
            _ => &[],
        })
        .copied()
        .min_by_key(|pitch: &Pitch| pitch.semitones_above_c_minus_1())
}

/// How `track`'s line writes its notes, and the `tonic_octave` its header
/// carries. A pitched line is measured from the tonic the loader read it
/// from (`written`: its `tonic_octave` and that tonic), so an edit writes
/// the same intervals back whichever entry is selected. A line that has none
/// yet gets a tonic near its lowest note in `played` (the key the entry
/// plays the section in), and the `tonic_octave` of the tonic of `own` (the
/// section's own key) that leads there (Song Notation, section 4). A MIDI note track writes note names. A
/// line with no notes (a rhythm line, rests) writes none and needs no
/// `tonic_octave`, so its spelling is never used.
///
/// # Errors
///
/// When a pitched line has notes and the section no key.
fn spelling_for(
    track: &Track,
    played: Option<Key>,
    own: Option<Key>,
    written: Option<(i8, Pitch)>,
    lowest: Option<Pitch>,
) -> Result<(NoteSpelling, Option<i8>), String> {
    if !matches!(
        track.kind(),
        TrackKind::Voice {
            notes: TrackNotes::Pitched,
            ..
        }
    ) {
        return Ok((NoteSpelling::NoteNames, None));
    }
    let Some(lowest) = lowest else {
        return Ok((NoteSpelling::NoteNames, None));
    };
    if let Some((octave, tonic)) = written {
        return Ok((NoteSpelling::Intervals { tonic }, Some(octave)));
    }
    let no_key = || {
        format!(
            "track \"{}\" writes intervals, and the section has no key to measure them from. \
             Give the section a key first",
            track.name()
        )
    };
    let played: Key = played.ok_or_else(no_key)?;
    let own: Key = own.ok_or_else(no_key)?;
    let tonic: Pitch = Pitch::new(played.tonic(), tonic_octave_for(played.tonic(), lowest));
    // The octave of the section's own tonic whose nearest override tonic is
    // `tonic`: one of the three octaves around it.
    let octave: i8 = [-1_i8, 0, 1]
        .into_iter()
        .map(|offset: i8| tonic.octave().saturating_add(offset))
        .find(|candidate: &i8| {
            nearest_tonic(Pitch::new(own.tonic(), *candidate), played.tonic()) == tonic
        })
        .ok_or_else(|| format!("no tonic_octave of {own} leads to {tonic} in {played}"))?;
    Ok((NoteSpelling::Intervals { tonic }, Some(octave)))
}

/// A sample source's chords and notes as words, bar by bar.
fn source_words(
    bars: &[Vec<ElementContents>],
    spelling: NoteSpelling,
) -> Result<Vec<Vec<String>>, String> {
    bars.iter()
        .map(|bar: &Vec<ElementContents>| {
            bar.iter()
                .map(|contents: &ElementContents| {
                    contents_text(contents, spelling).map_err(|error| error.to_string())
                })
                .collect()
        })
        .collect()
}

/// Writes `bars` as the chosen line of the section, giving the new text.
fn write_line(
    target: &Target<'_>,
    text: &str,
    source: &SongSource,
    sample_source: bool,
    bars: LineBars,
) -> Result<String, String> {
    let role: LineRole = if sample_source {
        FIRST_SOURCE
    } else {
        FIRST_LAYER
    };
    let written: Option<(i8, Pitch)> = line_of(target.section, target.track.name(), role)
        .and_then(|line: &Line| line.tonic_octave().zip(line.tonic()));
    let lowest: Option<Pitch> = match &bars {
        LineBars::Rhythm(bars) => lowest_pitch(
            bars.iter()
                .flat_map(|bar: &WrittenBar| bar.elements.iter())
                .filter_map(|element: &WrittenElement| match &element.sound {
                    WrittenSound::Plays(contents) => Some(contents),
                    WrittenSound::Rest | WrittenSound::FromSource => None,
                }),
        ),
        LineBars::Source(bars) => lowest_pitch(bars.iter().flatten()),
    };
    let (spelling, tonic_octave): (NoteSpelling, Option<i8>) = spelling_for(
        target.track,
        target.section.key(),
        target.section.own_key(),
        written,
        lowest,
    )?;
    let choice: LineChoice = LineChoice {
        track_name: target.track.name(),
        layer: 1,
        sample_source,
        tonic_octave,
    };
    let indent: String = line_indent(text, source, target.section.name());
    let lines: Vec<String> = match bars {
        LineBars::Rhythm(bars) => line_text(&indent, &choice.header(), &bars, spelling)
            .map_err(|error| error.to_string())?,
        LineBars::Source(bars) => {
            source_text(&indent, &choice.header(), &source_words(&bars, spelling)?)
        }
    };
    replace_line(text, source, target.section.name(), choice, lines)
}

/// The harmony a section's lead holds, bar by bar.
fn lead_harmony(song: &Song, section: &Section) -> Vec<BarHarmony> {
    lead_track(song)
        .and_then(|lead: &Track| harmony_line(section, lead.name()))
        .map_or_else(Vec::new, |line: &Line| {
            harmony_of_line(line, section.bar_meters())
        })
}

fn meters_of(section: &Section, first_bar: usize, bar_count: usize) -> Vec<TimeSignature> {
    (first_bar..first_bar + bar_count)
        .map(|bar_index: usize| section.bar_meters().meter_of_bar(bar_index))
        .collect()
}

fn range_of<T: Clone>(all: &[T], first: usize, count: usize) -> Vec<T> {
    all.iter().skip(first).take(count).cloned().collect()
}

/// `bars[first..first+count]` replaced by `with`.
fn splice_bars<T>(mut bars: Vec<T>, first: usize, with: Vec<T>) -> Vec<T> {
    let end: usize = (first + with.len()).min(bars.len());
    bars.splice(first.min(bars.len())..end, with);
    bars
}

/// The words for a numeral standard, as the song file writes them.
fn standard_word(standard: NumeralStandard) -> &'static str {
    standard.text()
}

/// A text that also declares the library's numeral standard when the song
/// needs one for a minor progression and has none.
fn declare_standard(
    text: String,
    progression: &Progression,
    library_standard: Option<NumeralStandard>,
) -> String {
    match (progression.mode(), library_standard) {
        (Mode::Minor, Some(standard)) => {
            let source: SongSource = SongSource::read(&text);
            with_numeral_standard(&text, &source, standard_word(standard))
        }
        _ => text,
    }
}

/// Every sounding element of the bars made uncoupled: it now takes what it
/// plays from the sample source.
fn uncoupled(bars: Vec<WrittenBar>) -> Vec<WrittenBar> {
    bars.into_iter()
        .map(|bar: WrittenBar| WrittenBar {
            elements: bar
                .elements
                .into_iter()
                .map(|element: WrittenElement| WrittenElement {
                    sound: if element.sound == WrittenSound::Rest {
                        WrittenSound::Rest
                    } else {
                        WrittenSound::FromSource
                    },
                    ..element
                })
                .collect(),
        })
        .collect()
}

/// Most even shares a converted bar's source may have: a bar of
/// sixteenths in 4/4. A finer grid would make the source unreadable.
const MOST_SOURCE_SHARES: u64 = 16;

fn greatest_common_divisor(first: u64, second: u64) -> u64 {
    if second == 0 {
        first
    } else {
        greatest_common_divisor(second, first % second)
    }
}

/// Whether the track's line in the section is sampled: it has a sample
/// source, and its rhythm line takes from it.
#[must_use]
pub fn is_sampled(section: &Section, track_name: &str) -> bool {
    source_bars(section, track_name).is_some()
}

/// The track's coupled line in the section becomes sampled: a sample source
/// holding what the line plays, and a rhythm line of its own timing. Every
/// note sounds as before: each bar's source is shared into as many even
/// parts as it takes for every note to start on a part of its own (a bar
/// of half notes, 2; with an eighth note in it, 8).
///
/// # Errors
///
/// When the line is already sampled or not there, when a bar would need
/// more than 16 parts or plays nothing yet, or the result cannot be
/// written.
pub fn make_sampled(target: &Target<'_>) -> Result<String, String> {
    let section: &Section = target.section;
    let name: &str = target.track.name();
    if is_sampled(section, name) {
        return Err(format!(
            "\"{name}\" is already sampled in section \"{}\"",
            section.name()
        ));
    }
    let Some(line) = line_of(section, name, FIRST_LAYER) else {
        return Err(format!(
            "\"{name}\" has no line in section \"{}\" to convert",
            section.name()
        ));
    };
    let meters: &BarMeters = section.bar_meters();
    let composed: Vec<WrittenBar> = written_bars_of_line(line, meters);
    let harmony: Vec<BarHarmony> = harmony_of_line(line, meters);
    let mut source: Vec<Vec<ElementContents>> = Vec::with_capacity(composed.len());
    for (bar_index, bar) in composed.iter().enumerate() {
        let bar_ticks: u64 = meters.meter_of_bar(bar_index).bar_length().ticks();
        // The share is the largest length that every note's start is a
        // whole number of: gcd over the starts, ticks.
        let mut share_ticks: u64 = bar_ticks;
        let mut offset_ticks: u64 = 0;
        for element in &bar.elements {
            if element.sound != WrittenSound::Rest {
                share_ticks = greatest_common_divisor(share_ticks, offset_ticks);
            }
            offset_ticks += element.length_ticks;
        }
        let shares: u64 = bar_ticks / share_ticks.max(1);
        if shares > MOST_SOURCE_SHARES {
            return Err(format!(
                "bar {} would need {shares} even parts in its source to keep every note; at most {MOST_SOURCE_SHARES} are made",
                bar_index + 1
            ));
        }
        let parts: Option<Vec<ElementContents>> = (0..shares)
            .map(|share: u64| {
                harmony
                    .get(bar_index)
                    .and_then(|bar_harmony: &BarHarmony| bar_harmony.at(share * share_ticks))
                    .cloned()
            })
            .collect();
        source.push(parts.ok_or_else(|| {
            format!(
                "bar {} plays nothing yet, so its source would be empty",
                bar_index + 1
            )
        })?);
    }
    let rhythm_text: String = write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(uncoupled(composed)),
    )?;
    let rhythm_source: SongSource = SongSource::read(&rhythm_text);
    write_line(
        target,
        &rhythm_text,
        &rhythm_source,
        true,
        LineBars::Source(source),
    )
}

/// The track's sampled line in the section becomes coupled: each rhythm
/// element takes the source's chord or note at its start, and the sample
/// source line goes.
///
/// # Errors
///
/// When the line is not sampled, when the source changes while a note
/// sounds (track and hold, which a coupled line cannot hold), or the result
/// cannot be written.
pub fn make_coupled(target: &Target<'_>) -> Result<String, String> {
    let section: &Section = target.section;
    let name: &str = target.track.name();
    let Some(source_line) = section.lines().iter().find(|line: &&Line| {
        line.track_name() == name && line.role() == LineRole::SampleSource(VoiceLayerNumber::FIRST)
    }) else {
        return Err(format!(
            "\"{name}\" is already coupled in section \"{}\"",
            section.name()
        ));
    };
    let Some(line) = line_of(section, name, FIRST_LAYER) else {
        return Err(format!(
            "\"{name}\" has a sample source but no rhythm line in section \"{}\"",
            section.name()
        ));
    };
    let meters: &BarMeters = section.bar_meters();
    let harmony: Vec<BarHarmony> = harmony_of_line(source_line, meters);
    let mut bars: Vec<WrittenBar> = written_bars_of_line(line, meters);
    for (bar_index, bar) in bars.iter_mut().enumerate() {
        let bar_harmony: Option<&BarHarmony> = harmony.get(bar_index);
        let mut offset_ticks: u64 = 0;
        for element in &mut bar.elements {
            let start_ticks: u64 = offset_ticks;
            let end_ticks: u64 = start_ticks + element.length_ticks;
            offset_ticks = end_ticks;
            if element.sound != WrittenSound::FromSource {
                continue;
            }
            let changes_while_sounding: bool = bar_harmony.is_some_and(|bar: &BarHarmony| {
                bar.changes.iter().any(|change: &PlacedContents| {
                    change.offset_ticks > start_ticks && change.offset_ticks < end_ticks
                })
            });
            if changes_while_sounding {
                return Err(format!(
                    "in bar {} the source changes while a note sounds (track and hold). A coupled line cannot hold that, so the line stays sampled",
                    bar_index + 1
                ));
            }
            let contents: &ElementContents = bar_harmony
                .and_then(|bar: &BarHarmony| bar.at(start_ticks))
                .ok_or_else(|| format!("bar {} has no source to take from", bar_index + 1))?;
            element.sound = WrittenSound::Plays(contents.clone());
        }
    }
    let coupled_text: String = write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(bars),
    )?;
    let coupled_source: SongSource = SongSource::read(&coupled_text);
    remove_line(
        &coupled_text,
        &coupled_source,
        section.name(),
        LineChoice {
            track_name: name,
            layer: 1,
            sample_source: true,
            tonic_octave: None,
        },
    )
}

/// Rhythm bars of one sounding element per change, for a track that has no
/// rhythm yet: each chord lasts until the next.
fn rhythm_of_changes(harmony: &[BarHarmony], meters: &[TimeSignature]) -> RhythmBars {
    let bars: Vec<Vec<RhythmSlot>> = harmony
        .iter()
        .zip(meters)
        .map(|(bar, meter): (&BarHarmony, &TimeSignature)| {
            let bar_ticks: u64 = meter.bar_length().ticks();
            let mut starts: Vec<u64> = bar
                .changes
                .iter()
                .map(|change: &PlacedContents| change.offset_ticks)
                .collect();
            if starts.first() != Some(&0) {
                starts.insert(0, 0);
            }
            starts
                .iter()
                .enumerate()
                .map(|(index, start)| RhythmSlot {
                    offset_ticks: *start,
                    length_ticks: starts.get(index + 1).copied().unwrap_or(bar_ticks) - start,
                    sounding: true,
                    marks: ElementMarks::NONE,
                    tied_to_next: false,
                })
                .collect()
        })
        .collect();
    RhythmBars {
        bars,
        meters: meters.to_vec(),
    }
}

/// What the line plays for its kind of voice: the chords on a poly voice,
/// the roots on a mono voice.
fn target_for(track: &Track) -> Result<ApplyTarget, String> {
    match voice_of(track) {
        Some((VoiceKind::Poly, _)) => Ok(ApplyTarget::Chords),
        Some((VoiceKind::Mono, home_octave)) => Ok(ApplyTarget::Roots { home_octave }),
        None => Err(format!("track \"{}\" is not a voice track", track.name())),
    }
}

/// Lays `harmony` over the selected bars as the line is composed: into the
/// sample source of a sampled line, else onto the track's own rhythm.
fn write_harmony(target: &Target<'_>, harmony: &[BarHarmony]) -> Result<String, String> {
    let section: &Section = target.section;
    let key: Option<Key> = section.key();
    let standard: Option<NumeralStandard> = target.song.numeral_standard();
    let apply_target: ApplyTarget = target_for(target.track)?;
    let existing_source: Option<Vec<Vec<ElementContents>>> =
        source_bars(section, target.track.name());
    if let Some(source_now) = existing_source {
        let new_bars: Vec<Vec<ElementContents>> =
            crate::apply::source_bars(harmony, apply_target, key, standard);
        let bars: Vec<Vec<ElementContents>> = splice_bars(source_now, target.first_bar, new_bars);
        if bars.iter().any(Vec::is_empty) {
            return Err(String::from(
                "a bar of the sample source would be empty. Apply a progression to the whole section first",
            ));
        }
        return write_line(
            target,
            target.text,
            target.source,
            true,
            LineBars::Source(bars),
        );
    }
    let all: Vec<WrittenBar> = rhythm_bars(section, target.track.name());
    let meters: Vec<TimeSignature> = meters_of(section, target.first_bar, target.bar_count);
    let has_line: bool = line_of(section, target.track.name(), FIRST_LAYER).is_some();
    let rhythm: RhythmBars = if has_line {
        RhythmBars::of_written(&range_of(&all, target.first_bar, target.bar_count), meters)
    } else {
        rhythm_of_changes(harmony, &meters)
    };
    let applied: Applied = apply(
        &rhythm,
        harmony,
        section.bar_meters(),
        target.first_bar,
        apply_target,
        key,
        standard,
    )
    .map_err(|error| error.to_string())?;
    let bars: Vec<WrittenBar> = splice_bars(all, target.first_bar, applied.bars);
    write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(bars),
    )
}

/// A library progression over the selected bars of the track, keeping the
/// track's rhythm.
///
/// # Errors
///
/// When the result cannot be written (a message says why).
pub fn apply_progression(
    target: &Target<'_>,
    progression: &Progression,
    library_standard: Option<NumeralStandard>,
) -> Result<String, String> {
    let harmony: Vec<BarHarmony> = harmony_of_progression(
        progression.bars(),
        target.section.bar_meters(),
        target.first_bar,
        target.bar_count,
    );
    let text: String = write_harmony(target, &harmony)?;
    Ok(declare_standard(text, progression, library_standard))
}

/// A library rhythm over the selected bars of the track, keeping what the
/// bars play: a coupled line keeps its own chords or notes, a sampled line
/// keeps its source and the rhythm goes to its rhythm line. A track with no
/// line yet takes the lead's chords (their roots, on a mono voice).
///
/// # Errors
///
/// When the rhythm's meter is not the bars', or the result cannot be
/// written.
pub fn apply_rhythm(target: &Target<'_>, rhythm: &RhythmBars) -> Result<String, String> {
    let section: &Section = target.section;
    let meters: &BarMeters = section.bar_meters();
    if source_bars(section, target.track.name()).is_some() {
        let empty: Vec<BarHarmony> = vec![BarHarmony::default(); target.bar_count];
        let applied: Applied = apply(
            rhythm,
            &empty,
            meters,
            target.first_bar,
            ApplyTarget::FromSource,
            None,
            None,
        )
        .map_err(|error| error.to_string())?;
        let all: Vec<WrittenBar> = uncoupled(rhythm_bars(section, target.track.name()));
        let bars: Vec<WrittenBar> = splice_bars(all, target.first_bar, applied.bars);
        return write_line(
            target,
            target.text,
            target.source,
            false,
            LineBars::Rhythm(bars),
        );
    }
    let apply_target: ApplyTarget = target_for(target.track)?;
    // The line's own chords or notes play the new rhythm (Song Notation,
    // section 7: the harmony from the line), so nothing composed is lost.
    // Only a track with no line yet takes the lead's harmony: chords on a
    // poly voice, their roots on a mono voice.
    let harmony_source: Vec<BarHarmony> = line_of(section, target.track.name(), FIRST_LAYER)
        .map_or_else(
            || lead_harmony(target.song, section),
            |line: &Line| harmony_of_line(line, meters),
        );
    let harmony: Vec<BarHarmony> = range_of(&harmony_source, target.first_bar, target.bar_count);
    if harmony.len() < target.bar_count
        || harmony
            .iter()
            .any(|bar: &BarHarmony| bar.changes.is_empty())
    {
        return Err(String::from(
            "these bars have no chords to play yet. Apply a progression first (Progressions tab)",
        ));
    }
    let applied: Applied = apply(
        rhythm,
        &harmony,
        meters,
        target.first_bar,
        apply_target,
        section.key(),
        target.song.numeral_standard(),
    )
    .map_err(|error| error.to_string())?;
    let bars: Vec<WrittenBar> = splice_bars(
        rhythm_bars(section, target.track.name()),
        target.first_bar,
        applied.bars,
    );
    write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(bars),
    )
}

/// The selected bars of a mono track play the roots of the lead's chords,
/// on the track's own rhythm.
///
/// # Errors
///
/// When the track is not a mono voice, or the bars have no chords.
pub fn bass_from_roots(target: &Target<'_>) -> Result<String, String> {
    let Some((VoiceKind::Mono, home_octave)) = voice_of(target.track) else {
        return Err(format!(
            "\"{}\" is not a mono voice: roots go to a bass line",
            target.track.name()
        ));
    };
    let section: &Section = target.section;
    let harmony: Vec<BarHarmony> = range_of(
        &lead_harmony(target.song, section),
        target.first_bar,
        target.bar_count,
    );
    if harmony
        .iter()
        .any(|bar: &BarHarmony| bar.changes.is_empty())
    {
        return Err(String::from("these bars have no chords in the lead track"));
    }
    let all: Vec<WrittenBar> = rhythm_bars(section, target.track.name());
    let meters: Vec<TimeSignature> = meters_of(section, target.first_bar, target.bar_count);
    let rhythm: RhythmBars = if line_of(section, target.track.name(), FIRST_LAYER).is_some() {
        RhythmBars::of_written(&range_of(&all, target.first_bar, target.bar_count), meters)
    } else {
        rhythm_of_changes(&harmony, &meters)
    };
    let applied: Applied = apply(
        &rhythm,
        &harmony,
        section.bar_meters(),
        target.first_bar,
        ApplyTarget::Roots { home_octave },
        section.key(),
        target.song.numeral_standard(),
    )
    .map_err(|error| error.to_string())?;
    let bars: Vec<WrittenBar> = splice_bars(all, target.first_bar, applied.bars);
    write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(bars),
    )
}

/// Contents moved by an interval in `key`. Returns the new contents and
/// the octave shift change a chord needs (a chord is voiced by its track,
/// so an octave move is written as `+8va` or `-8va`).
fn moved(
    contents: &ElementContents,
    step: Step,
    key: Key,
    standard: Option<NumeralStandard>,
) -> (ElementContents, i8) {
    let pitch_moved = |pitch: &Pitch| -> Pitch {
        match step {
            Step::Diatonic(steps) => diatonic_step(*pitch, key, steps),
            Step::Chromatic(semitones) => chromatic_step(*pitch, key, semitones),
            Step::Octave(octaves) => octave_step(*pitch, octaves),
        }
    };
    let chord_moved = |chord: vocabulary::roman_chord::RomanChord| match step {
        Step::Diatonic(steps) => diatonic_chord_step(chord, key, standard, steps),
        Step::Chromatic(semitones) => chromatic_chord_step(chord, key, standard, semitones),
        Step::Octave(_) => chord,
    };
    let shift: i8 = match step {
        Step::Octave(octaves) => octaves,
        Step::Diatonic(_) | Step::Chromatic(_) => 0,
    };
    match contents {
        ElementContents::Notes(notes) => (
            ElementContents::Notes(notes.iter().map(pitch_moved).collect()),
            0,
        ),
        ElementContents::Chord(chord) => (ElementContents::Chord(chord_moved(*chord)), shift),
        ElementContents::InvertedChord { chord, inversion } => {
            let new_chord = chord_moved(*chord);
            // A diatonic step keeps the chord's size, so the inversion still
            // exists; if not, the chord is written in root position.
            match vocabulary::roman_chord::Inversion::new(inversion.number(), new_chord.kind()) {
                Ok(inversion) => (
                    ElementContents::InvertedChord {
                        chord: new_chord,
                        inversion,
                    },
                    shift,
                ),
                Err(_) => (ElementContents::Chord(new_chord), shift),
            }
        }
        ElementContents::ChordWithNotes { chord, notes } => (
            ElementContents::ChordWithNotes {
                chord: chord_moved(*chord),
                notes: notes.iter().map(pitch_moved).collect(),
            },
            0,
        ),
        other @ (ElementContents::Dynamic(_) | ElementContents::Cue(_)) => (other.clone(), 0),
    }
}

/// Moves the written notes of the selected bars of the track (or of one
/// element, when `element` names it) `steps` tones of the lead's chord in
/// effect where each note starts, up or down through that chord's notes,
/// so a bass line is shaped from the harmony without counting intervals.
///
/// # Errors
///
/// When the section has no key, the track no line, the lead no chord at a
/// chosen note, nothing chosen is a written note, or the result cannot be
/// written.
pub fn move_to_chord_tones(
    target: &Target<'_>,
    steps: i16,
    element: Option<usize>,
) -> Result<String, String> {
    let section: &Section = target.section;
    let key: Key = section.key().ok_or_else(|| {
        format!(
            "section \"{}\" has no key, so its chords have no notes",
            section.name()
        )
    })?;
    let standard: Option<NumeralStandard> = target.song.numeral_standard();
    let Some(line) = line_of(section, target.track.name(), FIRST_LAYER) else {
        return Err(format!(
            "\"{}\" has no line in section \"{}\"",
            target.track.name(),
            section.name()
        ));
    };
    let harmony: Vec<BarHarmony> = lead_harmony(target.song, section);
    let bars_range = target.first_bar..target.first_bar + target.bar_count;
    let mut bars: Vec<WrittenBar> = written_bars_of_line(line, section.bar_meters());
    let mut line_index: usize = 0;
    let mut moved_count: usize = 0;
    for (bar_index, bar) in bars.iter_mut().enumerate() {
        let mut offset_ticks: u64 = 0;
        for written in &mut bar.elements {
            let start_ticks: u64 = offset_ticks;
            offset_ticks += written.length_ticks;
            let chosen: bool = match element {
                Some(wanted) => wanted == line_index,
                None => bars_range.contains(&bar_index),
            };
            line_index += 1;
            if !chosen {
                continue;
            }
            let WrittenSound::Plays(contents) = &written.sound else {
                continue;
            };
            let notes: &Vec<Pitch> = match contents {
                ElementContents::Notes(notes) | ElementContents::ChordWithNotes { notes, .. } => {
                    notes
                }
                _ => continue,
            };
            let tones: Vec<NoteName> =
                chord_tones_at(&harmony, bar_index, start_ticks, key, standard)?;
            let stepped: Vec<Pitch> = notes
                .iter()
                .map(|note: &Pitch| {
                    chord_tone_step(*note, &tones, steps)
                        .ok_or_else(|| format!("no chord tone to move to in bar {}", bar_index + 1))
                })
                .collect::<Result<Vec<Pitch>, String>>()?;
            written.sound = WrittenSound::Plays(match contents {
                ElementContents::ChordWithNotes { chord, .. } => ElementContents::ChordWithNotes {
                    chord: *chord,
                    notes: stepped,
                },
                _ => ElementContents::Notes(stepped),
            });
            moved_count += 1;
        }
    }
    if moved_count == 0 {
        return Err(String::from(
            "nothing selected is a written note. Chord tones move notes, not chords",
        ));
    }
    write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(bars),
    )
}

/// Only a chord's note names are wanted, so any octave in range will do.
const ANY_OCTAVE_FOR_NAMES: i8 = 4;

/// The note names of the lead's chord in effect `offset_ticks` into the bar.
fn chord_tones_at(
    harmony: &[BarHarmony],
    bar_index: usize,
    offset_ticks: u64,
    key: Key,
    standard: Option<NumeralStandard>,
) -> Result<Vec<NoteName>, String> {
    let chord: RomanChord = match harmony.get(bar_index).and_then(|bar| bar.at(offset_ticks)) {
        Some(
            ElementContents::Chord(chord)
            | ElementContents::InvertedChord { chord, .. }
            | ElementContents::ChordWithNotes { chord, .. },
        ) => *chord,
        _ => {
            return Err(format!(
                "the lead has no chord in bar {}, so there are no chord tones to move to",
                bar_index + 1
            ));
        }
    };
    let pitches: Vec<Pitch> =
        chord_pitches(chord, key, standard, ANY_OCTAVE_FOR_NAMES).map_err(|error| {
            format!(
                "cannot spell the lead's chord in bar {}: {error}",
                bar_index + 1
            )
        })?;
    Ok(pitches.iter().map(|pitch: &Pitch| pitch.name()).collect())
}

/// Moves what the selected bars of the track play (or one element, when
/// `element` names it) by `step`. On an uncoupled line the chords of the
/// sample source move.
///
/// # Errors
///
/// When the section has no key, or the result cannot be written.
pub fn move_contents(
    target: &Target<'_>,
    step: Step,
    element: Option<usize>,
) -> Result<String, String> {
    let section: &Section = target.section;
    let key: Key = section.key().ok_or_else(|| {
        format!(
            "section \"{}\" has no key, so a scale step means nothing",
            section.name()
        )
    })?;
    let standard: Option<NumeralStandard> = target.song.numeral_standard();
    let bars_range = target.first_bar..target.first_bar + target.bar_count;
    if element.is_none()
        && let Some(source) = source_bars(section, target.track.name())
    {
        let moved_bars: Vec<Vec<ElementContents>> = source
            .into_iter()
            .enumerate()
            .map(|(bar_index, bar)| {
                if bars_range.contains(&bar_index) {
                    bar.iter()
                        .map(|contents| moved(contents, step, key, standard).0)
                        .collect()
                } else {
                    bar
                }
            })
            .collect();
        return write_line(
            target,
            target.text,
            target.source,
            true,
            LineBars::Source(moved_bars),
        );
    }
    let Some(line) = line_of(section, target.track.name(), FIRST_LAYER) else {
        return Err(format!(
            "\"{}\" has no line in section \"{}\"",
            target.track.name(),
            section.name()
        ));
    };
    let meters: &BarMeters = section.bar_meters();
    let mut bars: Vec<WrittenBar> = written_bars_of_line(line, meters);
    let mut line_index: usize = 0;
    for (bar_index, bar) in bars.iter_mut().enumerate() {
        for written in &mut bar.elements {
            let chosen: bool = match element {
                Some(wanted) => wanted == line_index,
                None => bars_range.contains(&bar_index),
            };
            line_index += 1;
            if !chosen {
                continue;
            }
            if let WrittenSound::Plays(contents) = &written.sound {
                let (new_contents, shift): (ElementContents, i8) =
                    moved(contents, step, key, standard);
                written.sound = WrittenSound::Plays(new_contents);
                written.octave_shift_octaves = written.octave_shift_octaves.saturating_add(shift);
            }
        }
    }
    write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(bars),
    )
}

/// The selected element turns from a rest into a sound of the same length,
/// or from a sound into a rest. A new sound is what the lead's harmony
/// gives at that beat: its chord on a poly voice, the chord's root in the
/// clef's home octave on a mono voice, so the note can then be moved by
/// intervals; on an uncoupled line it takes the sample source's chord.
/// Returns the new text, and whether the element is now a rest.
///
/// # Errors
///
/// When the element is not there, or the lead has no chord at that beat.
pub fn toggle_rest(target: &Target<'_>, element: usize) -> Result<(String, bool), String> {
    let section: &Section = target.section;
    let Some(line) = line_of(section, target.track.name(), FIRST_LAYER) else {
        return Err(format!("\"{}\" has no line here", target.track.name()));
    };
    let mut bars: Vec<WrittenBar> = written_bars_of_line(line, section.bar_meters());
    let (bar_index, offset_ticks): (usize, u64) = bars
        .iter()
        .enumerate()
        .flat_map(|(bar_index, bar): (usize, &WrittenBar)| {
            bar.elements
                .iter()
                .scan(0_u64, |offset_ticks: &mut u64, written: &WrittenElement| {
                    let start_ticks: u64 = *offset_ticks;
                    *offset_ticks += written.length_ticks;
                    Some(start_ticks)
                })
                .map(move |start_ticks: u64| (bar_index, start_ticks))
        })
        .nth(element)
        .ok_or_else(|| String::from("no element is selected"))?;
    let new_sound: WrittenSound = {
        let written: &WrittenElement = bars
            .iter()
            .flat_map(|bar: &WrittenBar| bar.elements.iter())
            .nth(element)
            .ok_or_else(|| String::from("no element is selected"))?;
        if written.sound != WrittenSound::Rest {
            WrittenSound::Rest
        } else if source_bars(section, target.track.name()).is_some() {
            WrittenSound::FromSource
        } else {
            let harmony: Vec<BarHarmony> = lead_harmony(target.song, section);
            let contents: &ElementContents = harmony
                .get(bar_index)
                .and_then(|bar: &BarHarmony| bar.at(offset_ticks))
                .ok_or_else(|| {
                    format!(
                        "the lead has no chord in bar {}, so there is no note to give",
                        bar_index + 1
                    )
                })?;
            let sound: ElementContents = played(
                contents,
                target_for(target.track)?,
                section.key(),
                target.song.numeral_standard(),
                bar_index + 1,
            )
            .map_err(|error| error.to_string())?;
            WrittenSound::Plays(sound)
        }
    };
    let now_rest: bool = new_sound == WrittenSound::Rest;
    if let Some(written) = bars
        .iter_mut()
        .flat_map(|bar: &mut WrittenBar| bar.elements.iter_mut())
        .nth(element)
    {
        written.sound = new_sound;
        written.octave_shift_octaves = 0;
    }
    // A rest holds nothing to tie: no tie into it or out of it.
    let mut elements: Vec<&mut WrittenElement> = bars
        .iter_mut()
        .flat_map(|bar: &mut WrittenBar| bar.elements.iter_mut())
        .collect();
    for index in 0..elements.len() {
        let next_is_rest: bool = elements
            .get(index + 1)
            .is_some_and(|next: &&mut WrittenElement| next.sound == WrittenSound::Rest);
        if elements[index].sound == WrittenSound::Rest || next_is_rest {
            elements[index].tied_to_next = false;
        }
    }
    write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(bars),
    )
    .map(|text: String| (text, now_rest))
}

/// The selected element's accent goes to the next level: none, accent,
/// marcato, then none again.
///
/// # Errors
///
/// When the element is not there or is a rest.
pub fn cycle_accent(target: &Target<'_>, element: usize) -> Result<String, String> {
    use vocabulary::rhythm_pattern::AccentLevel;
    let section: &Section = target.section;
    let Some(line) = line_of(section, target.track.name(), FIRST_LAYER) else {
        return Err(format!("\"{}\" has no line here", target.track.name()));
    };
    let mut bars: Vec<WrittenBar> = written_bars_of_line(line, section.bar_meters());
    let written: &mut WrittenElement = bars
        .iter_mut()
        .flat_map(|bar: &mut WrittenBar| bar.elements.iter_mut())
        .nth(element)
        .ok_or_else(|| String::from("no element is selected"))?;
    if written.sound == WrittenSound::Rest {
        return Err(String::from("a rest takes no accent"));
    }
    let marks: ElementMarks = written.marks;
    let next: AccentLevel = match marks.accent() {
        AccentLevel::Normal | AccentLevel::Soft | AccentLevel::Softer => AccentLevel::Accent,
        AccentLevel::Accent => AccentLevel::Marcato,
        AccentLevel::Marcato => AccentLevel::Normal,
    };
    written.marks = ElementMarks::new(next, marks.staccato(), marks.legato(), marks.fermata());
    write_line(
        target,
        target.text,
        target.source,
        false,
        LineBars::Rhythm(bars),
    )
}

/// The section's lines measured from a tonic `semitones` away, for a
/// section whose key moves from `key` to `new_key`. Its intervals and chord
/// numerals follow the key by themselves; only a `tonic_octave` changes,
/// where the new tonic lies in another octave, so the music moves by
/// `semitones` and not by a leap to the new tonic's octave.
///
/// # Errors
///
/// When the section is not in the text, or `new_key`'s tonic is not
/// `semitones` from `key`'s.
pub fn retune_tonic_octaves(
    text: &str,
    source: &SongSource,
    section_name: &str,
    key: Key,
    new_key: Key,
    semitones: i16,
) -> Result<String, String> {
    let section = source
        .section(section_name)
        .ok_or_else(|| format!("could not find section \"{section_name}\" in the text"))?;
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    for line in &section.lines {
        let Some(first) = lines.get(line.first_index) else {
            continue;
        };
        let words: Vec<&str> = first.split_whitespace().collect();
        let Some(position) = words.iter().position(|word: &&str| *word == "tonic_octave") else {
            continue;
        };
        let Some(octave) = words
            .get(position + 1)
            .and_then(|word: &&str| word.parse::<i8>().ok())
        else {
            continue;
        };
        // semitones above C-1 of the new tonic = of the old + semitones;
        // its octave is what remains after the new tonic's letter and
        // accidental: (semitones − letter − accidental) ÷ 12 semitones/octave − 1.
        let wanted_semitones: i16 =
            Pitch::new(key.tonic(), octave).semitones_above_c_minus_1() + semitones;
        let within_octave: i16 = new_key.tonic().letter().semitones_above_c()
            + new_key.tonic().accidental().semitone_offset();
        let new_octave: i16 = (wanted_semitones - within_octave).div_euclid(12) - 1;
        let new_octave: i8 = i8::try_from(new_octave)
            .ok()
            .filter(|new_octave: &i8| {
                Pitch::new(new_key.tonic(), *new_octave).semitones_above_c_minus_1()
                    == wanted_semitones
            })
            .ok_or_else(|| {
                format!(
                    "{new_key} is not {semitones} semitones from {key}, so its notes cannot follow"
                )
            })?;
        if new_octave != octave {
            let old_words: String = format!("tonic_octave {octave}");
            lines[line.first_index] =
                first.replacen(&old_words, &format!("tonic_octave {new_octave}"), 1);
        }
    }
    Ok(lines.join("\n"))
}

/// A new section after the entry `after_entry_index`, from a progression:
/// the lead plays each chord for its share of the bar, a bass line plays
/// the roots, and the other tracks rest. The section is named after the
/// progression, numbered if the name is taken.
///
/// # Errors
///
/// When the section cannot be written.
#[allow(clippy::too_many_arguments)] // one value per thing the new section takes from the song
pub fn new_section_from_progression(
    song: &Song,
    text: &str,
    source: &SongSource,
    after_entry_index: usize,
    progression: &Progression,
    library_standard: Option<NumeralStandard>,
    meter: TimeSignature,
    key: Option<Key>,
) -> Result<(String, String), String> {
    let mut name: String = progression.name().to_string();
    let mut number: u32 = 2;
    while source.section(&name).is_some() {
        name = format!("{} {number}", progression.name());
        number += 1;
    }
    let bar_count: usize = progression.bars().len();
    let meters: BarMeters = BarMeters::uniform(meter);
    let harmony: Vec<BarHarmony> =
        harmony_of_progression(progression.bars(), &meters, 0, bar_count);
    let rhythm: RhythmBars = rhythm_of_changes(&harmony, &vec![meter; bar_count]);
    let mut block: Vec<String> = vec![
        format!("section \"{name}\""),
        format!(
            "  meter {}/{}",
            meter.pulses_per_bar(),
            meter.pulses_per_whole_note()
        ),
    ];
    // The key is written, so the roots written below and the numerals mean
    // the same wherever the section is played.
    if let Some(key) = key {
        block.push(format!("  key {key}"));
    }
    for track in song.tracks() {
        let Some((voice, home_octave)) = voice_of(track) else {
            continue;
        };
        let is_lead: bool = matches!(track.kind(), TrackKind::Voice { lead: true, .. });
        let target: ApplyTarget = match (is_lead, voice) {
            (true, _) => ApplyTarget::Chords,
            (false, VoiceKind::Mono)
                if matches!(
                    track.kind(),
                    TrackKind::Voice {
                        clef: vocabulary::song::Clef::Bass,
                        ..
                    }
                ) =>
            {
                ApplyTarget::Roots { home_octave }
            }
            _ => continue,
        };
        // The bass roots are spelled in the key the new section first plays
        // in (the entry before it), which it writes as its own.
        let applied: Applied = apply(
            &rhythm,
            &harmony,
            &meters,
            0,
            target,
            key,
            song.numeral_standard(),
        )
        .map_err(|error| error.to_string())?;
        let lowest: Option<Pitch> = lowest_pitch(
            applied
                .bars
                .iter()
                .flat_map(|bar: &WrittenBar| bar.elements.iter())
                .filter_map(|element: &WrittenElement| match &element.sound {
                    WrittenSound::Plays(contents) => Some(contents),
                    WrittenSound::Rest | WrittenSound::FromSource => None,
                }),
        );
        let (spelling, tonic_octave): (NoteSpelling, Option<i8>) =
            // The new section writes `key` as its own and plays in it.
            spelling_for(track, key, key, None, lowest)?;
        let header: String = LineChoice {
            track_name: track.name(),
            layer: 1,
            sample_source: false,
            tonic_octave,
        }
        .header();
        block.extend(
            line_text("  ", &header, &applied.bars, spelling).map_err(|error| error.to_string())?,
        );
    }
    let new_text: String = add_section(text, source, after_entry_index, &name, block)?;
    Ok((
        declare_standard(new_text, progression, library_standard),
        name,
    ))
}

/// The kind of an element at an index of a line: whether it sounds.
#[must_use]
pub fn sounds(line: &Line, element: usize) -> bool {
    line.elements()
        .get(element)
        .is_some_and(|line_element| line_element.element().kind() == ElementKind::Sounding)
}
