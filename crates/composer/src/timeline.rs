//! The song as the screen shows it: the composition's entries in order,
//! each with its bars, the lead's chords named in the entry's key, and
//! every voice track's elements with their written pitches. Built from the
//! song as read, so the screen shows exactly what plays (EFS-5.8).

use note_spelling::chord_pitches::{chord_pitches, inverted_chord_pitches};
use note_spelling::note_value::NoteValue;
use vocabulary::bar_meters::BarMeters;
use vocabulary::key::Key;
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::pitch::Pitch;
use vocabulary::rhythm_pattern::{ElementKind, ElementMarks};
use vocabulary::song::{
    Clef, ElementContents, Line, LineElement, LineRole, Section, Song, Track, TrackKind, VoiceKind,
    VoiceLayerNumber,
};
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;
use vocabulary::time_signature::TimeSignature;

use crate::chord_symbol::{chord_symbol, inverted_chord_symbol, note_name_text, numeral_text};
use crate::line_bars::{BarHarmony, harmony_of_line};
use crate::song_source::{EntrySource, SongSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackView {
    pub name: String,
    pub voice: VoiceKind,
    pub clef: Clef,
    pub lead: bool,
}

/// A chord symbol at its place in a bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChordView {
    pub offset_ticks: u64,
    /// `Dm7`, spelled in the entry's key.
    pub symbol: String,
    /// `ii7`.
    pub numeral: String,
}

/// One element of a track's line in a bar.
// Each flag is a separate fact about the element as written.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementView {
    pub offset_ticks: u64,
    pub length_ticks: u64,
    pub rest: bool,
    /// An uncoupled element: it plays what the sample source holds.
    pub from_source: bool,
    /// The written pitches it sounds, lowest first, octave annotation
    /// applied.
    pub pitches: Vec<Pitch>,
    /// What it plays in words: a chord symbol or the note names.
    pub label: String,
    /// The written value, when it has one; inside a triplet, the value
    /// before the triplet squeezes it.
    pub value: Option<NoteValue>,
    pub triplet: bool,
    pub marks: ElementMarks,
    pub tied_to_next: bool,
    /// The element's place in its line (for editing).
    pub line_element_index: usize,
}

/// A chord or note of a sample source, with the share of the bar it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceView {
    pub offset_ticks: u64,
    pub length_ticks: u64,
    pub label: String,
}

/// One track in one bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrackBarView {
    /// Whether the section has a line for the track; without one it rests.
    pub has_line: bool,
    /// Whether the line is sampled: its rhythm line takes what it plays
    /// from a sample source (`source`), so its pitches are derived. A line
    /// that is not sampled is coupled.
    pub sampled: bool,
    pub elements: Vec<ElementView>,
    /// The sample source's chords, for an uncoupled line.
    pub source: Vec<SourceView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarView {
    pub meter: TimeSignature,
    pub length_ticks: u64,
    /// The lead's chords, where each starts.
    pub chords: Vec<ChordView>,
    /// One per track in `Timeline::tracks`.
    pub tracks: Vec<TrackBarView>,
}

/// One entry of the composition.
// Each flag is a separate fact about the entry as written.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryView {
    pub section_name: String,
    pub repeat_count: u32,
    /// `repeat 0`: listed, not played.
    pub muted: bool,
    pub count_in: bool,
    /// The key the entry plays in, and whether the entry writes it itself.
    pub key: Option<Key>,
    pub key_written_on_entry: bool,
    /// The meter of the first bar; `None` for a muted entry whose section
    /// no other entry plays, so it was never built.
    pub meter: Option<TimeSignature>,
    /// Tempo in thousandths of a beat per minute.
    pub milli_beats_per_minute: u64,
    pub tempo_written_on_entry: bool,
    /// The bars of one pass.
    pub bars: Vec<BarView>,
    /// Ticks from the start of the song to the entry's first pass; a muted
    /// entry has the place where it would start.
    pub start_ticks: u64,
    /// The length of one pass.
    pub pass_length_ticks: u64,
}

