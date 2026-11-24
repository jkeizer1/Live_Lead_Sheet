//! Edits to a song's text: each takes the text and its outline and gives the
//! new text, changing only the text lines the edit is about. The document
//! reads the result back before it accepts it (`SongDocument::replace`).

use crate::song_source::{
    CompositionSource, EntrySource, LineSource, SectionSource, SongSource, entry_text, indentation,
    replace_setting,
};

/// The text as text lines, edited, and joined again with the file's
/// final newline kept.
pub(crate) struct TextLines {
    pub(crate) lines: Vec<String>,
    ends_with_newline: bool,
}

impl TextLines {
    pub(crate) fn of(text: &str) -> TextLines {
        TextLines {
            lines: text.lines().map(str::to_string).collect(),
            ends_with_newline: text.ends_with('\n'),
        }
    }

    pub(crate) fn joined(&self) -> String {
        let mut text: String = self.lines.join("\n");
        if self.ends_with_newline {
            text.push('\n');
        }
        text
    }

    fn line(&self, index: usize) -> Result<&str, String> {
        self.lines
            .get(index)
            .map(String::as_str)
            .ok_or_else(|| format!("the song's text has no line {}", index + 1))
    }
}

fn composition(source: &SongSource) -> Result<&CompositionSource, String> {
    source
        .composition
        .as_ref()
        .ok_or_else(|| String::from("the song has no song_composition block"))
}

fn entry(source: &SongSource, entry_index: usize) -> Result<&EntrySource, String> {
    composition(source)?
        .entries
        .get(entry_index)
        .ok_or_else(|| format!("the composition has no entry {}", entry_index + 1))
}

fn section<'a>(source: &'a SongSource, name: &str) -> Result<&'a SectionSource, String> {
    source
        .section(name)
        .ok_or_else(|| format!("the song has no section \"{name}\""))
}

/// Rewrites one entry's text line.
fn rewrite_entry(
    text: &str,
    source: &SongSource,
    entry_index: usize,
    change: impl FnOnce(&mut EntrySource),
) -> Result<String, String> {
    let mut written: EntrySource = entry(source, entry_index)?.clone();
    let mut lines: TextLines = TextLines::of(text);
    let indent: String = indentation(lines.line(written.index)?).to_string();
    change(&mut written);
    lines.lines[written.index] = entry_text(&indent, &written);
    Ok(lines.joined())
}

/// The entry plays `repeat_count` times; 0 leaves it out (a mute the
/// composition keeps, EFS-5.2b).
///
/// # Errors
///
/// When the entry does not exist.
pub fn set_entry_repeat(
    text: &str,
    source: &SongSource,
    entry_index: usize,
    repeat_count: u32,
) -> Result<String, String> {
    rewrite_entry(text, source, entry_index, |entry: &mut EntrySource| {
        entry.repeat_count = repeat_count;
    })
}

/// The entry's own key (`None` takes the section's or the composition's).
///
/// # Errors
///
/// When the entry does not exist.
pub fn set_entry_key(
    text: &str,
    source: &SongSource,
    entry_index: usize,
    key_words: Option<String>,
) -> Result<String, String> {
    rewrite_entry(text, source, entry_index, |entry: &mut EntrySource| {
        entry.key_words = key_words;
    })
}

/// The entry's own tempo, as the song writes it (`132 beats/min`).
///
/// # Errors
///
/// When the entry does not exist.
pub fn set_entry_tempo(
    text: &str,
    source: &SongSource,
    entry_index: usize,
    tempo_words: Option<String>,
) -> Result<String, String> {
    rewrite_entry(text, source, entry_index, |entry: &mut EntrySource| {
        entry.tempo_words = tempo_words;
    })
}

