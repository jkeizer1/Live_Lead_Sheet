//! A section in the other mode (Editor Draft, "A key override in F1"): a
//! key override keeps a section's major or minor, so playing a major
//! section in a minor key (or the reverse) needs a section of its own. This
//! writes that section as a close transposition of the original, each note
//! on its scale step and each chord of the old scale on the new scale's
//! chord of the same degree, for the musician to look at and edit; the
//! entry then plays the new section, and the original is left as it is.

use song_text::contents_text::{parse_chord, parse_interval};
use vocabulary::interval::{Interval, nearest_tonic};
use vocabulary::key::{Key, Mode};
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::pitch::Pitch;
use vocabulary::roman_chord::RomanChord;
use vocabulary::song::{Song, Track, TrackKind, TrackNotes};

use crate::arrange_editor::key_from_words;
use crate::chord_moves::chord_in_mode;
use crate::composition_edits::section_name_problem;
use crate::edits::{set_entry_section, with_numeral_standard};
use crate::song_source::{LineSource, SectionSource, SongSource};

/// Characters that end a word of a line (Song Notation, sections 3 and 4):
/// spaces, barlines, brackets, the colon and the marks.
const WORD_ENDS: &str = "|[](){}:,>'_^~<";

/// Writes a copy of the section that entry `entry_index` plays, named
/// `new_name`, in `new_key`, after the original section, and makes the
/// entry play it in its own key.
///
/// # Errors
///
/// When the name cannot be used, the entry or section is not in the text,
/// `new_key` is in the section's own mode, or a note has no interval the
/// notation writes in the new mode.
pub fn section_in_other_mode(
    text: &str,
    source: &SongSource,
    song: &Song,
    entry_index: usize,
    new_name: &str,
    new_key: Key,
) -> Result<String, String> {
    if let Some(problem) = section_name_problem(source, new_name) {
        return Err(problem.trim_end_matches('.').to_string());
    }
    let section_name: String = source
        .composition
        .as_ref()
        .and_then(|composition| composition.entries.get(entry_index))
        .map(|entry| entry.section_name.clone())
        .ok_or_else(|| format!("the composition has no entry {}", entry_index + 1))?;
    let section: &SectionSource = source
        .section(&section_name)
        .ok_or_else(|| format!("the song has no section \"{section_name}\""))?;
    let own_key: Key = section
        .key_words
        .as_deref()
        .and_then(key_from_words)
        .ok_or_else(|| format!("\"{section_name}\" has no key of its own to transpose from"))?;
    if own_key.mode() == new_key.mode() {
        return Err(format!(
            "\"{section_name}\" is already {}; give the entry the key instead",
            own_key.mode().text()
        ));
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let mut block: Vec<String> = Vec::new();
    for index in section.header_index..=section.last_index {
        let line: &str = lines
            .get(index)
            .ok_or_else(|| format!("section \"{section_name}\" runs past the text"))?;
        block.push((*line).to_string());
    }
    let first: usize = section.header_index;
    block[0] = block[0].replacen(
        &format!("section \"{section_name}\""),
        &format!("section \"{new_name}\""),
        1,
    );
    let own_words: String = format!("key {own_key}");
    for index in &section.setting_indices {
        let line: &mut String = &mut block[index - first];
        *line = line.replacen(&own_words, &format!("key {new_key}"), 1);
    }
    let change: ModeChange = ModeChange {
        from: own_key,
        to: new_key,
        numeral_standard: song.numeral_standard(),
    };
    for line_source in &section.lines {
        let pitched: bool = is_pitched(song, &line_source.track_name);
        for index in line_source.first_index..=line_source.last_index {
            let line: &mut String = &mut block[index - first];
            *line = change.line(line, pitched)?;
            if index == line_source.first_index && pitched {
                *line = change.tonic_octave(line, line_source)?;
            }
        }
    }
    let mut new_lines: Vec<String> = lines
        .iter()
        .map(|line: &&str| (*line).to_string())
        .collect();
    let after: usize = section.last_index + 1;
    let mut inserted: Vec<String> = vec![String::new()];
    inserted.extend(block);
    new_lines.splice(after..after, inserted);
    let written: String = new_lines.join("\n");
    let written_source: SongSource = SongSource::read(&written);
    let mut result: String =
        set_entry_section(&written, &written_source, entry_index, new_name, None)?;
    // Minor numerals need the song to say how it counts them (Song
    // Notation, section 5); the editor writes the key's own scale.
    if new_key.mode() == Mode::Minor && song.numeral_standard().is_none() {
        let result_source: SongSource = SongSource::read(&result);
        result = with_numeral_standard(&result, &result_source, "key_dependent");
    }
    Ok(result)
}

fn is_pitched(song: &Song, track_name: &str) -> bool {
    song.tracks().iter().any(|track: &Track| {
        track.name() == track_name
            && matches!(
                track.kind(),
                TrackKind::Voice {
                    notes: TrackNotes::Pitched,
                    ..
                }
            )
    })
}

/// From the section's own key to the new one.
struct ModeChange {
    from: Key,
    to: Key,
    numeral_standard: Option<NumeralStandard>,
}

impl ModeChange {
    /// One text line of a line, its intervals and chords moved; every other
    /// word as written. A MIDI note track's notes do not move.
    fn line(&self, line: &str, pitched: bool) -> Result<String, String> {
        let mut out: String = String::with_capacity(line.len());
        let mut word: String = String::new();
        let mut in_quotes: bool = false;
        for character in line.chars() {
            if character == '"' {
                in_quotes = !in_quotes;
            }
            if !in_quotes && (character.is_whitespace() || WORD_ENDS.contains(character)) {
                out.push_str(&self.word(&word, pitched)?);
                word.clear();
                out.push(character);
            } else {
                word.push(character);
            }
        }
        out.push_str(&self.word(&word, pitched)?);
        Ok(out)
    }

    fn word(&self, word: &str, pitched: bool) -> Result<String, String> {
        if word.is_empty() || word.starts_with('"') {
            return Ok(word.to_string());
        }
        if pitched && let Ok(interval) = parse_interval(word) {
            let moved: Interval = interval
                .in_mode(self.from.mode(), self.to.mode())
                .map_err(|error| format!("{word} in {}: {error}", self.to))?;
            return Ok(moved.to_string());
        }
        if let Ok(chord) = parse_chord(word) {
            let moved: RomanChord = chord_in_mode(
                chord,
                self.from.mode(),
                self.to.mode(),
                self.numeral_standard,
            );
            return Ok(moved.to_string());
        }
        Ok(word.to_string())
    }

    /// `tonic_octave n` for the new tonic nearest the old one, as a key
    /// override takes it (Song Notation, section 4).
    fn tonic_octave(&self, line: &str, line_source: &LineSource) -> Result<String, String> {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(position) = words.iter().position(|word: &&str| *word == "tonic_octave") else {
            return Ok(line.to_string());
        };
        let octave: i8 = words
            .get(position + 1)
            .and_then(|word: &&str| word.parse::<i8>().ok())
            .ok_or_else(|| {
                format!(
                    "line \"{}\": its tonic_octave is not a number",
                    line_source.track_name
                )
            })?;
        let new_tonic: Pitch =
            nearest_tonic(Pitch::new(self.from.tonic(), octave), self.to.tonic());
        Ok(line.replacen(
            &format!("tonic_octave {octave}"),
            &format!("tonic_octave {}", new_tonic.octave()),
            1,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song_document::read_song;
    use vocabulary::pitch::{Accidental, NoteLetter, NoteName};

    const OTHER_MODE: &str =
        include_str!("../../../songs_canonical/composer/other_mode_section.song");

    #[test]
    fn a_major_section_is_copied_into_a_minor_key_by_scale_step() {
        let song: Song = read_song("other_mode_section.song", OTHER_MODE).unwrap();
        let source: SongSource = SongSource::read(OTHER_MODE);
        let a_minor: Key = Key::new(
            NoteName::new(NoteLetter::A, Accidental::Natural),
            Mode::Minor,
        );
        let text: String =
            section_in_other_mode(OTHER_MODE, &source, &song, 1, "Verse (A minor)", a_minor)
                .unwrap();
        // The copy, after the original: each note on its scale step from A
        // (A1, the tonic nearest C2), the chromatic A4 kept, the chords on
        // their degrees with the dominant kept, the beat as it was.
        assert!(text.contains(
            "section \"Verse (A minor)\"  key A minor  meter 4/4\n  line \"Keys\"  | i:q iiø7 V7 VI |\n  line \"Bass\"  tonic_octave 1  | P1:e m3 P5 m6 m7 A4 P5 P1 |\n  line \"Beat\"  | C4:q> / / / |"
        ), "{text}");
        // The original is unchanged, and the second entry plays the copy.
        assert!(text.contains(
            "section \"Verse\"  key C major  meter 4/4\n  line \"Keys\"  | I:q ii7 V7 vi |"
        ));
        assert!(text.contains("  \"Verse\"\n  \"Verse (A minor)\"\nsong_composition_close"));
        // Minor numerals need the song to say how it counts them.
        assert!(text.contains("song \"Other mode\"\nchord_numeral_standard key_dependent\n"));
        let copied: Song = read_song("other_mode_section.song", &text).unwrap();
        assert_eq!(copied.sections()[1].key(), Some(a_minor));
        // The same mode is not a section of its own.
        let e_major: Key = Key::new(
            NoteName::new(NoteLetter::E, Accidental::Natural),
            Mode::Major,
        );
        assert!(section_in_other_mode(OTHER_MODE, &source, &song, 1, "Verse 2", e_major).is_err());
    }
}