impl EntryView {
    /// All passes together.
    #[must_use]
    pub fn length_ticks(&self) -> u64 {
        if self.muted {
            0
        } else {
            self.pass_length_ticks * u64::from(self.repeat_count)
        }
    }

    /// Bars in one pass.
    #[must_use]
    pub fn bar_count(&self) -> usize {
        self.bars.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    pub song_name: String,
    /// The voice tracks, in the song's order.
    pub tracks: Vec<TrackView>,
    pub entries: Vec<EntryView>,
    pub total_ticks: u64,
}

/// A place in the song as the screen names it: which entry, which pass,
/// which bar of the section, and how far into the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelinePlace {
    pub entry_index: usize,
    /// Counted from 1.
    pub pass_number: u32,
    pub bar_index: usize,
    pub offset_ticks: u64,
}

impl Timeline {
    /// Builds the view of `song`, whose entries `source` lists.
    #[must_use]
    pub fn build(song: &Song, source: &SongSource) -> Timeline {
        let tracks: Vec<(&Track, TrackView)> = song
            .tracks()
            .iter()
            .filter_map(|track: &Track| match track.kind() {
                TrackKind::Voice {
                    voice, clef, lead, ..
                } => Some((
                    track,
                    TrackView {
                        name: track.name().to_string(),
                        voice: *voice,
                        clef: *clef,
                        lead: *lead,
                    },
                )),
                _ => None,
            })
            .collect();
        let numeral_standard: Option<NumeralStandard> = song.numeral_standard();
        let written_entries: Vec<EntrySource> = source
            .composition
            .as_ref()
            .map_or_else(Vec::new, |composition| composition.entries.clone());
        let mut playing = song.sections().iter();
        let mut entries: Vec<EntryView> = Vec::new();
        let mut start_ticks: u64 = 0;
        for written in &written_entries {
            let section: Option<&Section> = if written.repeat_count == 0 {
                // Muted: no section was built for it; show the bars of any
                // other entry that plays the same section.
                song.sections()
                    .iter()
                    .find(|section: &&Section| section.name() == written.section_name)
            } else {
                playing.next()
            };
            let Some(section) = section else {
                entries.push(EntryView {
                    section_name: written.section_name.clone(),
                    repeat_count: written.repeat_count,
                    muted: true,
                    count_in: false,
                    key: None,
                    key_written_on_entry: written.key_words.is_some(),
                    meter: None,
                    milli_beats_per_minute: 0,
                    tempo_written_on_entry: written.tempo_words.is_some(),
                    bars: Vec::new(),
                    start_ticks,
                    pass_length_ticks: 0,
                });
                continue;
            };
            let mut entry: EntryView =
                entry_view(written, section, &tracks, numeral_standard, start_ticks);
            if written.repeat_count == 0 {
                entry.muted = true;
                entry.repeat_count = 0;
            }
            start_ticks += entry.length_ticks();
            entries.push(entry);
        }
        Timeline {
            song_name: song.name().to_string(),
            tracks: tracks.into_iter().map(|(_, view)| view).collect(),
            entries,
            total_ticks: start_ticks,
        }
    }

    /// Where `position_ticks` falls; `None` past the end.
    #[must_use]
    pub fn place_at(&self, position_ticks: u64) -> Option<TimelinePlace> {
        for (entry_index, entry) in self.entries.iter().enumerate() {
            if entry.muted || entry.pass_length_ticks == 0 {
                continue;
            }
            let end_ticks: u64 = entry.start_ticks + entry.length_ticks();
            if position_ticks < entry.start_ticks || position_ticks >= end_ticks {
                continue;
            }
            let into_entry_ticks: u64 = position_ticks - entry.start_ticks;
            // ticks ÷ ticks/pass → whole passes before this one.
            let pass_index: u64 = into_entry_ticks / entry.pass_length_ticks;
            let mut into_pass_ticks: u64 = into_entry_ticks % entry.pass_length_ticks;
            for (bar_index, bar) in entry.bars.iter().enumerate() {
                if into_pass_ticks < bar.length_ticks {
                    return Some(TimelinePlace {
                        entry_index,
                        pass_number: u32::try_from(pass_index + 1).unwrap_or(u32::MAX),
                        bar_index,
                        offset_ticks: into_pass_ticks,
                    });
                }
                into_pass_ticks -= bar.length_ticks;
            }
        }
        None
    }