/// Moves the entry `delta` places later (earlier when negative).
///
/// # Errors
///
/// When the entry does not exist or would move past either end.
pub fn move_entry(
    text: &str,
    source: &SongSource,
    entry_index: usize,
    delta: isize,
) -> Result<String, String> {
    let entries: &[EntrySource] = &composition(source)?.entries;
    let target: usize = entry_index
        .checked_add_signed(delta)
        .filter(|target: &usize| *target < entries.len())
        .ok_or_else(|| String::from("the entry is already at that end of the composition"))?;
    let from: usize = entry(source, entry_index)?.index;
    let to: usize = entries[target].index;
    let mut lines: TextLines = TextLines::of(text);
    lines.lines.swap(from, to);
    Ok(lines.joined())
}

/// Writes the entry again just after itself, so the section plays twice in
/// a row as two entries (AABA forms are built this way).
///
/// # Errors
///
/// When the entry does not exist.
pub fn duplicate_entry(
    text: &str,
    source: &SongSource,
    entry_index: usize,
) -> Result<String, String> {
    let index: usize = entry(source, entry_index)?.index;
    let mut lines: TextLines = TextLines::of(text);
    let copy: String = lines.line(index)?.to_string();
    lines.lines.insert(index + 1, copy);
    Ok(lines.joined())
}

/// Takes the entry out of the composition. The section's definition stays.
///
/// # Errors
///
/// When the entry does not exist or is the only one.
pub fn remove_entry(text: &str, source: &SongSource, entry_index: usize) -> Result<String, String> {
    if composition(source)?.entries.len() <= 1 {
        return Err(String::from("the composition needs at least one entry"));
    }
    let index: usize = entry(source, entry_index)?.index;
    let mut lines: TextLines = TextLines::of(text);
    lines.lines.remove(index);
    Ok(lines.joined())
}

/// Sets a section's own `key` or `tempo` setting, writing it on the first
/// setting line when the section has none yet. `word_count` is how many
/// words the old value has (`C major` 2, the tempo's number 1).
fn set_section_setting(
    text: &str,
    source: &SongSource,
    section_name: &str,
    keyword: &str,
    word_count: usize,
    value: &str,
) -> Result<String, String> {
    let section: &SectionSource = section(source, section_name)?;
    let mut lines: TextLines = TextLines::of(text);
    for index in &section.setting_indices {
        if let Some(changed) = replace_setting(lines.line(*index)?, keyword, word_count, value) {
            lines.lines[*index] = changed;
            return Ok(lines.joined());
        }
    }
    if let Some(index) = section
        .setting_indices
        .iter()
        .find(|index: &&usize| **index != section.header_index)
    {
        let line: String = format!("{}  {keyword} {value}", lines.line(*index)?.trim_end());
        lines.lines[*index] = line;
    } else {
        let header: String = lines.line(section.header_index)?.to_string();
        let indent: String = format!("{}  ", indentation(&header));
        lines.lines.insert(
            section.header_index + 1,
            format!("{indent}{keyword} {value}"),
        );
    }
    Ok(lines.joined())
}

/// The section's own key, as the song writes it (`D major`).
///
/// # Errors
///
/// When the section does not exist.
pub fn set_section_key(
    text: &str,
    source: &SongSource,
    section_name: &str,
    key_words: &str,
) -> Result<String, String> {
    set_section_setting(text, source, section_name, "key", 2, key_words)
}

/// The section's own tempo number, keeping the unit it writes; a section
/// with no tempo gets `tempo n beats/min`.
///
/// # Errors
///
/// When the section does not exist.
pub fn set_section_tempo(
    text: &str,
    source: &SongSource,
    section_name: &str,
    beats_per_minute_text: &str,
) -> Result<String, String> {
    let section_source: &SectionSource = section(source, section_name)?;
    let lines: TextLines = TextLines::of(text);
    let has_tempo: bool = section_source.setting_indices.iter().any(|index: &usize| {
        lines
            .line(*index)
            .is_ok_and(|line: &str| line.split_whitespace().any(|word: &str| word == "tempo"))
    });
    if has_tempo {
        set_section_setting(
            text,
            source,
            section_name,
            "tempo",
            1,
            beats_per_minute_text,
        )
    } else {
        set_section_setting(
            text,
            source,
            section_name,
            "tempo",
            1,
            &format!("{beats_per_minute_text} beats/min"),
        )
    }
}

