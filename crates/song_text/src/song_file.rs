//! The song file: `song "Name"`, its `track` blocks, and its `section`
//! blocks with a `line` for each track (Song Notation, section 5).
//!
//! A file with any mistake does not load, not even partly (EFS-6.3): the
//! result is the song or all the mistakes found, in line order.

use vocabulary::key::Key;
use vocabulary::midi_values::{
    ControllerNumber, ControllerValue, NoteNumber, ProgramNumber, Velocity,
};
use vocabulary::song::{
    Clef, CueDefinition, CueDestination, CueMessage, Line, Section, Song, Track, TrackKind,
    VoiceKind,
};
use vocabulary::tempo::{Tempo, TempoError};
use vocabulary::time_signature::TimeSignature;

use crate::contents_text::parse_key;
use crate::parse_error::ParseError;
use crate::song_line::build_line;
use crate::text_line::{Word, keyword_rest, parse_meter, quoted_prefix, without_comment, words};

/// A track block while it is being read.
struct OpenTrack {
    name: String,
    line_number: usize,
    type_name: Option<String>,
    clef: Option<Clef>,
    lead: bool,
    home_octave: Option<i8>,
    cues: Vec<CueDefinition>,
    /// How many mistakes were reported when the block began; more by its
    /// end means it was reported on and is not checked again.
    errors_at_start: usize,
}

/// A line of a section as written, built once the section's meter is known.
struct RawLine {
    track_name: String,
    line_number: usize,
    bars_text: String,
}

/// A section block while it is being read.
struct OpenSection {
    name: String,
    line_number: usize,
    count_in: bool,
    repeat_count: Option<u32>,
    key: Option<Key>,
    time_signature: Option<TimeSignature>,
    tempo: Option<Tempo>,
    lines: Vec<RawLine>,
    errors_at_start: usize,
}

/// The key, meter and tempo a section carries on from the one before.
#[derive(Default)]
struct Carried {
    key: Option<Key>,
    time_signature: Option<TimeSignature>,
    tempo: Option<Tempo>,
}

struct SongReader<'a> {
    file_name: &'a str,
    errors: Vec<ParseError>,
    song_name: Option<(String, usize)>,
    tracks: Vec<Track>,
    /// Tracks that were written but are not usable, so their lines are not
    /// also reported.
    failed_track_names: Vec<String>,
    track: Option<OpenTrack>,
    sections: Vec<Section>,
    section: Option<OpenSection>,
    carried: Carried,
}

/// Reads a song file.
///
/// # Errors
///
/// Every mistake found, in line order, each naming the file and line.
pub fn parse_song(file_name: &str, text: &str) -> Result<Song, Vec<ParseError>> {
    let mut reader: SongReader = SongReader {
        file_name,
        errors: Vec::new(),
        song_name: None,
        tracks: Vec::new(),
        failed_track_names: Vec::new(),
        track: None,
        sections: Vec::new(),
        section: None,
        carried: Carried::default(),
    };
    for (index, raw_line) in text.lines().enumerate() {
        let line: &str = without_comment(raw_line).trim();
        if !line.is_empty() {
            reader.read_line(index + 1, line);
        }
    }
    reader.finish()
}

