//! The edits F1 (Arrange Song Sections) makes: the `song_composition`
//! block's start, its change lines and its entries, and the few section
//! facts that belong to arranging (a section's name, a new section, the
//! meter of a section that holds only rests). Each takes the text and its
//! outline and gives the new text; the document reads it back with the song
//! file's reader before keeping it, so what is kept is exactly what the
//! text player plays.

use vocabulary::key::Key;
use vocabulary::rhythm_pattern::ElementKind;
use vocabulary::song::{Line, LineElement, Section, Song, Track, TrackKind};
use vocabulary::time_signature::TimeSignature;

use crate::edits::TextLines;
use crate::line_bars::rest_bar;
use crate::line_text::{WrittenBar, line_text};
use crate::song_source::{
    CompositionRowSource, CompositionSource, SongSource, indentation, replace_setting,
};

fn composition(source: &SongSource) -> Result<&CompositionSource, String> {
    source
        .composition
        .as_ref()
        .ok_or_else(|| String::from("the song has no song_composition block"))
}

/// The indentation the block's lines take: that of its first entry, else
/// two spaces.
fn block_indent(lines: &TextLines, composition: &CompositionSource) -> String {
    composition
        .entries
        .first()
        .and_then(|entry| lines.lines.get(entry.index))
        .or_else(|| {
            composition
                .start_indices
                .iter()
                .find(|index: &&usize| **index != composition.open_index)
                .and_then(|index: &usize| lines.lines.get(*index))
        })
        .map_or_else(
            || String::from("  "),
            |line: &String| indentation(line).to_string(),
        )
}

/// How many words a setting's value has: `C major` 2, `4/4` 1,
/// `120 beats/min` 2.
fn value_word_count(keyword: &str) -> usize {
    if keyword == "meter" { 1 } else { 2 }
}

/// The meter as the song writes it: `3/4`.
#[must_use]
pub fn meter_words(meter: TimeSignature) -> String {
    format!(
        "{}/{}",
        meter.pulses_per_bar(),
        meter.pulses_per_whole_note()
    )
}

/// A typed tempo made into what the song writes: `112.5` gives
/// `112.5 beats/min`. The number is held exactly by the reader, so only
/// whole thousandths of a beat are taken.
///
/// # Errors
///
/// When the text is not a number above 0 with at most three decimals.
pub fn tempo_words_from_typed(typed: &str) -> Result<String, String> {
    let number: &str = typed.trim();
    let wrong = || format!("\"{number}\" is not a tempo. Type beats per minute, e.g. 120 or 112.5");
    let (whole, decimals): (&str, &str) = number.split_once('.').unwrap_or((number, ""));
    let digits_only = |text: &str| {
        text.chars()
            .all(|character: char| character.is_ascii_digit())
    };
    if whole.is_empty() || !digits_only(whole) || !digits_only(decimals) {
        return Err(wrong());
    }
    if decimals.len() > 3 {
        return Err(format!(
            "\"{number}\" has more than three decimals. A tempo is held to a thousandth of a beat"
        ));
    }
    if whole.trim_start_matches('0').is_empty() && decimals.trim_matches('0').is_empty() {
        return Err(String::from("a tempo of 0 never plays. Use more than 0"));
    }
    Ok(format!("{number} beats/min"))
}

/// milli-beats/min → the number the song writes: 120000 → `120`,
/// 112500 → `112.5`.
#[must_use]
pub fn beats_per_minute_text(milli_beats_per_minute: u64) -> String {
    let whole: u64 = milli_beats_per_minute / 1000;
    let thousandths: u64 = milli_beats_per_minute % 1000;
    if thousandths == 0 {
        whole.to_string()
    } else {
        let decimals: String = format!("{thousandths:03}");
        format!("{whole}.{}", decimals.trim_end_matches('0'))
    }
}