/// Which line of a section an edit writes: a track's voice layer, or its
/// sample source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineChoice<'a> {
    pub track_name: &'a str,
    pub layer: u8,
    pub sample_source: bool,
}

impl LineChoice<'_> {
    fn matches(&self, line: &LineSource) -> bool {
        line.track_name == self.track_name
            && line.layer() == self.layer
            && line.sample_source == self.sample_source
            && !line.dynamics
    }

    /// `line "Keys"`, with `voice_layer n` and `sample_source` as needed.
    #[must_use]
    pub fn header(&self) -> String {
        let mut words: Vec<String> = vec![format!("line \"{}\"", self.track_name)];
        if self.layer != 1 {
            words.push(format!("voice_layer {}", self.layer));
        }
        if self.sample_source {
            words.push(String::from("sample_source"));
        }
        words.join("  ")
    }
}

/// The indentation lines take in a section: that of its first line, else
/// two spaces more than the header.
#[must_use]
pub fn line_indent(text: &str, source: &SongSource, section_name: &str) -> String {
    let lines: TextLines = TextLines::of(text);
    let Some(section) = source.section(section_name) else {
        return String::from("  ");
    };
    if let Some(first) = section.lines.first()
        && let Ok(line) = lines.line(first.first_index)
    {
        return indentation(line).to_string();
    }
    lines.line(section.header_index).map_or_else(
        |_| String::from("  "),
        |header: &str| format!("{}  ", indentation(header)),
    )
}

/// Replaces the chosen line of a section with `new_lines`, or adds it after
/// the section's last line. A source line goes just before its rhythm
/// line, as the notation shows it.
///
/// # Errors
///
/// When the section does not exist.
pub fn replace_line(
    text: &str,
    source: &SongSource,
    section_name: &str,
    choice: LineChoice<'_>,
    new_lines: Vec<String>,
) -> Result<String, String> {
    let section: &SectionSource = section(source, section_name)?;
    let mut lines: TextLines = TextLines::of(text);
    if let Some(existing) = section
        .lines
        .iter()
        .find(|line: &&LineSource| choice.matches(line))
    {
        lines
            .lines
            .splice(existing.first_index..=existing.last_index, new_lines);
        return Ok(lines.joined());
    }
    let rhythm_line: Option<&LineSource> = section.lines.iter().find(|line: &&LineSource| {
        line.track_name == choice.track_name
            && line.layer() == choice.layer
            && !line.sample_source
            && !line.dynamics
    });
    let at: usize = match (choice.sample_source, rhythm_line) {
        (true, Some(rhythm)) => rhythm.first_index,
        _ => {
            section
                .lines
                .last()
                .map_or(section.last_index, |line: &LineSource| line.last_index)
                + 1
        }
    };
    lines.lines.splice(at..at, new_lines);
    Ok(lines.joined())
}

/// Removes the chosen line from a section, if it has it.
///
/// # Errors
///
/// When the section does not exist.
pub fn remove_line(
    text: &str,
    source: &SongSource,
    section_name: &str,
    choice: LineChoice<'_>,
) -> Result<String, String> {
    let section: &SectionSource = section(source, section_name)?;
    let mut lines: TextLines = TextLines::of(text);
    if let Some(existing) = section
        .lines
        .iter()
        .find(|line: &&LineSource| choice.matches(line))
    {
        lines
            .lines
            .drain(existing.first_index..=existing.last_index);
    }
    Ok(lines.joined())
}