impl SongReader<'_> {
    fn report(&mut self, line_number: usize, message: String) {
        self.errors
            .push(ParseError::new(self.file_name, line_number, message));
    }

    fn read_line(&mut self, line_number: usize, line: &str) {
        let result: Result<(), String> = if let Some(rest) = keyword_rest(line, "song") {
            self.start_song(line_number, rest)
        } else if let Some(rest) = keyword_rest(line, "track") {
            self.start_track(line_number, rest)
        } else if let Some(rest) = keyword_rest(line, "section") {
            self.start_section(line_number, rest)
        } else if self.section.is_some() {
            self.read_section_line(line_number, line)
        } else if self.track.is_some() {
            self.read_track_line(line_number, line)
        } else {
            Err(format!(
                "\"{line}\" is outside a block. Start one with song \"Name\", track \"Name\" \
                 or section \"Name\""
            ))
        };
        if let Err(message) = result {
            self.report(line_number, message);
        }
    }

    fn start_song(&mut self, line_number: usize, rest: &str) -> Result<(), String> {
        if self.song_name.is_some() {
            return Err(String::from("a second song line. A file holds one song"));
        }
        match quoted_prefix(rest) {
            Some((name, "")) => {
                self.song_name = Some((name.to_string(), line_number));
                Ok(())
            }
            _ => Err(String::from(
                "write the song's name in quotes, e.g. song \"Demo\"",
            )),
        }
    }

    fn start_track(&mut self, line_number: usize, rest: &str) -> Result<(), String> {
        self.finish_track();
        self.finish_section();
        if !self.sections.is_empty() {
            return Err(String::from(
                "a track after the first section. Declare all tracks before the sections",
            ));
        }
        let Some((name, "")) = quoted_prefix(rest) else {
            return Err(String::from(
                "write the track's name in quotes, e.g. track \"Keys\"",
            ));
        };
        self.track = Some(OpenTrack {
            name: name.to_string(),
            line_number,
            type_name: None,
            clef: None,
            lead: false,
            home_octave: None,
            cues: Vec::new(),
            errors_at_start: self.errors.len(),
        });
        Ok(())
    }

    fn read_track_line(&mut self, line_number: usize, line: &str) -> Result<(), String> {
        let Some(track) = self.track.as_mut() else {
            return Ok(());
        };
        if let Some(rest) = keyword_rest(line, "type") {
            track.type_name = Some(rest.to_string());
        } else if let Some(rest) = keyword_rest(line, "clef") {
            track.clef = Some(parse_clef(rest)?);
        } else if line == "lead" {
            track.lead = true;
        } else if let Some(rest) = keyword_rest(line, "home_octave") {
            let octave: i8 = rest
                .parse::<i8>()
                .map_err(|_| format!("\"{rest}\" is not an octave. Write e.g. home_octave 3"))?;
            track.home_octave = Some(octave);
        } else if let Some(rest) = keyword_rest(line, "cue") {
            let cue: CueDefinition = parse_cue(rest)?;
            if track
                .cues
                .iter()
                .any(|earlier| earlier.name() == cue.name())
            {
                return Err(format!(
                    "the cue \"{}\" is already defined on this track. Give each cue its own name",
                    cue.name()
                ));
            }
            track.cues.push(cue);
        } else {
            let _ = line_number;
            return Err(format!(
                "\"{line}\" is not understood in a track. Expected type, clef, lead, \
                 home_octave or cue"
            ));
        }
        Ok(())
    }

    /// Turns the open track into a `Track`, or reports why not.
    fn finish_track(&mut self) {
        let Some(open) = self.track.take() else {
            return;
        };
        if self.errors.len() > open.errors_at_start {
            self.failed_track_names.push(open.name);
            return;
        }
        match build_track(&open) {
            Ok(track) => {
                if self
                    .tracks
                    .iter()
                    .any(|earlier| earlier.name() == track.name())
                {
                    self.report(
                        open.line_number,
                        format!(
                            "two tracks are named \"{}\". Give each track its own name",
                            open.name
                        ),
                    );
                } else {
                    self.tracks.push(track);
                }
            }
            Err(message) => {
                self.report(open.line_number, message);
                self.failed_track_names.push(open.name);
            }
        }
    }

    fn start_section(&mut self, line_number: usize, rest: &str) -> Result<(), String> {
        self.finish_track();
        self.finish_section();
        let Some((name, settings)) = quoted_prefix(rest) else {
            return Err(String::from(
                "write the section's name in quotes, e.g. section \"Verse\"",
            ));
        };
        let mut section: OpenSection = OpenSection {
            name: name.to_string(),
            line_number,
            count_in: false,
            repeat_count: None,
            key: None,
            time_signature: None,
            tempo: None,
            lines: Vec::new(),
            errors_at_start: self.errors.len(),
        };
        let result: Result<(), String> = read_settings(&mut section, settings);
        self.section = Some(section);
        result
    }

    fn read_section_line(&mut self, line_number: usize, line: &str) -> Result<(), String> {
        let Some(section) = self.section.as_mut() else {
            return Ok(());
        };
        if let Some(rest) = keyword_rest(line, "line") {
            let Some((track_name, bars_text)) = quoted_prefix(rest) else {
                return Err(String::from(
                    "write the track's name in quotes, then the bars, e.g. line \"Keys\" | q:I q q q |",
                ));
            };
            section.lines.push(RawLine {
                track_name: track_name.to_string(),
                line_number,
                bars_text: bars_text.to_string(),
            });
            Ok(())
        } else {
            read_settings(section, line)
        }
    }

    /// Turns the open section into a `Section`, or reports why not.
    fn finish_section(&mut self) {
        let Some(open) = self.section.take() else {
            return;
        };
        if self.errors.len() > open.errors_at_start {
            return;
        }
        let Some(time_signature) = open.time_signature.or(self.carried.time_signature) else {
            self.report(
                open.line_number,
                format!(
                    "section \"{}\" has no meter. Add meter 4/4 (or the meter you want)",
                    open.name
                ),
            );
            return;
        };
        let Some(tempo) = open.tempo.or(self.carried.tempo) else {
            self.report(
                open.line_number,
                format!(
                    "section \"{}\" has no tempo. Add tempo 120 q/min (or the tempo you want)",
                    open.name
                ),
            );
            return;
        };
        let key: Option<Key> = open.key.or(self.carried.key);
        let lines: Vec<Line> = self.build_lines(&open, time_signature);
        if self.errors.len() > open.errors_at_start {
            return;
        }
        // Repeating once is the plain reading of a section with no repeat.
        let repeat_count: u32 = open.repeat_count.unwrap_or(1);
        match Section::new(
            &open.name,
            open.count_in,
            repeat_count,
            key,
            time_signature,
            tempo,
            lines,
        ) {
            Ok(section) => {
                self.carried = Carried {
                    key,
                    time_signature: Some(time_signature),
                    tempo: Some(tempo),
                };
                self.sections.push(section);
            }
            Err(error) => self.report(open.line_number, error.to_string()),
        }
    }

    fn build_lines(&mut self, open: &OpenSection, time_signature: TimeSignature) -> Vec<Line> {
        let mut lines: Vec<Line> = Vec::new();
        for raw in &open.lines {
            if self.failed_track_names.contains(&raw.track_name) {
                continue;
            }
            let Some(track) = self
                .tracks
                .iter()
                .find(|track| track.name() == raw.track_name)
            else {
                self.report(
                    raw.line_number,
                    format!(
                        "\"{}\" is not a track of the song. Declare it with track \"{}\" \
                         before the sections, or fix the name",
                        raw.track_name, raw.track_name
                    ),
                );
                continue;
            };
            match build_line(track, &raw.bars_text, time_signature) {
                Ok(line) => lines.push(line),
                Err(message) => {
                    self.report(
                        raw.line_number,
                        format!("line \"{}\": {message}", raw.track_name),
                    );
                }
            }
        }
        lines
    }

    fn finish(mut self) -> Result<Song, Vec<ParseError>> {
        self.finish_track();
        self.finish_section();
        let Some((name, name_line)) = self.song_name.clone() else {
            self.report(
                1,
                String::from("the file has no song. Start it with song \"Name\""),
            );
            return Err(self.errors);
        };
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        match Song::new(&name, self.tracks, self.sections) {
            Ok(song) => Ok(song),
            Err(error) => {
                self.errors.push(ParseError::new(
                    self.file_name,
                    name_line,
                    error.to_string(),
                ));
                Err(self.errors)
            }
        }
    }
}