/// The milli-beats per minute of tempo words written in beats/min, `None`
/// for any other unit.
#[must_use]
pub fn milli_beats_per_minute_of(tempo_words: &str) -> Option<u64> {
    let number: &str = tempo_words.strip_suffix(" beats/min")?;
    let (whole, decimals): (&str, &str) = number.split_once('.').unwrap_or((number, ""));
    if decimals.len() > 3 {
        return None;
    }
    let whole_beats: u64 = whole.parse().ok()?;
    // Decimals padded to thousandths: "5" is 500 thousandths.
    let thousandths: u64 = if decimals.is_empty() {
        0
    } else {
        format!("{decimals:0<3}").parse().ok()?
    };
    Some(whole_beats * 1000 + thousandths)
}

/// The song's starting `key`, `meter` or `tempo`, written in place.
///
/// # Errors
///
/// When the song has no composition or the start lacks the setting.
pub fn set_start_setting(
    text: &str,
    source: &SongSource,
    keyword: &str,
    value: &str,
) -> Result<String, String> {
    let composition: &CompositionSource = composition(source)?;
    let mut lines: TextLines = TextLines::of(text);
    for index in &composition.start_indices {
        let Some(line) = lines.lines.get(*index) else {
            continue;
        };
        if let Some(changed) = replace_setting(line, keyword, value_word_count(keyword), value) {
            lines.lines[*index] = changed;
            return Ok(lines.joined());
        }
    }
    Err(format!(
        "the composition's first line has no {keyword} to change. Write key, meter and tempo there"
    ))
}

/// A change line written from its values: `key E minor  tempo 120 beats/min`.
fn change_text(indent: &str, key_words: Option<&str>, tempo_words: Option<&str>) -> String {
    let mut words: Vec<String> = Vec::new();
    if let Some(key) = key_words {
        words.push(format!("key {key}"));
    }
    if let Some(tempo) = tempo_words {
        words.push(format!("tempo {tempo}"));
    }
    format!("{indent}{}", words.join("  "))
}

/// The change line at text line `line_index` with new values; with neither
/// a key nor a tempo the line goes, since it would change nothing.
///
/// # Errors
///
/// When there is no change line there.
pub fn set_change(
    text: &str,
    source: &SongSource,
    line_index: usize,
    key_words: Option<&str>,
    tempo_words: Option<&str>,
) -> Result<String, String> {
    let composition: &CompositionSource = composition(source)?;
    let is_change: bool = composition.rows.iter().any(|row: &CompositionRowSource| {
        matches!(row, CompositionRowSource::Change(change) if change.index == line_index)
    });
    if !is_change {
        return Err(String::from("that row is not a key or tempo change"));
    }
    let mut lines: TextLines = TextLines::of(text);
    if key_words.is_none() && tempo_words.is_none() {
        lines.lines.remove(line_index);
        return Ok(lines.joined());
    }
    let indent: String = indentation(&lines.lines[line_index]).to_string();
    lines.lines[line_index] = change_text(&indent, key_words, tempo_words);
    Ok(lines.joined())
}

/// The composition's rows other than the start: a new line may go after
/// the start only if it is an entry, since a settings line before the
/// first entry belongs to the start.
fn insertion_line(composition: &CompositionSource, row: usize) -> Result<usize, String> {
    let written: &CompositionRowSource = composition
        .rows
        .get(row)
        .ok_or_else(|| format!("the composition has no row {}", row + 1))?;
    Ok(composition.row_line_index(written) + 1)
}

/// A key change line after row `row`, for every entry below it.
///
/// # Errors
///
/// When the row is the start (a change goes after an entry).
pub fn add_change_after(
    text: &str,
    source: &SongSource,
    row: usize,
    key: Key,
) -> Result<String, String> {
    let composition: &CompositionSource = composition(source)?;
    if matches!(composition.rows.get(row), Some(CompositionRowSource::Start)) {
        return Err(String::from(
            "a key or tempo change goes after an entry. Select an entry, then add the change",
        ));
    }
    let at: usize = insertion_line(composition, row)?;
    let mut lines: TextLines = TextLines::of(text);
    let indent: String = block_indent(&lines, composition);
    lines
        .lines
        .insert(at, change_text(&indent, Some(&key.to_string()), None));
    Ok(lines.joined())
}