/// Adds a section block at the end of the file and an entry for it in the
/// composition after `after_entry_index`.
///
/// # Errors
///
/// When the song has no composition or the section name is taken.
pub fn add_section(
    text: &str,
    source: &SongSource,
    after_entry_index: usize,
    section_name: &str,
    block_lines: Vec<String>,
) -> Result<String, String> {
    if source.section(section_name).is_some() {
        return Err(format!("the song already has a section \"{section_name}\""));
    }
    let composition: &CompositionSource = composition(source)?;
    let entry_line_index: usize = composition
        .entries
        .get(after_entry_index)
        .map_or(composition.close_index, |entry: &EntrySource| {
            entry.index + 1
        });
    let mut lines: TextLines = TextLines::of(text);
    let indent: String = composition
        .entries
        .first()
        .and_then(|entry: &EntrySource| lines.line(entry.index).ok())
        .map_or_else(
            || String::from("  "),
            |line: &str| indentation(line).to_string(),
        );
    lines.lines.push(String::new());
    lines.lines.extend(block_lines);
    lines
        .lines
        .insert(entry_line_index, format!("{indent}\"{section_name}\""));
    Ok(lines.joined())
}

/// Writes `chord_numeral_standard` after the `song` line when the song has
/// none, so numerals in a minor key read the way the library wrote them.
#[must_use]
pub fn with_numeral_standard(text: &str, source: &SongSource, standard_word: &str) -> String {
    if source.numeral_standard_index.is_some() {
        return text.to_string();
    }
    let mut lines: TextLines = TextLines::of(text);
    let at: usize = source.song_index.map_or(0, |index: usize| index + 1);
    lines
        .lines
        .insert(at, format!("chord_numeral_standard {standard_word}"));
    lines.joined()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song_document::read_song;

    const DEMO: &str = include_str!("../../../songs_canonical/composer/composer_demo.song");

    fn reads(text: &str) {
        if let Err(messages) = read_song("composer_demo.song", text) {
            panic!("{}\n\n{text}", messages.join("\n"));
        }
    }

    #[test]
    fn composition_edits_read_back() {
        let source: SongSource = SongSource::read(DEMO);
        let repeated: String = set_entry_repeat(DEMO, &source, 1, 3).unwrap();
        assert!(repeated.contains("  \"Verse\"  repeat 3\n"));
        reads(&repeated);
        let moved: String = move_entry(DEMO, &source, 2, 1).unwrap();
        reads(&moved);
        let keyed: String =
            set_entry_key(DEMO, &source, 3, Some(String::from("Eb major"))).unwrap();
        assert!(keyed.contains("\"Bridge\"  key Eb major"));
        reads(&keyed);
        reads(&duplicate_entry(DEMO, &source, 2).unwrap());
        reads(&remove_entry(DEMO, &source, 4).unwrap());
    }

    #[test]
    fn section_settings_are_set_or_added() {
        let source: SongSource = SongSource::read(DEMO);
        let keyed: String = set_section_key(DEMO, &source, "Verse", "G major").unwrap();
        assert!(keyed.contains("  meter 4/4  key G major"));
        reads(&keyed);
        let tempo: String = set_section_tempo(DEMO, &source, "Count-in", "100").unwrap();
        assert!(tempo.contains("tempo 100 beats/min"));
        reads(&tempo);
    }

    #[test]
    fn a_line_is_replaced_or_added() {
        let source: SongSource = SongSource::read(DEMO);
        let choice: LineChoice = LineChoice {
            track_name: "Bass",
            layer: 1,
            sample_source: false,
        };
        let replaced: String = replace_line(
            DEMO,
            &source,
            "Chorus",
            choice,
            vec![String::from("  line \"Bass\"  | F2:w | G2:w |")],
        )
        .unwrap();
        assert!(replaced.contains("line \"Bass\"  | F2:w | G2:w |"));
        reads(&replaced);
        let added: String = replace_line(
            DEMO,
            &source,
            "Count-in",
            choice,
            vec![String::from("  line \"Bass\"  | C2:w |")],
        )
        .unwrap();
        reads(&added);
    }
}