fn parse_clef(text: &str) -> Result<Clef, String> {
    match text {
        "treble" => Ok(Clef::Treble),
        "bass" => Ok(Clef::Bass),
        "alto" => Ok(Clef::Alto),
        "tenor" => Ok(Clef::Tenor),
        "percussion" => Ok(Clef::Percussion),
        "grand" => Ok(Clef::Grand),
        other => Err(format!(
            "\"{other}\" is not a clef. Use treble, bass, alto, tenor, percussion or grand"
        )),
    }
}

/// Builds a track from what its block said.
fn build_track(open: &OpenTrack) -> Result<Track, String> {
    let Some(type_name) = open.type_name.as_deref() else {
        return Err(format!(
            "track \"{}\" has no type. Add a line such as type poly_voice",
            open.name
        ));
    };
    let voice_only = |what: &str| {
        format!(
            "track \"{}\": {what} belongs to a voice track (poly_voice or mono_voice), not {type_name}",
            open.name
        )
    };
    let kind: TrackKind = match type_name {
        "poly_voice" | "mono_voice" => {
            let Some(clef) = open.clef else {
                return Err(format!(
                    "track \"{}\" has no clef. Add a line such as clef treble",
                    open.name
                ));
            };
            let voice: VoiceKind = if type_name == "poly_voice" {
                VoiceKind::Poly
            } else {
                VoiceKind::Mono
            };
            TrackKind::Voice {
                voice,
                clef,
                lead: open.lead,
                home_octave: open.home_octave,
            }
        }
        _ if open.clef.is_some() => return Err(voice_only("clef")),
        _ if open.lead => return Err(voice_only("lead")),
        _ if open.home_octave.is_some() => return Err(voice_only("home_octave")),
        "clock_and_transport" => TrackKind::ClockAndTransport,
        "cue_control" => TrackKind::CueControl {
            cues: open.cues.clone(),
        },
        "tempo_control" | "musician_voice" | "automation_control" => {
            return Err(format!(
                "track \"{}\": {type_name} is planned but not in V1",
                open.name
            ));
        }
        other => {
            return Err(format!(
                "track \"{}\": \"{other}\" is not a track type. Use poly_voice, mono_voice, \
                 clock_and_transport or cue_control",
                open.name
            ));
        }
    };
    if !open.cues.is_empty() && type_name != "cue_control" {
        return Err(format!(
            "track \"{}\" defines cues but is {type_name}. Cues belong on a cue_control track",
            open.name
        ));
    }
    Track::new(&open.name, kind).map_err(|error| error.to_string())
}