/// An entry playing `section_name` once, after row `row`.
///
/// # Errors
///
/// When the song has no such section.
pub fn add_entry_after(
    text: &str,
    source: &SongSource,
    row: usize,
    section_name: &str,
) -> Result<String, String> {
    if source.section(section_name).is_none() {
        return Err(format!("the song has no section \"{section_name}\""));
    }
    let composition: &CompositionSource = composition(source)?;
    let at: usize = insertion_line(composition, row)?;
    let mut lines: TextLines = TextLines::of(text);
    let indent: String = block_indent(&lines, composition);
    lines
        .lines
        .insert(at, format!("{indent}\"{section_name}\""));
    Ok(lines.joined())
}

/// Takes the change line at text line `line_index` out.
///
/// # Errors
///
/// When there is no change line there.
pub fn remove_change(text: &str, source: &SongSource, line_index: usize) -> Result<String, String> {
    set_change(text, source, line_index, None, None)
}

/// Moves row `row` `delta` rows later (earlier when negative), past the
/// row next to it. The start stays first, and the row after the start must
/// be an entry, since a settings line there would belong to the start.
///
/// # Errors
///
/// When the move would leave either rule broken or pass either end.
pub fn move_row(
    text: &str,
    source: &SongSource,
    row: usize,
    delta: isize,
) -> Result<String, String> {
    let composition: &CompositionSource = composition(source)?;
    let rows: &[CompositionRowSource] = &composition.rows;
    if matches!(rows.get(row), Some(CompositionRowSource::Start) | None) {
        return Err(String::from(
            "the song's starting key, meter and tempo stay first",
        ));
    }
    let target: usize = row
        .checked_add_signed(delta)
        .filter(|target: &usize| *target >= 1 && *target < rows.len())
        .ok_or_else(|| String::from("the row is already at that end of the composition"))?;
    let mut order: Vec<&CompositionRowSource> = rows.iter().collect();
    order.swap(row, target);
    if matches!(order.get(1), Some(CompositionRowSource::Change(_))) {
        return Err(String::from(
            "a key or tempo change cannot come before the first entry. Change the song's start instead",
        ));
    }
    let from: usize = composition.row_line_index(&rows[row]);
    let to: usize = composition.row_line_index(&rows[target]);
    let mut lines: TextLines = TextLines::of(text);
    lines.lines.swap(from, to);
    Ok(lines.joined())
}

/// Why a section name cannot be used, if it cannot.
#[must_use]
pub fn section_name_problem(source: &SongSource, name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return Some(String::from("Type a name for the section."));
    }
    if name != name.trim() {
        return Some(String::from(
            "A section name cannot start or end with a space.",
        ));
    }
    if name.contains('"') {
        return Some(String::from("A section name cannot hold a \" sign."));
    }
    if source.section(name).is_some() {
        return Some(format!(
            "The song already has a section \"{name}\". Choose another name."
        ));
    }
    None
}

/// The section called `old_name` is called `new_name`: its definition and
/// every entry that plays it.
///
/// # Errors
///
/// When there is no such section, or the new name cannot be used.
pub fn rename_section(
    text: &str,
    source: &SongSource,
    old_name: &str,
    new_name: &str,
) -> Result<String, String> {
    let section = source
        .section(old_name)
        .ok_or_else(|| format!("the song has no section \"{old_name}\""))?;
    if old_name == new_name {
        return Ok(text.to_string());
    }
    if let Some(problem) = section_name_problem(source, new_name) {
        return Err(problem.trim_end_matches('.').to_string());
    }
    let old_quoted: String = format!("\"{old_name}\"");
    let new_quoted: String = format!("\"{new_name}\"");
    let mut lines: TextLines = TextLines::of(text);
    let mut indices: Vec<usize> = vec![section.header_index];
    if let Some(composition) = source.composition.as_ref() {
        indices.extend(
            composition
                .entries
                .iter()
                .filter(|entry| entry.section_name == old_name)
                .map(|entry| entry.index),
        );
    }
    // The name is the first quoted word on each of these lines.
    for index in indices {
        if let Some(line) = lines.lines.get_mut(index) {
            *line = line.replacen(&old_quoted, &new_quoted, 1);
        }
    }
    Ok(lines.joined())
}

