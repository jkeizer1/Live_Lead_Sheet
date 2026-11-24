//! The two libraries as the editor browses them (EFS-5.9): progressions in
//! their idiom and kind, rhythms by meter and group. The rows are a flat
//! list with group headings, so the browser moves through it with the
//! arrow keys and the screen draws it as a tree.

use song_text::parse_error::ParseError;
use song_text::progression_library::{ProgressionLibrary, parse_progression_library};
use song_text::rhythm_library::{RhythmLibraryEntry, parse_rhythm_library};
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::progression::Progression;
use vocabulary::rhythm_pattern::RhythmPattern;
use vocabulary::time_signature::TimeSignature;

/// Which library the browser shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Progressions,
    Rhythms,
}

/// One row of the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryRow {
    /// A group heading: `Blues · Full form`, `4/4 · Swing`.
    Heading(String),
    /// A progression, by its place in the library.
    Progression(usize),
    /// A rhythm, by its place in the library; `fits` is false when its
    /// meter is not the selected bars' meter, so it cannot be applied there.
    Rhythm { index: usize, fits: bool },
}

pub struct Libraries {
    progressions: ProgressionLibrary,
    rhythms: Vec<RhythmLibraryEntry>,
}

fn messages(errors: &[ParseError]) -> Vec<String> {
    errors.iter().map(ParseError::line_and_message).collect()
}

impl Libraries {
    /// Reads both libraries.
    ///
    /// # Errors
    ///
    /// Every mistake in either file, each with its file and line.
    pub fn read(
        progression_file_name: &str,
        progression_text: &str,
        rhythm_file_name: &str,
        rhythm_text: &str,
    ) -> Result<Libraries, Vec<String>> {
        let progressions: Result<ProgressionLibrary, Vec<ParseError>> =
            parse_progression_library(progression_file_name, progression_text);
        let rhythms: Result<Vec<RhythmLibraryEntry>, Vec<ParseError>> =
            parse_rhythm_library(rhythm_file_name, rhythm_text);
        match (progressions, rhythms) {
            (Ok(progressions), Ok(rhythms)) => Ok(Libraries {
                progressions,
                rhythms,
            }),
            (Err(errors), Ok(_)) | (Ok(_), Err(errors)) => Err(messages(&errors)),
            (Err(first), Err(second)) => {
                let mut all: Vec<String> = messages(&first);
                all.extend(messages(&second));
                Err(all)
            }
        }
    }

    #[must_use]
    pub fn progression(&self, index: usize) -> Option<&Progression> {
        self.progressions.progressions().get(index)
    }

    #[must_use]
    pub fn rhythm(&self, index: usize) -> Option<&RhythmPattern> {
        self.rhythms.get(index).map(RhythmLibraryEntry::pattern)
    }

    /// The rhythm's group, `Swing`.
    #[must_use]
    pub fn rhythm_group(&self, index: usize) -> Option<&str> {
        self.rhythms.get(index).map(RhythmLibraryEntry::group)
    }

    /// How the progression library counts minor-key numerals.
    #[must_use]
    pub fn numeral_standard(&self) -> Option<NumeralStandard> {
        self.progressions.numeral_standard()
    }

    /// The progressions under their idiom and kind, in the order the file
    /// first names each group.
    #[must_use]
    pub fn progression_rows(&self) -> Vec<LibraryRow> {
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        for (index, progression) in self.progressions.progressions().iter().enumerate() {
            let heading: String = format!("{} · {}", progression.idiom(), progression.kind());
            match groups.iter_mut().find(|(name, _)| *name == heading) {
                Some((_, members)) => members.push(index),
                None => groups.push((heading, vec![index])),
            }
        }
        let mut rows: Vec<LibraryRow> = Vec::new();
        for (heading, members) in groups {
            rows.push(LibraryRow::Heading(heading));
            rows.extend(members.into_iter().map(LibraryRow::Progression));
        }
        rows
    }

    /// The rhythms under their meter and group; those in `meter` first, as
    /// they are the ones that can be applied to the selected bars.
    #[must_use]
    pub fn rhythm_rows(&self, meter: Option<TimeSignature>) -> Vec<LibraryRow> {
        let mut groups: Vec<(TimeSignature, String, Vec<usize>)> = Vec::new();
        for (index, entry) in self.rhythms.iter().enumerate() {
            let rhythm_meter: TimeSignature = entry.pattern().time_signature();
            match groups.iter_mut().find(|(group_meter, group, _)| {
                *group_meter == rhythm_meter && group == entry.group()
            }) {
                Some((_, _, members)) => members.push(index),
                None => groups.push((rhythm_meter, entry.group().to_string(), vec![index])),
            }
        }
        // Stable: the file's order is kept within the fitting and the
        // other groups.
        groups.sort_by_key(|(group_meter, _, _)| Some(*group_meter) != meter);
        let mut rows: Vec<LibraryRow> = Vec::new();
        for (group_meter, group, members) in groups {
            rows.push(LibraryRow::Heading(format!(
                "{}/{} · {group}",
                group_meter.pulses_per_bar(),
                group_meter.pulses_per_whole_note()
            )));
            let fits: bool = meter.is_none_or(|bar_meter: TimeSignature| bar_meter == group_meter);
            rows.extend(
                members
                    .into_iter()
                    .map(|index: usize| LibraryRow::Rhythm { index, fits }),
            );
        }
        rows
    }

    /// The rows of `tab`.
    #[must_use]
    pub fn rows(&self, tab: LibraryTab, meter: Option<TimeSignature>) -> Vec<LibraryRow> {
        match tab {
            LibraryTab::Progressions => self.progression_rows(),
            LibraryTab::Rhythms => self.rhythm_rows(meter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRESSIONS: &str =
        include_str!("../../../songs_canonical/composer/composer_progressions.library");
    const RHYTHMS: &str =
        include_str!("../../../songs_canonical/composer/composer_rhythms.library");

    #[test]
    fn rows_are_grouped_and_fitting_rhythms_come_first() {
        let libraries: Libraries =
            Libraries::read("p.library", PROGRESSIONS, "r.library", RHYTHMS).unwrap();
        let rows: Vec<LibraryRow> = libraries.progression_rows();
        assert_eq!(rows[0], LibraryRow::Heading(String::from("Jazz · Cadence")));
        assert_eq!(rows[1], LibraryRow::Progression(0));
        assert_eq!(rows[2], LibraryRow::Progression(2));
        let three_four: TimeSignature = TimeSignature::new(3, 4).unwrap();
        let rhythm_rows: Vec<LibraryRow> = libraries.rhythm_rows(Some(three_four));
        assert_eq!(
            rhythm_rows[0],
            LibraryRow::Heading(String::from("3/4 · Pulses"))
        );
        assert_eq!(
            rhythm_rows[1],
            LibraryRow::Rhythm {
                index: 4,
                fits: true
            }
        );
        assert!(rhythm_rows.contains(&LibraryRow::Rhythm {
            index: 0,
            fits: false
        }));
    }
}