/// `"Start SQ-1"  destination "SQ-1"  start` and the other cue forms.
fn parse_cue(rest: &str) -> Result<CueDefinition, String> {
    let all: Vec<Word> = words(rest)?;
    let usage = || {
        String::from(
            "write a cue as: cue \"Name\" destination \"Device\" start, or note 36 velocity 100, \
             or cc 74 value 64, or program 3 (destination track \"Keys\" sends to a voice track)",
        )
    };
    let [name, keyword, tail @ ..] = all.as_slice() else {
        return Err(usage());
    };
    if !name.quoted || keyword.quoted || keyword.text != "destination" {
        return Err(usage());
    }
    let (destination, message_words): (CueDestination, &[Word]) = match tail {
        [device, rest @ ..] if device.quoted => (CueDestination::Named(device.text.clone()), rest),
        [track, voice, rest @ ..] if !track.quoted && track.text == "track" && voice.quoted => {
            (CueDestination::VoiceTrack(voice.text.clone()), rest)
        }
        _ => return Err(usage()),
    };
    let message: CueMessage = parse_cue_message(message_words).ok_or_else(usage)??;
    CueDefinition::new(&name.text, destination, message).map_err(|error| error.to_string())
}

/// `None` when the words are not a cue message at all; `Some(Err)` when they
/// are one with a value out of range.
fn parse_cue_message(message_words: &[Word]) -> Option<Result<CueMessage, String>> {
    let texts: Vec<&str> = message_words
        .iter()
        .map(|word| word.text.as_str())
        .collect();
    let number = |text: &str| text.parse::<u8>().ok();
    match texts.as_slice() {
        ["start"] => Some(Ok(CueMessage::Start)),
        ["stop"] => Some(Ok(CueMessage::Stop)),
        ["continue"] => Some(Ok(CueMessage::Continue)),
        ["program", program] => Some(
            number(program)
                .ok_or_else(|| format!("\"{program}\" is not a program number (0 to 127)"))
                .and_then(|value| ProgramNumber::new(value).map_err(|e| e.to_string()))
                .map(|program| CueMessage::ProgramChange { program }),
        ),
        ["note", note, "velocity", velocity] => Some(match (number(note), number(velocity)) {
            (Some(note), Some(velocity)) => NoteNumber::new(note)
                .and_then(|note| Velocity::new(velocity).map(|velocity| (note, velocity)))
                .map(|(note, velocity)| CueMessage::Note { note, velocity })
                .map_err(|e| e.to_string()),
            _ => Err(String::from(
                "a note cue is: note 36 velocity 100 (whole numbers)",
            )),
        }),
        ["cc", controller, "value", value] => Some(match (number(controller), number(value)) {
            (Some(controller), Some(value)) => ControllerNumber::new(controller)
                .and_then(|controller| ControllerValue::new(value).map(|value| (controller, value)))
                .map(|(controller, value)| CueMessage::ControlChange { controller, value })
                .map_err(|e| e.to_string()),
            _ => Err(String::from(
                "a controller cue is: cc 74 value 64 (whole numbers)",
            )),
        }),
        _ => None,
    }
}