    /// Ticks from the song's start to the start of a bar of an entry's
    /// first pass.
    #[must_use]
    pub fn bar_start_ticks(&self, entry_index: usize, bar_index: usize) -> Option<u64> {
        let entry: &EntryView = self.entries.get(entry_index)?;
        let into_ticks: u64 = entry
            .bars
            .iter()
            .take(bar_index)
            .map(|bar: &BarView| bar.length_ticks)
            .sum();
        Some(entry.start_ticks + into_ticks)
    }
}

/// milli-quarters/min × ticks/q ÷ ticks/beat → milli-beats/min.
fn milli_beats_per_minute(section: &Section) -> u64 {
    section.tempo().milli_quarters_per_minute() * TICKS_PER_QUARTER_NOTE
        / section.time_signature().beat_length().ticks().max(1)
}

fn entry_view(
    written: &EntrySource,
    section: &Section,
    tracks: &[(&Track, TrackView)],
    numeral_standard: Option<NumeralStandard>,
    start_ticks: u64,
) -> EntryView {
    let key: Option<Key> = section.key();
    let bar_meters: &BarMeters = section.bar_meters();
    let bar_count: usize = usize::try_from(section.bar_count()).unwrap_or(0);
    let lead_harmony: Vec<BarHarmony> = tracks
        .iter()
        .find(|(_, view)| view.lead)
        .and_then(|(track, _)| harmony_line(section, track.name()))
        .map_or_else(Vec::new, |line: &Line| harmony_of_line(line, bar_meters));
    let mut bars: Vec<BarView> = (0..bar_count)
        .map(|bar_index: usize| BarView {
            meter: bar_meters.meter_of_bar(bar_index),
            length_ticks: bar_meters.bar_length_ticks(bar_index),
            chords: lead_harmony
                .get(bar_index)
                .map_or_else(Vec::new, |bar| chord_views(bar, key, numeral_standard)),
            tracks: vec![TrackBarView::default(); tracks.len()],
        })
        .collect();
    for (track_index, (track, view)) in tracks.iter().enumerate() {
        let home_octave: i8 = view.clef.default_home_octave();
        let source: Option<&Line> = section.lines().iter().find(|line: &&Line| {
            line.track_name() == track.name()
                && line.role() == LineRole::SampleSource(VoiceLayerNumber::FIRST)
        });
        let source_harmony: Vec<BarHarmony> =
            source.map_or_else(Vec::new, |line: &Line| harmony_of_line(line, bar_meters));
        let Some(line) = section.lines().iter().find(|line: &&Line| {
            line.track_name() == track.name()
                && line.role() == LineRole::VoiceLayer(VoiceLayerNumber::FIRST)
        }) else {
            continue;
        };
        let speller: Speller = Speller {
            key,
            numeral_standard,
            home_octave,
        };
        for (line_element_index, line_element) in line.elements().iter().enumerate() {
            let (bar_index, offset_ticks): (usize, u64) =
                bar_meters.bar_at(line_element.element().start().ticks());
            let Some(bar) = bars.get_mut(bar_index) else {
                continue;
            };
            let from_source_contents: Option<&ElementContents> = source_harmony
                .get(bar_index)
                .and_then(|harmony: &BarHarmony| harmony.at(offset_ticks));
            let element: ElementView = element_view(
                line_element,
                line_element_index,
                offset_ticks,
                from_source_contents,
                &speller,
            );
            if let Some(track_bar) = bar.tracks.get_mut(track_index) {
                track_bar.has_line = true;
                track_bar.elements.push(element);
            }
        }
        if let Some(source_line) = source {
            for (bar_index, bar) in bars.iter_mut().enumerate() {
                if let Some(track_bar) = bar.tracks.get_mut(track_index) {
                    track_bar.sampled = true;
                    track_bar.source = source_views(source_line, bar_meters, bar_index, &speller);
                }
            }
        }
    }
    EntryView {
        section_name: written.section_name.clone(),
        repeat_count: section.repeat_count(),
        muted: false,
        count_in: section.count_in(),
        key,
        key_written_on_entry: written.key_words.is_some(),
        meter: Some(section.time_signature()),
        milli_beats_per_minute: milli_beats_per_minute(section),
        tempo_written_on_entry: written.tempo_words.is_some(),
        bars,
        start_ticks,
        pass_length_ticks: section.length_ticks(),
    }
}

