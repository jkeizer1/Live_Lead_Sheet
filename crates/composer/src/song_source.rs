//! Where things are in a song's text: the `song_composition` block and its
//! entries, each section's block, its setting lines and its lines. The
//! song file's own reader (`song_text::song_file`) decides what the text
//! means; this only finds where an edit goes, so the edit can change those
//! text lines and leave the rest of the file, comments included, as the
//! musician wrote it. Every edit is read back by the song file's reader,
//! so a mistake here is refused, never saved.

use song_text::text_line::{keyword_rest, quoted_prefix, without_comment};

/// A line of a section, as written: which track, which role, and the text
/// lines it covers (its `line` text line and the `|` lines that go on
/// with it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSource {
    pub track_name: String,
    /// `voice_layer n`, when written.
    pub voice_layer: Option<u8>,
    pub sample_source: bool,
    pub dynamics: bool,
    /// Index of the `line` text line (0 the file's first).
    pub first_index: usize,
    /// Index of the last text line that carries its bars.
    pub last_index: usize,
}

impl LineSource {
    /// The voice layer the line is: written, or 1.
    #[must_use]
    pub fn layer(&self) -> u8 {
        self.voice_layer.unwrap_or(1)
    }
}

/// A track's block: its `track "Name"` line and the lines under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackSource {
    pub name: String,
    /// Index of the `track "Name"` text line.
    pub header_index: usize,
    /// Index of the last text line of the block that is not blank or a
    /// comment.
    pub last_index: usize,
    pub type_index: Option<usize>,
    pub clef_index: Option<usize>,
    pub lead_index: Option<usize>,
    /// The `voice_layer` lines.
    pub voice_layer_indices: Vec<usize>,
}

/// A section's block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSource {
    pub name: String,
    /// Index of the `section "Name"` text line.
    pub header_index: usize,
    /// Index of the last text line of the block that is not blank or a
    /// comment.
    pub last_index: usize,
    /// Setting lines (`key`, `meter`, `tempo`, ...), the header included
    /// when it carries settings.
    pub setting_indices: Vec<usize>,
    pub lines: Vec<LineSource>,
}

/// An entry of the composition: `"Verse"  repeat 2  key D major`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySource {
    pub index: usize,
    pub section_name: String,
    pub repeat_count: u32,
    /// The words after the name other than `repeat n`, kept as written:
    /// the entry's own `key` and `tempo`.
    pub key_words: Option<String>,
    pub tempo_words: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionSource {
    pub open_index: usize,
    pub close_index: usize,
    /// The first settings line after `song_composition` (the starting key,
    /// meter and tempo).
    pub header_index: Option<usize>,
    pub entries: Vec<EntrySource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongSource {
    /// The `song "Name"` text line.
    pub song_index: Option<usize>,
    /// The `chord_numeral_standard` text line.
    pub numeral_standard_index: Option<usize>,
    pub composition: Option<CompositionSource>,
    pub tracks: Vec<TrackSource>,
    pub sections: Vec<SectionSource>,
}

impl SongSource {
    #[must_use]
    pub fn read(text: &str) -> SongSource {
        let mut source: SongSource = SongSource {
            song_index: None,
            numeral_standard_index: None,
            composition: None,
            tracks: Vec::new(),
            sections: Vec::new(),
        };
        let mut track: Option<TrackSource> = None;
        let mut section: Option<SectionSource> = None;
        let mut composition: Option<CompositionSource> = None;
        let mut in_composition: bool = false;
        for (index, raw_line) in text.lines().enumerate() {
            let line: &str = without_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }
            if in_composition {
                if keyword_rest(line, "song_composition_close").is_some() {
                    if let Some(open) = composition.as_mut() {
                        open.close_index = index;
                    }
                    in_composition = false;
                } else if let Some(open) = composition.as_mut() {
                    read_composition_line(open, index, line);
                }
                continue;
            }
            if keyword_rest(line, "song_composition").is_some() {
                close_track(&mut source, &mut track);
                close_section(&mut source, &mut section);
                in_composition = true;
                composition = Some(CompositionSource {
                    open_index: index,
                    close_index: index,
                    header_index: None,
                    entries: Vec::new(),
                });
            } else if keyword_rest(line, "song").is_some() {
                source.song_index = Some(index);
            } else if keyword_rest(line, "chord_numeral_standard").is_some() {
                source.numeral_standard_index = Some(index);
            } else if let Some(rest) = keyword_rest(line, "track") {
                close_track(&mut source, &mut track);
                close_section(&mut source, &mut section);
                track = Some(TrackSource {
                    name: quoted_prefix(rest).map_or("", |(name, _)| name).to_string(),
                    header_index: index,
                    last_index: index,
                    type_index: None,
                    clef_index: None,
                    lead_index: None,
                    voice_layer_indices: Vec::new(),
                });
            } else if let Some(rest) = keyword_rest(line, "section") {
                close_track(&mut source, &mut track);
                close_section(&mut source, &mut section);
                let (name, after): (&str, &str) = quoted_prefix(rest).unwrap_or(("", ""));
                section = Some(SectionSource {
                    name: name.to_string(),
                    header_index: index,
                    last_index: index,
                    setting_indices: if after.is_empty() {
                        Vec::new()
                    } else {
                        vec![index]
                    },
                    lines: Vec::new(),
                });
            } else if let Some(open) = section.as_mut() {
                read_section_line(open, index, line);
            } else if let Some(open) = track.as_mut() {
                read_track_line(open, index, line);
            }
        }
        close_track(&mut source, &mut track);
        close_section(&mut source, &mut section);
        source.composition = composition;
        source
    }

    #[must_use]
    pub fn track(&self, name: &str) -> Option<&TrackSource> {
        self.tracks
            .iter()
            .find(|track: &&TrackSource| track.name == name)
    }

    #[must_use]
    pub fn section(&self, name: &str) -> Option<&SectionSource> {
        self.sections
            .iter()
            .find(|section: &&SectionSource| section.name == name)
    }
}

