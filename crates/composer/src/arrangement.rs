//! F1 (Arrange Song Sections) as rows: one per line of the
//! `song_composition` block, in the order written: the song's start, its
//! key and tempo changes, and its entries. Each row says which cells it
//! has and, for an entry, where the key and tempo it plays come from. What
//! an entry plays (its key, tempo, bars and chords) is read from the
//! timeline, which is built from the song as the song file's reader made
//! it, so the rows show exactly what plays.

use vocabulary::song::{Section, Song};

use crate::composition_edits::holds_only_rests;
use crate::song_source::{CompositionRowSource, SongSource};
use crate::timeline::Timeline;

/// A cell of a row, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrangeColumn {
    Name,
    Plays,
    Key,
    Meter,
    Tempo,
}

/// Where the key or tempo an entry plays comes from: the entry's own line,
/// the section's own settings, else the composition (its start, or the last
/// change line above the entry). The song file's reader takes them in this
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Entry,
    Section,
    Composition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRow {
    pub key_words: Option<String>,
    pub meter_words: Option<String>,
    pub tempo_words: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRow {
    /// The change's text line.
    pub line_index: usize,
    pub key_words: Option<String>,
    pub tempo_words: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRow {
    /// The entry's place in the composition, and in `Timeline::entries`.
    pub entry_index: usize,
    pub section_name: String,
    pub key_origin: Origin,
    pub tempo_origin: Origin,
    /// The entry's own tempo as written, when it has one.
    pub entry_tempo_words: Option<String>,
    /// Whether F1 may change the section's meter: only while the section
    /// writes a meter of its own and holds nothing but rests.
    pub meter_editable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrangeRow {
    Start(StartRow),
    Change(ChangeRow),
    Entry(EntryRow),
}

impl ArrangeRow {
    /// The cells the row has. The start has no name and no repeat; a
    /// change line has no meter (the meter belongs to each section).
    #[must_use]
    pub const fn columns(&self) -> &'static [ArrangeColumn] {
        match self {
            ArrangeRow::Start(_) => &[
                ArrangeColumn::Key,
                ArrangeColumn::Meter,
                ArrangeColumn::Tempo,
            ],
            ArrangeRow::Change(_) => &[
                ArrangeColumn::Name,
                ArrangeColumn::Key,
                ArrangeColumn::Tempo,
            ],
            ArrangeRow::Entry(_) => &[
                ArrangeColumn::Name,
                ArrangeColumn::Plays,
                ArrangeColumn::Key,
                ArrangeColumn::Meter,
                ArrangeColumn::Tempo,
            ],
        }
    }
}

/// `key section` on a song composition entry: play the section's own key.
pub const SECTION_KEY_WORD: &str = "section";

/// The rows of the composition as written in `source`, for `song` (as
/// read from the same text) and its `timeline`.
#[must_use]
pub fn arrangement_rows(source: &SongSource, song: &Song, timeline: &Timeline) -> Vec<ArrangeRow> {
    let Some(composition) = source.composition.as_ref() else {
        return Vec::new();
    };
    // A key line between entries overrides the entries below it (Song
    // Notation, section 5).
    let mut key_line_above: bool = false;
    composition
        .rows
        .iter()
        .filter_map(|row: &CompositionRowSource| match row {
            CompositionRowSource::Start => Some(ArrangeRow::Start(StartRow {
                key_words: composition.start_key_words.clone(),
                meter_words: composition.start_meter_words.clone(),
                tempo_words: composition.start_tempo_words.clone(),
            })),
            CompositionRowSource::Change(change) => {
                key_line_above |= change.key_words.is_some();
                Some(ArrangeRow::Change(ChangeRow {
                    line_index: change.index,
                    key_words: change.key_words.clone(),
                    tempo_words: change.tempo_words.clone(),
                }))
            }
            CompositionRowSource::Entry(entry_index) => {
                let written = composition.entries.get(*entry_index)?;
                let section = source.section(&written.section_name);
                let origin = |on_entry: bool, on_section: bool| {
                    if on_entry {
                        Origin::Entry
                    } else if on_section {
                        Origin::Section
                    } else {
                        Origin::Composition
                    }
                };
                let built: Option<&Section> = song
                    .sections()
                    .iter()
                    .find(|built: &&Section| built.name() == written.section_name);
                let _ = timeline.entries.get(*entry_index)?;
                Some(ArrangeRow::Entry(EntryRow {
                    entry_index: *entry_index,
                    section_name: written.section_name.clone(),
                    // `key section` plays the section's own key; else the
                    // entry's own, else a key line above, else the section's.
                    key_origin: match written.key_words.as_deref() {
                        Some(SECTION_KEY_WORD) => Origin::Section,
                        Some(_) => Origin::Entry,
                        None if key_line_above => Origin::Composition,
                        None => origin(
                            false,
                            section.is_some_and(|section| section.key_words.is_some()),
                        ),
                    },
                    tempo_origin: origin(
                        written.tempo_words.is_some(),
                        section.is_some_and(|section| section.tempo_words.is_some()),
                    ),
                    entry_tempo_words: written.tempo_words.clone(),
                    meter_editable: section.is_some_and(|section| section.meter_words.is_some())
                        && built.is_some_and(holds_only_rests),
                }))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song_document::read_song;
    use crate::timeline::EntryView;

    const ARRANGE: &str = include_str!("../../../songs_canonical/composer/arrange_song.song");

    /// The key and tempo each entry plays follow the song file's reader,
    /// the same one the text player uses: the entry's own, else the
    /// section's own, else the composition's current (its start, or the
    /// last change line above).
    #[test]
    fn each_entry_plays_the_key_and_tempo_the_song_file_gives_it() {
        let song: Song = read_song("arrange_song.song", ARRANGE).unwrap();
        let source: SongSource = SongSource::read(ARRANGE);
        let timeline: Timeline = Timeline::build(&song, &source);
        let played: Vec<(String, String, u64)> = timeline
            .entries
            .iter()
            .map(|entry: &EntryView| {
                (
                    entry.section_name.clone(),
                    entry.key.map_or_else(String::new, |key| key.to_string()),
                    entry.milli_beats_per_minute,
                )
            })
            .collect();
        assert_eq!(
            played,
            vec![
                // From the start.
                (String::from("Intro"), String::from("G major"), 100_000),
                // The entry's own key; the start's tempo.
                (String::from("Riff"), String::from("A major"), 100_000),
                // The change line's key, an override of the section's own,
                // and its tempo.
                (String::from("Riff"), String::from("E major"), 120_000),
                // The change line's key; the section's own tempo wins.
                (String::from("Bridge"), String::from("E major"), 90_000),
            ]
        );
        let origins: Vec<(Origin, Origin)> = arrangement_rows(&source, &song, &timeline)
            .iter()
            .filter_map(|row: &ArrangeRow| match row {
                ArrangeRow::Entry(entry) => Some((entry.key_origin, entry.tempo_origin)),
                _ => None,
            })
            .collect();
        assert_eq!(
            origins,
            vec![
                (Origin::Section, Origin::Composition),
                (Origin::Entry, Origin::Composition),
                (Origin::Composition, Origin::Composition),
                (Origin::Composition, Origin::Section),
            ]
        );
    }
}