/// Reads `count-in`, `repeat n`, `key C major`, `meter 4/4`, `tempo 120
/// q/min` in any order, on the section line or lines after it.
fn read_settings(section: &mut OpenSection, text: &str) -> Result<(), String> {
    let all: Vec<Word> = words(text)?;
    let mut remaining: &[Word] = &all;
    while let Some((first, after)) = remaining.split_first() {
        if first.quoted {
            return Err(format!(
                "\"{}\" in quotes is not a setting. Expected count-in, repeat, key, meter, \
                 tempo or line",
                first.text
            ));
        }
        remaining = match first.text.as_str() {
            "count-in" => {
                section.count_in = true;
                after
            }
            "repeat" => read_repeat(section, after)?,
            "key" => read_key(section, after)?,
            "meter" => read_meter(section, after)?,
            "tempo" => read_tempo(section, after)?,
            other => {
                return Err(format!(
                    "\"{other}\" is not understood in a section. Expected count-in, repeat n, \
                     key C major, meter 4/4, tempo 120 q/min, or line \"Track\" | ... |"
                ));
            }
        };
    }
    Ok(())
}

fn read_repeat<'a>(section: &mut OpenSection, after: &'a [Word]) -> Result<&'a [Word], String> {
    let Some((count, rest)) = after.split_first() else {
        return Err(String::from("repeat needs a number, e.g. repeat 2"));
    };
    let repeat_count: u32 = count.text.parse::<u32>().map_err(|_| {
        format!(
            "\"{}\" is not a repeat count. Write e.g. repeat 2",
            count.text
        )
    })?;
    if section.repeat_count.replace(repeat_count).is_some() {
        return Err(String::from("repeat is written twice in this section"));
    }
    Ok(rest)
}

fn read_key<'a>(section: &mut OpenSection, after: &'a [Word]) -> Result<&'a [Word], String> {
    let [tonic, mode, rest @ ..] = after else {
        return Err(String::from(
            "key needs a note and a mode, e.g. key C major",
        ));
    };
    let key: Key = parse_key(&tonic.text, &mode.text)?;
    if section.key.replace(key).is_some() {
        return Err(String::from("key is written twice in this section"));
    }
    Ok(rest)
}

fn read_meter<'a>(section: &mut OpenSection, after: &'a [Word]) -> Result<&'a [Word], String> {
    let Some((meter, rest)) = after.split_first() else {
        return Err(String::from("meter needs two numbers, e.g. meter 4/4"));
    };
    let time_signature: TimeSignature = parse_meter(&meter.text)?;
    if section.time_signature.replace(time_signature).is_some() {
        return Err(String::from("meter is written twice in this section"));
    }
    Ok(rest)
}

