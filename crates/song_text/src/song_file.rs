//! The song file: `song "Name"`, its `track` blocks, and its `section`
//! blocks with a `line` for each track, or for each voice layer of a track
//! (Song Notation, section 5).
//!
//! A file with any mistake does not load, not even partly (EFS-6.3): the
//! result is the song or all the mistakes found, in line order.

use vocabulary::key::Key;
use vocabulary::midi_values::{
    ControllerNumber, ControllerValue, NoteNumber, ProgramNumber, Velocity,
};
use vocabulary::score_length::ScoreLength;
use vocabulary::song::{
    Clef, CueDefinition, CueDestination, CueMessage, Line, LineRole, Section, Song, Track,
    TrackKind, VoiceKind, VoiceLayerName, VoiceLayerNumber,
};
use vocabulary::tempo::{Tempo, TempoError};
use vocabulary::time_signature::TimeSignature;

use crate::contents_text::parse_key;
use crate::parse_error::ParseError;
use crate::song_line::{BarsText, build_line};
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
    voice_layer_names: Vec<VoiceLayerName>,
    /// How many mistakes were reported when the block began; more by its
    /// end means it was reported on and is not checked again.
    errors_at_start: usize,
}

/// A line of a section as written, built once the section's meter is known.
/// Its bars may be on the `line` line and on the text lines after it that
/// start with `|`.
struct RawLine {
    track_name: String,
    /// `voice_layer n` when written after the track's name.
    voice_layer: Option<VoiceLayerNumber>,
    /// `sample_source` written after the track's name (and voice layer).
    sample_source: bool,
    line_number: usize,
    parts: Vec<RawBarsText>,
}

/// One text line of a line's bars, as written.
struct RawBarsText {
    line_number: usize,
    text: String,
}

/// A section block while it is being read.
struct OpenSection {
    name: String,
    line_number: usize,
    count_in: bool,
    repeat_count: Option<u32>,
    key: Option<Key>,
    time_signature: Option<TimeSignature>,
    tempo: Option<WrittenTempo>,
    lines: Vec<RawLine>,
    /// True right after a `line` or its continuation: a text line starting
    /// with `|` then adds bars to that line.
    continuing_line: bool,
    errors_at_start: usize,
}

/// A tempo as the song file writes it. How long a beat is comes from the
/// section's meter, which may be written after the tempo or carried from
/// the section before, so the conversion waits until the section closes.
#[derive(Debug, Clone)]
enum WrittenTempo {
    BeatsPerMinute(MilliBeats),
    BeatsPerSecond(MilliBeats),
}

/// The number of a written tempo: whole milli-beats (1000 mbeats = 1 beat)
/// for exactness, with the text as typed for messages.
#[derive(Debug, Clone)]
struct MilliBeats {
    milli_beats: u64,
    text: String,
}

impl WrittenTempo {
    fn to_tempo(&self, time_signature: TimeSignature) -> Result<Tempo, String> {
        let beat_length: ScoreLength = time_signature.beat_length();
        let (result, written): (Result<Tempo, TempoError>, String) = match self {
            WrittenTempo::BeatsPerMinute(number) => (
                Tempo::from_milli_beats_per_minute(number.milli_beats, beat_length),
                format!("{} beats/min", number.text),
            ),
            WrittenTempo::BeatsPerSecond(number) => (
                Tempo::from_milli_beats_per_second(number.milli_beats, beat_length),
                format!("{} beats/s", number.text),
            ),
        };
        result.map_err(|error| match error {
            TempoError::Zero => format!("a tempo of {written} never plays. Use more than 0"),
            TempoError::TooFast { .. }
            | TempoError::BeatsTooFast { .. }
            | TempoError::BeatsPerSecondTooFast { .. } => format!("{written} is too fast"),
            TempoError::BeatNotWholeMilliQuarters { .. } => {
                format!(
                    "{written} in {}/{}: {error}",
                    time_signature.pulses_per_bar(),
                    time_signature.pulses_per_whole_note()
                )
            }
        })
    }
}