/// The line the track's chords come from: its sample source when its line
/// is uncoupled, else its first voice layer.
#[must_use]
pub fn harmony_line<'a>(section: &'a Section, track_name: &str) -> Option<&'a Line> {
    let of_role = |role: LineRole| {
        section
            .lines()
            .iter()
            .find(move |line: &&Line| line.track_name() == track_name && line.role() == role)
    };
    of_role(LineRole::SampleSource(VoiceLayerNumber::FIRST))
        .or_else(|| of_role(LineRole::VoiceLayer(VoiceLayerNumber::FIRST)))
}

fn chord_views(
    bar: &BarHarmony,
    key: Option<Key>,
    numeral_standard: Option<NumeralStandard>,
) -> Vec<ChordView> {
    bar.changes
        .iter()
        .filter_map(|change| {
            let (symbol, numeral): (String, String) = match (&change.contents, key) {
                (
                    ElementContents::Chord(chord) | ElementContents::ChordWithNotes { chord, .. },
                    Some(key),
                ) => (
                    chord_symbol(*chord, key, numeral_standard).ok()?,
                    numeral_text(*chord),
                ),
                (ElementContents::InvertedChord { chord, inversion }, Some(key)) => (
                    inverted_chord_symbol(*chord, *inversion, key, numeral_standard).ok()?,
                    numeral_text(*chord),
                ),
                _ => return None,
            };
            Some(ChordView {
                offset_ticks: change.offset_ticks,
                symbol,
                numeral,
            })
        })
        .collect()
}

/// What turns an element's contents into written pitches and words.
struct Speller {
    key: Option<Key>,
    numeral_standard: Option<NumeralStandard>,
    home_octave: i8,
}

impl Speller {
    fn pitches_and_label(&self, contents: &ElementContents) -> (Vec<Pitch>, String) {
        let names = |pitches: &[Pitch]| -> String {
            pitches
                .iter()
                .map(|pitch: &Pitch| format!("{}{}", note_name_text(pitch.name()), pitch.octave()))
                .collect::<Vec<String>>()
                .join(" ")
        };
        match (contents, self.key) {
            (ElementContents::Chord(chord), Some(key)) => (
                chord_pitches(*chord, key, self.numeral_standard, self.home_octave)
                    .unwrap_or_default(),
                chord_symbol(*chord, key, self.numeral_standard)
                    .unwrap_or_else(|_| chord.to_string()),
            ),
            (ElementContents::InvertedChord { chord, inversion }, Some(key)) => (
                inverted_chord_pitches(
                    *chord,
                    key,
                    self.numeral_standard,
                    self.home_octave,
                    *inversion,
                )
                .unwrap_or_default(),
                inverted_chord_symbol(*chord, *inversion, key, self.numeral_standard)
                    .unwrap_or_else(|_| chord.to_string()),
            ),
            (ElementContents::ChordWithNotes { chord, notes }, Some(key)) => (
                notes.clone(),
                chord_symbol(*chord, key, self.numeral_standard)
                    .unwrap_or_else(|_| chord.to_string()),
            ),
            (ElementContents::Notes(notes), _) => (notes.clone(), names(notes)),
            (ElementContents::Cue(name), _) => (Vec::new(), name.clone()),
            (other, _) => (Vec::new(), crate::line_text::contents_text(other)),
        }
    }
}