fn close_track(source: &mut SongSource, track: &mut Option<TrackSource>) {
    if let Some(done) = track.take() {
        source.tracks.push(done);
    }
}

fn read_track_line(track: &mut TrackSource, index: usize, line: &str) {
    track.last_index = index;
    if keyword_rest(line, "type").is_some() {
        track.type_index = Some(index);
    } else if keyword_rest(line, "clef").is_some() {
        track.clef_index = Some(index);
    } else if line == "lead" {
        track.lead_index = Some(index);
    } else if keyword_rest(line, "voice_layer").is_some() {
        track.voice_layer_indices.push(index);
    }
}

fn close_section(source: &mut SongSource, section: &mut Option<SectionSource>) {
    if let Some(done) = section.take() {
        source.sections.push(done);
    }
}

fn read_section_line(section: &mut SectionSource, index: usize, line: &str) {
    section.last_index = index;
    if let Some(rest) = keyword_rest(line, "line") {
        let (track_name, after): (&str, &str) = quoted_prefix(rest).unwrap_or(("", rest));
        let mut voice_layer: Option<u8> = None;
        let mut sample_source: bool = false;
        let mut dynamics: bool = false;
        let words: Vec<&str> = after
            .split_whitespace()
            .take_while(|word: &&str| !word.starts_with('|'))
            .collect();
        let mut position: usize = 0;
        while position < words.len() {
            match words[position] {
                "voice_layer" => {
                    voice_layer = words
                        .get(position + 1)
                        .and_then(|number: &&str| number.parse().ok());
                    position += 2;
                }
                "sample_source" => {
                    sample_source = true;
                    position += 1;
                }
                "dynamics" => {
                    dynamics = true;
                    position += 1;
                }
                _ => position += 1,
            }
        }
        section.lines.push(LineSource {
            track_name: track_name.to_string(),
            voice_layer,
            sample_source,
            dynamics,
            first_index: index,
            last_index: index,
        });
    } else if line.starts_with('|') {
        if let Some(last) = section.lines.last_mut() {
            last.last_index = index;
        }
    } else {
        section.setting_indices.push(index);
    }
}