fn read_tempo<'a>(section: &mut OpenSection, after: &'a [Word]) -> Result<&'a [Word], String> {
    let wrong = || String::from("write the tempo with its unit, e.g. tempo 120 q/min");
    let [number, unit, rest @ ..] = after else {
        return Err(wrong());
    };
    if unit.text != "q/min" {
        return Err(wrong());
    }
    let quarter_notes_per_minute: u64 = number.text.parse::<u64>().map_err(|_| wrong())?;
    let tempo: Tempo = Tempo::from_quarter_notes_per_minute(quarter_notes_per_minute).map_err(
        |error| match error {
            TempoError::Zero => String::from("a tempo of 0 q/min never plays. Use 1 or more"),
            TempoError::TooFast { .. } => format!("{quarter_notes_per_minute} q/min is too fast"),
        },
    )?;
    if section.tempo.replace(tempo).is_some() {
        return Err(String::from("tempo is written twice in this section"));
    }
    Ok(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::song::ElementContents;

    const DEMO: &str = include_str!("../../../songs/Demo.song");

    fn errors_of(text: &str) -> Vec<String> {
        parse_song("test.song", text)
            .unwrap_err()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// A song with one section; `tail` replaces the section's lines.
    fn song_with(tail: &str) -> String {
        format!(
            "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
             track \"Keys\"\n  type poly_voice\n  clef grand\n  lead\n\
             track \"Bass\"\n  type mono_voice\n  clef bass\n\
             section \"A\"\n  key C major  meter 4/4  tempo 120 q/min\n{tail}"
        )
    }

    #[test]
    fn the_demo_song_loads() {
        let song: Song = parse_song("Demo.song", DEMO).unwrap();
        assert_eq!(song.name(), "Demo");
        assert_eq!(song.tracks().len(), 5);
        assert_eq!(song.sections().len(), 2);
        let verse: &Section = &song.sections()[1];
        assert_eq!(verse.repeat_count(), 2);
        assert_eq!(verse.bar_count(), 2);
        assert_eq!(verse.lines().len(), 4);
        // The count-in carries no key; the verse names one.
        assert!(song.sections()[0].key().is_none());
        assert!(song.sections()[0].count_in());
    }

    #[test]
    fn the_second_chord_carries_to_the_rest_of_its_bar() {
        let song: Song = parse_song("Demo.song", DEMO).unwrap();
        let keys: &Line = song.sections()[1]
            .lines()
            .iter()
            .find(|line| line.track_name() == "Keys")
            .unwrap();
        // I q q q: four elements, all the I chord.
        let first_four: Vec<String> = keys.elements()[..4]
            .iter()
            .map(|element| match element.contents() {
                Some(ElementContents::Chord(chord)) => chord.to_string(),
                _ => String::from("?"),
            })
            .collect();
        assert_eq!(first_four, vec!["I"; 4]);
    }

    #[test]
    fn a_line_for_an_undeclared_track_names_the_track() {
        let errors: Vec<String> = errors_of(&song_with("  line \"Drums\"  | q q q q |\n"));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("test.song line 13:"));
        assert!(errors[0].contains("\"Drums\" is not a track of the song"));
    }

    #[test]
    fn a_chord_on_the_bass_is_refused_with_the_place() {
        let errors: Vec<String> = errors_of(&song_with(
            "  line \"Keys\"  | q:I q q q |\n  line \"Bass\"  | q:C2 q q q:V7 |\n",
        ));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("bar 1 count 4:"), "{}", errors[0]);
    }

    #[test]
    fn lines_of_different_length_are_reported_at_the_section() {
        let errors: Vec<String> = errors_of(&song_with(
            "  line \"Keys\"  | q:I q q q |\n  line \"Bass\"  | q:C2 q q q | q q q q |\n",
        ));
        assert!(errors[0].contains("same number of bars"), "{}", errors[0]);
    }

    #[test]
    fn a_song_without_a_lead_or_clock_is_refused() {
        let text: &str = "song \"T\"\ntrack \"Keys\"\n  type poly_voice\n  clef treble\n\
                          section \"A\"\n  key C major  meter 4/4  tempo 120 q/min\n  line \"Keys\" | q:I q q q |\n";
        let errors: Vec<String> = errors_of(text);
        assert!(errors[0].contains("clock and transport"), "{}", errors[0]);
    }

    #[test]
    fn a_missing_meter_and_tempo_are_asked_for() {
        let text: &str = "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
                          section \"A\"\n";
        let errors: Vec<String> = errors_of(text);
        assert!(errors[0].contains("has no meter"), "{}", errors[0]);
    }

    #[test]
    fn mistakes_in_a_track_block_are_all_reported() {
        let text: &str = "song \"T\"\ntrack \"A\"\n  clef treble\n  type poly_voice\n  wobble\n\
                          track \"B\"\n  type bass_synth\n";
        let errors: Vec<String> = errors_of(text);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("test.song line 5:"));
        assert!(errors[1].contains("\"bass_synth\" is not a track type"));
    }

    #[test]
    fn chords_in_a_section_without_a_key_are_refused() {
        let text: &str = "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
                          track \"Keys\"\n  type poly_voice\n  clef treble\n  lead\n\
                          section \"A\"\n  meter 4/4  tempo 120 q/min\n  line \"Keys\" | q:I q q q |\n";
        let errors: Vec<String> = errors_of(text);
        assert!(errors[0].contains("has chords but no key"), "{}", errors[0]);
    }

    #[test]
    fn a_cue_not_defined_is_refused() {
        let text: &str = "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
                          track \"Cues\"\n  type cue_control\n  cue \"Go\" destination \"X\" start\n\
                          section \"A\"\n  meter 4/4  tempo 120 q/min\n  line \"Cues\" | w:\"Stop\" |\n";
        let errors: Vec<String> = errors_of(text);
        assert!(errors[0].contains("no cue named \"Stop\""), "{}", errors[0]);
    }
}