/// What the new-section dialog gives: the section's own facts. Its bars
/// are not among them; the Section Editor sets them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSection {
    pub name: String,
    pub meter: TimeSignature,
    /// `None`: the section plays in the composition's key.
    pub key: Option<Key>,
    /// `None`: the section plays at the composition's tempo.
    pub tempo_words: Option<String>,
    pub count_in: bool,
}

/// The track a new section's bar of rest goes on: the lead, else the first
/// voice track.
fn first_bar_track(song: &Song) -> Option<&Track> {
    let voices = || {
        song.tracks()
            .iter()
            .filter(|track: &&Track| matches!(track.kind(), TrackKind::Voice { .. }))
    };
    voices()
        .find(|track: &&Track| matches!(track.kind(), TrackKind::Voice { lead: true, .. }))
        .or_else(|| voices().next())
}

/// A new section at the end of the file, with one bar of rest on the lead
/// track (the song file needs a section to have a line), and an entry that
/// plays it once after row `row`.
///
/// # Errors
///
/// When the name cannot be used, or the song has no voice track.
pub fn add_new_section(
    text: &str,
    source: &SongSource,
    song: &Song,
    row: usize,
    new: &NewSection,
) -> Result<String, String> {
    if let Some(problem) = section_name_problem(source, &new.name) {
        return Err(problem.trim_end_matches('.').to_string());
    }
    let track: &Track = first_bar_track(song).ok_or_else(|| {
        String::from(
            "the song has no voice track for the section's bar. Add one in Song Settings (F6)",
        )
    })?;
    let composition: &CompositionSource = composition(source)?;
    let at: usize = insertion_line(composition, row)?;
    let mut lines: TextLines = TextLines::of(text);
    let indent: String = block_indent(&lines, composition);
    let mut header: String = format!("section \"{}\"", new.name);
    if new.count_in {
        header.push_str("  count-in");
    }
    let mut settings: Vec<String> = vec![format!("meter {}", meter_words(new.meter))];
    if let Some(key) = new.key {
        settings.push(format!("key {key}"));
    }
    if let Some(tempo) = &new.tempo_words {
        settings.push(format!("tempo {tempo}"));
    }
    let rest: Vec<WrittenBar> = vec![rest_bar(new.meter.bar_length().ticks())];
    let bar_lines: Vec<String> = line_text("  ", &format!("line \"{}\"", track.name()), &rest)
        .map_err(|error| error.to_string())?;
    lines.lines.insert(at, format!("{indent}\"{}\"", new.name));
    if lines
        .lines
        .last()
        .is_some_and(|last: &String| !last.trim().is_empty())
    {
        lines.lines.push(String::new());
    }
    lines.lines.push(header);
    lines.lines.push(format!("  {}", settings.join("  ")));
    lines.lines.extend(bar_lines);
    Ok(lines.joined())
}

/// Whether a section's lines hold only rests: nothing composed yet, so its
/// meter can change without changing music.
#[must_use]
pub fn holds_only_rests(section: &Section) -> bool {
    section.lines().iter().all(|line: &Line| {
        line.elements()
            .iter()
            .all(|element: &LineElement| element.element().kind() == ElementKind::Rest)
    })
}