fn read_composition_line(composition: &mut CompositionSource, index: usize, line: &str) {
    let Some((name, after)) = quoted_prefix(line) else {
        if composition.header_index.is_none() {
            composition.header_index = Some(index);
        }
        return;
    };
    let words: Vec<&str> = after.split_whitespace().collect();
    let mut repeat_count: u32 = 1;
    let mut key_words: Option<String> = None;
    let mut tempo_words: Option<String> = None;
    let mut position: usize = 0;
    while position < words.len() {
        match words[position] {
            "repeat" => {
                repeat_count = words
                    .get(position + 1)
                    .and_then(|count: &&str| count.parse().ok())
                    .unwrap_or(1);
                position += 2;
            }
            "key" => {
                key_words = Some(words[position + 1..(position + 3).min(words.len())].join(" "));
                position += 3;
            }
            "tempo" => {
                tempo_words = Some(words[position + 1..(position + 3).min(words.len())].join(" "));
                position += 3;
            }
            _ => position += 1,
        }
    }
    composition.entries.push(EntrySource {
        index,
        section_name: name.to_string(),
        repeat_count,
        key_words,
        tempo_words,
    });
}

/// The entry written back as one text line.
#[must_use]
pub fn entry_text(indent: &str, entry: &EntrySource) -> String {
    let mut words: Vec<String> = vec![format!("{indent}\"{}\"", entry.section_name)];
    if entry.repeat_count != 1 {
        words.push(format!("repeat {}", entry.repeat_count));
    }
    if let Some(key) = &entry.key_words {
        words.push(format!("key {key}"));
    }
    if let Some(tempo) = &entry.tempo_words {
        words.push(format!("tempo {tempo}"));
    }
    words.join("  ")
}

/// The indentation a text line starts with.
#[must_use]
pub fn indentation(line: &str) -> &str {
    let content_start: usize = line.len() - line.trim_start().len();
    &line[..content_start]
}

/// The text line with the words after `keyword` replaced by `value_words`
/// words, `None` when the keyword is not on the line. `key C major` takes
/// two words after `key`; `tempo 140 beats/min` two after `tempo`.
#[must_use]
pub fn replace_setting(
    line: &str,
    keyword: &str,
    word_count: usize,
    value: &str,
) -> Option<String> {
    let words: Vec<(usize, &str)> = word_spans(line);
    let position: usize = words
        .iter()
        .position(|(_, word): &(usize, &str)| *word == keyword)?;
    let last: &(usize, &str) = words.get(position + word_count)?;
    let start: usize = words[position + 1].0;
    let end: usize = last.0 + last.1.len();
    Some(format!("{}{value}{}", &line[..start], &line[end..]))
}

/// Each whitespace-separated word of `line` with its byte offset.
fn word_spans(line: &str) -> Vec<(usize, &str)> {
    let mut spans: Vec<(usize, &str)> = Vec::new();
    let mut start: Option<usize> = None;
    for (offset, character) in line.char_indices() {
        match (character.is_whitespace(), start) {
            (true, Some(begin)) => {
                spans.push((begin, &line[begin..offset]));
                start = None;
            }
            (false, None) => start = Some(offset),
            _ => {}
        }
    }
    if let Some(begin) = start {
        spans.push((begin, &line[begin..]));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = include_str!("../../../songs_canonical/composer/composer_demo.song");

    #[test]
    fn the_outline_finds_entries_sections_and_lines() {
        let source: SongSource = SongSource::read(DEMO);
        let composition: &CompositionSource = source.composition.as_ref().unwrap();
        let names: Vec<&str> = composition
            .entries
            .iter()
            .map(|entry: &EntrySource| entry.section_name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["Count-in", "Verse", "Chorus", "Bridge", "Verse"]
        );
        assert_eq!(composition.entries[1].repeat_count, 2);
        assert_eq!(composition.entries[2].key_words.as_deref(), Some("D major"));
        let verse: &SectionSource = source.section("Verse").unwrap();
        let tracks: Vec<&str> = verse
            .lines
            .iter()
            .map(|line: &LineSource| line.track_name.as_str())
            .collect();
        assert_eq!(tracks, vec!["Keys", "Bass", "Beat"]);
        // The Keys line goes on over a second text line.
        assert_eq!(verse.lines[0].last_index, verse.lines[0].first_index + 1);
    }

    #[test]
    fn a_setting_is_replaced_in_place() {
        assert_eq!(
            replace_setting(
                "  key C major  meter 4/4  tempo 140 beats/min",
                "key",
                2,
                "D major"
            )
            .as_deref(),
            Some("  key D major  meter 4/4  tempo 140 beats/min")
        );
        assert_eq!(
            replace_setting("  key C major  tempo 140 beats/min", "tempo", 1, "150").as_deref(),
            Some("  key C major  tempo 150 beats/min")
        );
        assert_eq!(replace_setting("  meter 4/4", "key", 2, "D major"), None);
    }
}