/// The key, meter and tempo a section carries on from the one before. The
/// tempo carries as note speed, not as beats: a section that changes the
/// meter but writes no tempo keeps the quarter note's speed (Jim, 7 Nov
/// 2026).
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
            voice_layer_names: Vec::new(),
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
        } else if let Some(rest) = keyword_rest(line, "voice_layer") {
            track.voice_layer_names.push(parse_voice_layer_name(rest)?);
        } else {
            let _ = line_number;
            return Err(format!(
                "\"{line}\" is not understood in a track. Expected type, clef, lead, \
                 home_octave, voice_layer or cue"
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
            continuing_line: false,
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
                section.continuing_line = false;
                return Err(String::from(
                    "write the track's name in quotes, then the bars, e.g. line \"Keys\" | I:q / / / |",
                ));
            };
            let (voice_layer, bars_text): (Option<VoiceLayerNumber>, &str) =
                match read_voice_layer(bars_text) {
                    Ok(read) => read,
                    Err(message) => {
                        section.continuing_line = false;
                        return Err(message);
                    }
                };
            let (sample_source, bars_text): (bool, &str) = read_sample_source(bars_text);
            // The bars may all be on the lines below, so a bare name is fine.
            let parts: Vec<RawBarsText> = if bars_text.trim().is_empty() {
                Vec::new()
            } else {
                vec![RawBarsText {
                    line_number,
                    text: bars_text.to_string(),
                }]
            };
            section.lines.push(RawLine {
                track_name: track_name.to_string(),
                voice_layer,
                sample_source,
                line_number,
                parts,
            });
            section.continuing_line = true;
            Ok(())
        } else if line.starts_with('|') {
            match section.lines.last_mut() {
                Some(raw) if section.continuing_line => {
                    raw.parts.push(RawBarsText {
                        line_number,
                        text: line.to_string(),
                    });
                    Ok(())
                }
                _ => Err(String::from(
                    "bars with no line above them. Put them right under the line they continue, \
                     or start them with line \"Name\"",
                )),
            }
        } else {
            section.continuing_line = false;
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
        let tempo: Tempo = match (&open.tempo, self.carried.tempo) {
            (Some(written), _) => match written.to_tempo(time_signature) {
                Ok(tempo) => tempo,
                Err(message) => {
                    self.report(
                        open.line_number,
                        format!("section \"{}\": {message}", open.name),
                    );
                    return;
                }
            },
            (None, Some(carried)) => carried,
            (None, None) => {
                self.report(
                    open.line_number,
                    format!(
                        "section \"{}\" has no tempo. Add tempo 120 beats/min (or the tempo \
                         you want)",
                        open.name
                    ),
                );
                return;
            }
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
            let parts: Vec<BarsText> = raw
                .parts
                .iter()
                .map(|part: &RawBarsText| BarsText {
                    line_number: part.line_number,
                    text: &part.text,
                })
                .collect();
            // A line names its voice layer, or is layer 1; a cue control
            // track's line places its cues. A role that does not suit the
            // track is refused by the line, with a message.
            let role: LineRole = match (raw.voice_layer, track.kind()) {
                (number, _) if raw.sample_source => {
                    LineRole::SampleSource(number.unwrap_or(VoiceLayerNumber::FIRST))
                }
                (Some(number), _) => LineRole::VoiceLayer(number),
                (None, TrackKind::CueControl { .. }) => LineRole::CuePlacement,
                (None, TrackKind::Voice { .. } | TrackKind::ClockAndTransport) => {
                    LineRole::VoiceLayer(VoiceLayerNumber::FIRST)
                }
            };
            match build_line(track, role, raw.line_number, &parts, time_signature) {
                Ok(line) => lines.push(line),
                Err(error) => {
                    self.report(
                        error.line_number,
                        format!("line \"{}\": {}", raw.track_name, error.message),
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
                voice_layer_names: open.voice_layer_names.clone(),
            }
        }
        _ if open.clef.is_some() => return Err(voice_only("clef")),
        _ if !open.voice_layer_names.is_empty() => return Err(voice_only("voice_layer")),
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

/// After a line's track name: `voice_layer 2` and what follows it, or the
/// text as it is when no voice layer is written.
fn read_voice_layer(after_name: &str) -> Result<(Option<VoiceLayerNumber>, &str), String> {
    let Some(rest) = keyword_rest(after_name.trim(), "voice_layer") else {
        if after_name.trim().is_empty()
            || after_name.trim_start().starts_with('|')
            || keyword_rest(after_name.trim(), "sample_source").is_some()
        {
            return Ok((None, after_name));
        }
        return Err(format!(
            "\"{}\" after the track's name is not understood. Write voice_layer and its \
             number, sample_source, or the bars, e.g. line \"Keys\" voice_layer 2 | C4:q D4 E4 F4 |",
            after_name.trim()
        ));
    };
    // Nothing after voice_layer reads as an empty number, which the next
    // step reports as not a number: no value is invented.
    let number_text: &str = rest.split_whitespace().next().unwrap_or("");
    let number: VoiceLayerNumber = voice_layer_number(number_text)?;
    Ok((Some(number), rest[number_text.len()..].trim_start()))
}

/// `sample_source` right after the track's name or its voice layer, and the
/// text after it; the text as it is when the word is not there.
fn read_sample_source(after: &str) -> (bool, &str) {
    match keyword_rest(after.trim(), "sample_source") {
        Some(rest) => (true, rest),
        None => (false, after),
    }
}

/// `2` → voice layer 2.
fn voice_layer_number(text: &str) -> Result<VoiceLayerNumber, String> {
    let number: u8 = text
        .parse::<u8>()
        .map_err(|_| format!("\"{text}\" is not a voice layer number. Write e.g. voice_layer 2"))?;
    VoiceLayerNumber::new(number).map_err(|error| error.to_string())
}

/// In a track block, `voice_layer 2 "Inner"`: a name for layer 2.
fn parse_voice_layer_name(rest: &str) -> Result<VoiceLayerName, String> {
    let usage = || {
        String::from(
            "in a track, voice_layer gives a layer a name: write its number and the name in \
             quotes, e.g. voice_layer 2 \"Inner\"",
        )
    };
    let (number_text, after_number): (&str, &str) =
        rest.split_once(char::is_whitespace).ok_or_else(usage)?;
    let number: VoiceLayerNumber = voice_layer_number(number_text)?;
    match quoted_prefix(after_number) {
        Some((name, "")) => VoiceLayerName::new(number, name).map_err(|error| error.to_string()),
        _ => Err(usage()),
    }
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
/// beats/min` in any order, on the section line or lines after it.
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
                     key C major, meter 4/4, tempo 120 beats/min, or line \"Track\" | ... |"
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

/// A tempo's number as whole milli-beats: digits, then optionally a point
/// and one to three decimals (`120`, `112.5`, `0.125`). `None` for anything
/// else, including a fourth decimal, which would not be held exactly.
fn parse_milli_beats(text: &str) -> Option<u64> {
    let (whole, decimals): (&str, &str) = text.split_once('.').unwrap_or((text, ""));
    let digits_only = |part: &str| part.bytes().all(|byte: u8| byte.is_ascii_digit());
    // A point must be followed by a decimal: "112." is a typing slip.
    let point_without_decimals: bool = text.contains('.') && decimals.is_empty();
    if whole.is_empty()
        || !digits_only(whole)
        || !digits_only(decimals)
        || decimals.len() > 3
        || point_without_decimals
    {
        return None;
    }
    // Pad the decimals to three digits: "5" is 500 mbeats, "125" is 125.
    let milli_part: u64 = format!("{decimals:0<3}").parse::<u64>().ok()?;
    whole
        .parse::<u64>()
        .ok()?
        .checked_mul(1000)?
        .checked_add(milli_part)
}

/// `tempo 120 beats/min`, `tempo 112.5 beats/min` or `tempo 2 beats/s`: a
/// number of beats with up to three decimals, the beat being the one the
/// meter defines.
fn read_tempo<'a>(section: &mut OpenSection, after: &'a [Word]) -> Result<&'a [Word], String> {
    let wrong = || {
        String::from(
            "write the tempo as beats with its unit, e.g. tempo 120 beats/min, \
             tempo 112.5 beats/min (up to 3 decimals) or tempo 2 beats/s",
        )
    };
    let [number, unit, rest @ ..] = after else {
        return Err(wrong());
    };
    let milli_beats: u64 = parse_milli_beats(&number.text).ok_or_else(wrong)?;
    let number_written = || MilliBeats {
        milli_beats,
        text: number.text.clone(),
    };
    let written: WrittenTempo = match unit.text.as_str() {
        "beats/min" => WrittenTempo::BeatsPerMinute(number_written()),
        "beats/s" => WrittenTempo::BeatsPerSecond(number_written()),
        // The note-value units of earlier song files (q/min, e/min, ...).
        old if old.ends_with("/min") || old.ends_with("/s") => {
            return Err(format!(
                "{old} is no longer read. Write the tempo in beats, e.g. tempo {} \
                 beats/min. The meter says what a beat is (in 6/8, a dotted quarter)",
                number.text
            ));
        }
        _ => return Err(wrong()),
    };
    if milli_beats == 0 {
        return Err(String::from(
            "a tempo of 0 beats never plays. Use more than 0",
        ));
    }
    if section.tempo.replace(written).is_some() {
        return Err(String::from("tempo is written twice in this section"));
    }
    Ok(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::song::ElementContents;

    const DEMO: &str = include_str!("../../../songs/song_test_count_in_and_verse.song");

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
             section \"A\"\n  key C major  meter 4/4  tempo 120 beats/min\n{tail}"
        )
    }

    #[test]
    fn the_demo_song_loads() {
        let song: Song = parse_song("song_test_count_in_and_verse.song", DEMO).unwrap();
        assert_eq!(song.name(), "Song test count-in and verse");
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
        let song: Song = parse_song("song_test_count_in_and_verse.song", DEMO).unwrap();
        let keys: &Line = song.sections()[1]
            .lines()
            .iter()
            .find(|line| line.track_name() == "Keys")
            .unwrap();
        // I:q / / /: four elements, all the I chord.
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
        let errors: Vec<String> = errors_of(&song_with("  line \"Drums\"  | C4:q D4 E4 F4 |\n"));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("test.song line 13:"));
        assert!(errors[0].contains("\"Drums\" is not a track of the song"));
    }

    #[test]
    fn a_chord_on_the_bass_is_refused_with_the_place() {
        let errors: Vec<String> = errors_of(&song_with(
            "  line \"Keys\"  | I:q / / / |\n  line \"Bass\"  | C2:q / / V7 |\n",
        ));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("bar 1 beat 4:"), "{}", errors[0]);
    }

    #[test]
    fn lines_of_different_length_are_reported_at_the_section() {
        let errors: Vec<String> = errors_of(&song_with(
            "  line \"Keys\"  | I:q / / / |\n  line \"Bass\"  | C2:q / / / | / / / / |\n",
        ));
        assert!(errors[0].contains("same number of bars"), "{}", errors[0]);
    }

    #[test]
    fn a_song_without_a_lead_or_clock_is_refused() {
        let text: &str = "song \"T\"\ntrack \"Keys\"\n  type poly_voice\n  clef treble\n\
                          section \"A\"\n  key C major  meter 4/4  tempo 120 beats/min\n  line \"Keys\" | I:q / / / |\n";
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
                          section \"A\"\n  meter 4/4  tempo 120 beats/min\n  line \"Keys\" | I:q / / / |\n";
        let errors: Vec<String> = errors_of(text);
        assert!(errors[0].contains("has chords but no key"), "{}", errors[0]);
    }

    #[test]
    fn a_cue_not_defined_is_refused() {
        let text: &str = "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
                          track \"Cues\"\n  type cue_control\n  cue \"Go\" destination \"X\" start\n\
                          section \"A\"\n  meter 4/4  tempo 120 beats/min\n  line \"Cues\" | \"Stop\":w |\n";
        let errors: Vec<String> = errors_of(text);
        assert!(errors[0].contains("no cue named \"Stop\""), "{}", errors[0]);
    }

    /// Two Keys sections. Each is one bar, holding one chord: `first_bar`
    /// and `second_bar` must fill the bar of `first` and `second`'s meter.
    fn two_sections(first: &str, first_bar: &str, second: &str, second_bar: &str) -> String {
        format!(
            "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
             track \"Keys\"\n  type poly_voice\n  clef grand\n  lead\n\
             section \"A\"\n  key C major  {first}\n  line \"Keys\" | {first_bar} |\n\
             section \"B\"\n  {second}\n  line \"Keys\" | {second_bar} |\n"
        )
    }

    fn tempos_of(text: &str) -> Vec<u64> {
        parse_song("test.song", text)
            .unwrap()
            .sections()
            .iter()
            .map(|section: &Section| section.tempo().milli_quarters_per_minute())
            .collect()
    }

    #[test]
    fn a_tempo_counts_the_beats_of_its_meter() {
        // 4/4: quarter-note beats. 120 beats/min = 120 q/min.
        // 6/8: dotted-quarter beats. 60 beats/min × 1.5 q/beat = 90 q/min.
        assert_eq!(
            tempos_of(&two_sections(
                "meter 4/4  tempo 120 beats/min",
                "I:w",
                "meter 6/8  tempo 60 beats/min",
                "I:h."
            )),
            vec![120_000, 90_000]
        );
        // 3/8: eighth-note beats. 30 beats/min × 0.5 q/beat = 15 q/min.
        assert_eq!(
            tempos_of(&two_sections(
                "meter 3/8  tempo 30 beats/min",
                "I:q.",
                "meter 9/8  tempo 60 beats/min",
                "I:h. /:q."
            )),
            vec![15_000, 90_000]
        );
    }

    #[test]
    fn a_tempo_may_come_before_its_meter_and_in_beats_per_second() {
        // 2 beats/s × 60 s/min = 120 dotted quarters/min × 1.5 q = 180 q/min.
        assert_eq!(
            tempos_of(&two_sections(
                "tempo 120 beats/min  meter 4/4",
                "I:w",
                "tempo 2 beats/s  meter 6/8",
                "I:h."
            )),
            vec![120_000, 180_000]
        );
    }

    #[test]
    fn a_carried_tempo_keeps_the_note_speed_across_a_meter_change() {
        // B writes no tempo: the quarter note stays at 120 q/min in 6/8
        // (80 dotted-quarter beats/min), not 120 dotted quarters/min.
        assert_eq!(
            tempos_of(&two_sections(
                "meter 4/4  tempo 120 beats/min",
                "I:w",
                "meter 6/8",
                "I:h."
            )),
            vec![120_000, 120_000]
        );
    }

    #[test]
    fn a_note_value_tempo_says_to_write_beats() {
        let errors: Vec<String> =
            errors_of(&song_with("").replace("tempo 120 beats/min", "tempo 30 e/min"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains(
                "e/min is no longer read. Write the tempo in beats, e.g. tempo 30 beats/min"
            ),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_tempo_without_a_beat_number_and_a_unit_is_refused() {
        for written in [
            "tempo 120",
            "tempo 112.1234 beats/min",
            "tempo .5 beats/min",
            "tempo 112. beats/min",
            "tempo 120 bpm",
            "tempo 0 beats/min",
        ] {
            let errors: Vec<String> =
                errors_of(&song_with("").replace("tempo 120 beats/min", written));
            assert_eq!(errors.len(), 1, "{written}: {errors:?}");
            assert!(errors[0].starts_with("test.song line 12:"), "{}", errors[0]);
        }
    }

    #[test]
    fn a_fractional_tempo_is_held_exactly() {
        // (written, meter, mq/min). 4/4: 112.5 beats/min × 1 q/beat = 112500
        // mq/min. 6/8: 100.5 beats/min × 1.5 q/beat = 150.75 q/min = 150750
        // mq/min.
        for (written, meter, expected_milli_quarters_per_minute) in [
            ("tempo 112.5 beats/min", "4/4", 112_500_u64),
            ("tempo 112.125 beats/min", "4/4", 112_125),
            ("tempo 100.5 beats/min", "6/8", 150_750),
            ("tempo 0.5 beats/s", "4/4", 30_000),
        ] {
            // One bar of a chord, filling the bar of the meter.
            let bar: &str = if meter == "6/8" { "I:q. /" } else { "I:w" };
            let text: String = song_with(&format!("  line \"Keys\" | {bar} |\n"))
                .replace("tempo 120 beats/min", written)
                .replace("meter 4/4", &format!("meter {meter}"));
            let song: Song = parse_song("test.song", &text).unwrap();
            assert_eq!(
                song.sections()[0].tempo().milli_quarters_per_minute(),
                expected_milli_quarters_per_minute,
                "{written} in {meter}"
            );
        }
    }

    #[test]
    fn a_fractional_tempo_that_is_not_whole_milli_quarters_is_refused() {
        // 16th-note beats (x/16 meters): 0.001 beats/min × 0.25 q/beat is
        // 0.25 mq/min, not whole.
        let errors: Vec<String> = errors_of(
            &song_with("  line \"Keys\" | I:q |\n")
                .replace("tempo 120 beats/min", "tempo 100.001 beats/min")
                .replace("meter 4/4", "meter 4/16"),
        );
        assert_eq!(errors.len(), 1, "{errors:?}");
        // Reported at the section's line, where the meter and tempo meet.
        assert!(errors[0].starts_with("test.song line 11:"), "{}", errors[0]);
    }

    #[test]
    fn a_compound_meter_above_twelve_is_refused_at_its_line() {
        let errors: Vec<String> = errors_of(&song_with("").replace("meter 4/4", "meter 15/8"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 12:"), "{}", errors[0]);
        assert!(errors[0].contains("15/8 is not read yet"), "{}", errors[0]);
    }

    fn bar_counts_of(text: &str) -> Vec<u32> {
        parse_song("test.song", text)
            .unwrap()
            .sections()
            .iter()
            .map(Section::bar_count)
            .collect()
    }

    #[test]
    fn a_line_goes_on_over_the_text_lines_below_it() {
        // Lines 13 to 16: the Keys line over two text lines with a comment
        // between, the Bass line written under its bare name.
        let text: String = song_with(
            "  line \"Keys\"  | I:w | V |\n\
             # the turnaround\n\
             \x20              | vi | IV |\n\
             \x20 line \"Bass\"\n\
             \x20   | C2:w | G2 | A2 | F2 |\n",
        );
        assert_eq!(bar_counts_of(&text), vec![4]);
    }

    #[test]
    fn a_mistake_in_a_continuation_names_its_own_text_line() {
        // Bar 3, on line 14, is short; the message counts bars along the line.
        let errors: Vec<String> = errors_of(&song_with(
            "  line \"Keys\"  | I:w | V |\n\
             \x20              | vi:h | IV:w |\n",
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("test.song line 14: line \"Keys\": bar 3 is short"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn bars_not_under_a_line_are_refused() {
        // Before any line in the section (line 13).
        let errors: Vec<String> = errors_of(&song_with("  | I:w |\n"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("test.song line 13: bars with no line above them"),
            "{}",
            errors[0]
        );
        // After a setting (line 14), which ends the line above it.
        let errors: Vec<String> = errors_of(&song_with(
            "  line \"Keys\"  | I:w |\n  repeat 2\n  | V:w |\n",
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("test.song line 15: bars with no line above them"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn joined_lines_of_different_lengths_are_reported() {
        let errors: Vec<String> = errors_of(&song_with(
            "  line \"Keys\"  | I:w | V |\n\
             \x20              | vi |\n\
             \x20 line \"Bass\"  | C2:w | G2 |\n",
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
    }

    #[test]
    fn voice_layers_are_lines_of_their_own_on_one_track() {
        let song: Song = parse_song(
            "test.song",
            &song_with(
                "  line \"Keys\"  voice_layer 1  | G4:q G4 F4 Eb4 |\n\
                 \x20 line \"Keys\"  voice_layer 2  | Eb4:sx D4 Eb4:e~ Eb4:sx F4 Eb4 D4 C4:e D4~ D4 C4 |\n\
                 \x20 line \"Keys\"  voice_layer 3\n\
                 \x20   | r:h A3~ |\n\
                 \x20   | A3 r |\n\
                 \x20 line \"Bass\"  | C3:w | / |\n",
            )
            .replace("| G4:q G4 F4 Eb4 |", "| G4:q G4 F4 Eb4 | C5:w |")
            .replace("D4~ D4 C4 |\n", "D4~ D4 C4 | C4:w |\n"),
        )
        .unwrap();
        let roles: Vec<String> = song.sections()[0]
            .lines()
            .iter()
            .map(|line: &Line| format!("{} {}", line.track_name(), line.role()))
            .collect();
        assert_eq!(
            roles,
            vec![
                "Keys voice layer 1",
                "Keys voice layer 2",
                "Keys voice layer 3",
                "Bass voice layer 1",
            ]
        );
    }

    #[test]
    fn a_line_without_voice_layer_is_layer_one_and_two_such_lines_are_refused() {
        let errors: Vec<String> = errors_of(&song_with(
            "  line \"Keys\"  | I:w |\n  line \"Keys\"  voice_layer 1  | V:w |\n",
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("two lines for \"Keys\" voice layer 1"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn voice_layer_mistakes_say_what_to_write() {
        let errors: Vec<String> =
            errors_of(&song_with("  line \"Keys\"  voice_layer two  | I:w |\n"));
        assert!(
            errors[0].contains("\"two\" is not a voice layer number"),
            "{}",
            errors[0]
        );
        let errors: Vec<String> = errors_of(&song_with("  line \"Keys\"  layer 2  | I:w |\n"));
        assert!(
            errors[0].contains("after the track's name is not understood"),
            "{}",
            errors[0]
        );
        let errors: Vec<String> =
            errors_of(&song_with("  line \"Bass\"  voice_layer 2  | C2:w |\n"));
        assert!(errors[0].contains("only voice layer 1"), "{}", errors[0]);
        let errors: Vec<String> =
            errors_of(&song_with("  line \"Keys\"  voice_layer 0  | I:w |\n"));
        assert!(errors[0].contains("counted from 1"), "{}", errors[0]);
    }

    #[test]
    fn a_voice_layer_may_be_named_in_its_track() {
        let text: String = song_with("  line \"Keys\"  voice_layer 2  | C4:w |\n").replace(
            "  clef grand\n  lead\n",
            "  clef grand\n  lead\n  voice_layer 2 \"Inner\"\n",
        );
        let song: Song = parse_song("test.song", &text).unwrap();
        let keys: &Track = &song.tracks()[1];
        let two: VoiceLayerNumber = VoiceLayerNumber::new(2).unwrap();
        let three: VoiceLayerNumber = VoiceLayerNumber::new(3).unwrap();
        assert_eq!(keys.voice_layer_display_name(two), "Inner");
        assert_eq!(keys.voice_layer_display_name(three), "3");
        let unnamed: Vec<String> = errors_of(&song_with("  line \"Keys\"  | I:w |\n").replace(
            "  clef grand\n  lead\n",
            "  clef grand\n  lead\n  voice_layer 2\n",
        ));
        assert!(
            unnamed[0].contains("voice_layer 2 \"Inner\""),
            "{}",
            unnamed[0]
        );
    }
}