/// The section's meter, for a section that holds only rests: the meter is
/// rewritten and every line is rewritten as the same number of bars of
/// rest in the new meter. A section with music keeps its meter here; its
/// bars are changed in the Section Editor.
///
/// # Errors
///
/// When the section has music, writes no meter of its own (a meter
/// pattern), or is not played by any entry.
pub fn set_section_meter(
    text: &str,
    source: &SongSource,
    song: &Song,
    section_name: &str,
    meter: TimeSignature,
) -> Result<String, String> {
    let section_source = source
        .section(section_name)
        .ok_or_else(|| format!("the song has no section \"{section_name}\""))?;
    if section_source.meter_words.is_none() {
        return Err(format!(
            "\"{section_name}\" takes its meters from a meter_control line. Change them in the Section Editor"
        ));
    }
    let section: &Section = song
        .sections()
        .iter()
        .find(|section: &&Section| section.name() == section_name)
        .ok_or_else(|| {
            format!("\"{section_name}\" plays in no entry, so its bars are not read. Give an entry repeat 1 first")
        })?;
    if !holds_only_rests(section) {
        return Err(format!(
            "\"{section_name}\" has music written for its meter. Change its bars in the Section Editor (F3)"
        ));
    }
    let bar_count: usize = usize::try_from(section.bar_count())
        .map_err(|_| format!("\"{section_name}\" has more bars than can be counted"))?;
    let new_words: String = meter_words(meter);
    let mut lines: TextLines = TextLines::of(text);
    let mut meter_written: bool = false;
    for index in &section_source.setting_indices {
        if let Some(line) = lines.lines.get(*index)
            && let Some(changed) = replace_setting(line, "meter", 1, &new_words)
        {
            lines.lines[*index] = changed;
            meter_written = true;
            break;
        }
    }
    if !meter_written {
        return Err(format!(
            "could not find the meter of \"{section_name}\" in the text"
        ));
    }
    let rests: Vec<WrittenBar> = (0..bar_count)
        .map(|_| rest_bar(meter.bar_length().ticks()))
        .collect();
    // Last line first, so the text lines of the lines above keep their
    // places when a rewritten line takes more or fewer text lines.
    for line in section_source.lines.iter().rev() {
        let first: &str = &lines.lines[line.first_index];
        let indent: String = indentation(first).to_string();
        let header: String = first
            .split('|')
            .next()
            .map_or_else(String::new, |words: &str| words.trim().to_string());
        let rewritten: Vec<String> =
            line_text(&indent, &header, &rests).map_err(|error| error.to_string())?;
        lines
            .lines
            .splice(line.first_index..=line.last_index, rewritten);
    }
    Ok(lines.joined())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song_document::read_song;
    use vocabulary::key::Mode;
    use vocabulary::pitch::{Accidental, NoteLetter, NoteName};

    const ARRANGE: &str = include_str!("../../../songs_canonical/composer/arrange_song.song");

    fn reads(text: &str) -> Song {
        match read_song("arrange_song.song", text) {
            Ok(song) => song,
            Err(messages) => panic!("{}\n\n{text}", messages.join("\n")),
        }
    }

    fn key(letter: NoteLetter, accidental: Accidental, mode: Mode) -> Key {
        Key::new(NoteName::new(letter, accidental), mode)
    }

    #[test]
    fn typed_tempos_are_checked_and_written_in_beats_per_minute() {
        assert_eq!(tempo_words_from_typed("112.5").unwrap(), "112.5 beats/min");
        assert_eq!(tempo_words_from_typed(" 90 ").unwrap(), "90 beats/min");
        assert!(tempo_words_from_typed("0").is_err());
        assert!(tempo_words_from_typed("1.2345").is_err());
        assert!(tempo_words_from_typed("fast").is_err());
        assert_eq!(milli_beats_per_minute_of("112.5 beats/min"), Some(112_500));
        assert_eq!(milli_beats_per_minute_of("2 beats/s"), None);
        assert_eq!(beats_per_minute_text(112_500), "112.5");
    }

    #[test]
    fn the_start_and_change_lines_are_rewritten() {
        let source: SongSource = SongSource::read(ARRANGE);
        let started: String = set_start_setting(ARRANGE, &source, "meter", "3/4").unwrap();
        assert!(started.contains("  key G major  meter 3/4  tempo 100 beats/min\n"));
        reads(&started);
        let changed: String =
            set_change(ARRANGE, &source, 21, Some("D major"), Some("120 beats/min")).unwrap();
        assert!(changed.contains("\n  key D major  tempo 120 beats/min\n"));
        reads(&changed);
        let removed: String = remove_change(ARRANGE, &source, 21).unwrap();
        assert!(!removed.contains("key E minor"));
        reads(&removed);
        assert!(set_change(ARRANGE, &source, 20, None, None).is_err());
    }

    #[test]
    fn rows_are_added_and_moved_within_the_rules() {
        let source: SongSource = SongSource::read(ARRANGE);
        let added: String = add_change_after(
            ARRANGE,
            &source,
            1,
            key(NoteLetter::B, Accidental::Flat, Mode::Major),
        )
        .unwrap();
        assert!(added.contains("  \"Intro\"  repeat 2\n  key Bb major\n  \"Riff\""));
        reads(&added);
        assert!(
            add_change_after(
                ARRANGE,
                &source,
                0,
                key(NoteLetter::C, Accidental::Natural, Mode::Major)
            )
            .is_err()
        );
        let entry: String = add_entry_after(ARRANGE, &source, 0, "Bridge").unwrap();
        assert!(entry.contains("100 beats/min\n  \"Bridge\"\n  \"Intro\""));
        reads(&entry);
        let moved: String = move_row(ARRANGE, &source, 3, -1).unwrap();
        assert!(moved.contains(
            "  \"Intro\"  repeat 2\n  key E minor  tempo 120 beats/min\n  \"Riff\"  key A major\n"
        ));
        reads(&moved);
        // The Intro cannot leave the change line first after the start.
        let source_moved: SongSource = SongSource::read(&moved);
        assert!(move_row(&moved, &source_moved, 1, 1).is_err());
        assert!(move_row(ARRANGE, &source, 1, -1).is_err());
    }

    #[test]
    fn a_section_is_renamed_with_every_entry_that_plays_it() {
        let source: SongSource = SongSource::read(ARRANGE);
        let renamed: String = rename_section(ARRANGE, &source, "Riff", "Chainsaw").unwrap();
        assert!(renamed.contains("  \"Chainsaw\"  key A major\n"));
        assert!(renamed.contains("120 beats/min\n  \"Chainsaw\"\n"));
        assert!(renamed.contains("section \"Chainsaw\"\n"));
        assert!(!renamed.contains("Riff"));
        reads(&renamed);
        assert!(rename_section(ARRANGE, &source, "Riff", "Bridge").is_err());
        assert!(rename_section(ARRANGE, &source, "Riff", "").is_err());
    }

    #[test]
    fn a_new_section_has_a_bar_of_rest_and_its_meter_follows_while_it_rests() {
        let source: SongSource = SongSource::read(ARRANGE);
        let song: Song = reads(ARRANGE);
        let new: NewSection = NewSection {
            name: String::from("Outro"),
            meter: TimeSignature::new(6, 8).unwrap(),
            key: Some(key(NoteLetter::E, Accidental::Natural, Mode::Minor)),
            tempo_words: Some(String::from("72.5 beats/min")),
            count_in: false,
        };
        let added: String = add_new_section(ARRANGE, &source, &song, 5, &new).unwrap();
        assert!(added.contains("  \"Bridge\"\n  \"Outro\"\nsong_composition_close"));
        assert!(added.ends_with(
            "section \"Outro\"\n  meter 6/8  key E minor  tempo 72.5 beats/min\n  line \"Keys\"  | r:h. |\n"
        ));
        let added_song: Song = reads(&added);
        let added_source: SongSource = SongSource::read(&added);
        let remetered: String = set_section_meter(
            &added,
            &added_source,
            &added_song,
            "Outro",
            TimeSignature::new(5, 4).unwrap(),
        )
        .unwrap();
        assert!(remetered.contains("  meter 5/4  key E minor"));
        assert!(
            remetered.contains("  line \"Keys\"  | r:w r:q |"),
            "{remetered}"
        );
        reads(&remetered);
        // The Riff has music, so its meter stays.
        assert!(
            set_section_meter(
                ARRANGE,
                &source,
                &song,
                "Riff",
                TimeSignature::new(3, 4).unwrap()
            )
            .is_err()
        );
        assert!(
            add_new_section(
                ARRANGE,
                &source,
                &song,
                5,
                &NewSection {
                    name: String::from("Riff"),
                    ..new
                }
            )
            .is_err()
        );
    }
}