fn element_view(
    line_element: &LineElement,
    line_element_index: usize,
    offset_ticks: u64,
    from_source: Option<&ElementContents>,
    speller: &Speller,
) -> ElementView {
    let element = line_element.element();
    let rest: bool = element.kind() == ElementKind::Rest;
    let contents: Option<&ElementContents> = if rest {
        None
    } else {
        line_element.contents().or(from_source)
    };
    let (pitches, label): (Vec<Pitch>, String) = contents.map_or_else(
        || (Vec::new(), String::new()),
        |contents| speller.pitches_and_label(contents),
    );
    let shift: i8 = line_element.octave_shift_octaves();
    let pitches: Vec<Pitch> = pitches
        .into_iter()
        .map(|pitch: Pitch| Pitch::new(pitch.name(), pitch.octave().saturating_add(shift)))
        .collect();
    let length_ticks: u64 = element.length().ticks();
    let plain: Option<NoteValue> = NoteValue::from_ticks(length_ticks);
    // A triplet note plays 2/3 of its written value: written = played × 3/2.
    let triplet_value: Option<NoteValue> =
        if plain.is_none() && (length_ticks * 3).is_multiple_of(2) {
            NoteValue::from_ticks(length_ticks * 3 / 2)
        } else {
            None
        };
    ElementView {
        offset_ticks,
        length_ticks,
        rest,
        from_source: !rest && line_element.contents().is_none(),
        pitches,
        label,
        value: plain.or(triplet_value),
        triplet: triplet_value.is_some(),
        marks: element.marks(),
        tied_to_next: element.tied_to_next(),
        line_element_index,
    }
}

fn source_views(
    source: &Line,
    bar_meters: &BarMeters,
    bar_index: usize,
    speller: &Speller,
) -> Vec<SourceView> {
    let starts: Vec<(u64, &LineElement)> = source
        .elements()
        .iter()
        .filter_map(|line_element: &LineElement| {
            let (index, offset_ticks): (usize, u64) =
                bar_meters.bar_at(line_element.element().start().ticks());
            (index == bar_index).then_some((offset_ticks, line_element))
        })
        .collect();
    starts
        .iter()
        .map(|(offset_ticks, line_element)| SourceView {
            offset_ticks: *offset_ticks,
            length_ticks: line_element.element().length().ticks(),
            label: line_element
                .contents()
                .map_or_else(String::new, |contents| {
                    speller.pitches_and_label(contents).1
                }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song_document::read_song;

    const DEMO: &str = include_str!("../../../songs_canonical/composer/composer_demo.song");

    #[test]
    fn entries_carry_their_key_and_chords() {
        let song: Song = read_song("composer_demo.song", DEMO).unwrap();
        let timeline: Timeline = Timeline::build(&song, &SongSource::read(DEMO));
        let names: Vec<&str> = timeline
            .entries
            .iter()
            .map(|entry| entry.section_name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["Count-in", "Verse", "Chorus", "Bridge", "Verse"]
        );
        let verse: &EntryView = &timeline.entries[1];
        assert_eq!(verse.bars.len(), 4);
        let symbols: Vec<&str> = verse
            .bars
            .iter()
            .map(|bar| bar.chords[0].symbol.as_str())
            .collect();
        assert_eq!(symbols, vec!["C", "Am", "Dm7", "G7"]);
        // The chorus plays in D major, so its IV is G.
        assert_eq!(timeline.entries[2].bars[0].chords[0].symbol, "G");
        // The bridge's chords come from its sample source.
        assert_eq!(timeline.entries[3].bars[1].chords[1].symbol, "G");
        let bridge_keys: &TrackBarView = &timeline.entries[3].bars[0].tracks[0];
        assert!(bridge_keys.elements[0].from_source);
        assert_eq!(bridge_keys.elements[0].label, "Am");
        // Count-in 1 bar, then the verse twice: 4 + 32 quarters.
        let quarter_ticks: u64 = TICKS_PER_QUARTER_NOTE;
        assert_eq!(timeline.entries[2].start_ticks, (4 + 32) * quarter_ticks);
        let place: TimelinePlace = timeline.place_at((4 + 16 + 5) * quarter_ticks).unwrap();
        assert_eq!(
            (place.entry_index, place.pass_number, place.bar_index),
            (1, 2, 1)
        );
    }
}
