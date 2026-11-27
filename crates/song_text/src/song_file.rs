//! The song file: `song "Name"`, its `track` blocks, and its `section`
//! blocks with a `line` for each track, or for each voice layer of a track
//! (Song Notation, section 5).
//!
//! A file with any mistake does not load, not even partly (EFS-6.3): the
//! result is the song or all the mistakes found, in line order.

use vocabulary::bar_meters::BarMeters;
use vocabulary::interval::nearest_tonic;
use vocabulary::key::Key;
use vocabulary::midi_values::{
    ControllerNumber, ControllerValue, NoteNumber, ProgramNumber, Velocity,
};
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::pitch::Pitch;
use vocabulary::score_length::ScoreLength;
use vocabulary::song::{
    Clef, CueDefinition, CueDestination, CueMessage, Line, LineRole, Section, Song, SongError,
    Track, TrackKind, TrackNotes, VoiceKind, VoiceLayerName, VoiceLayerNumber,
};
use vocabulary::tempo::{Tempo, TempoError};
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;
use vocabulary::time_signature::TimeSignature;

use crate::contents_text::{NoteReading, parse_key};
use crate::meter_pattern::parse_meter_pattern;
use crate::parse_error::ParseError;
use crate::song_line::{BarsText, build_line};
use crate::text_line::{Word, keyword_rest, parse_meter, quoted_prefix, without_comment, words};

/// A track block while it is being read.
struct OpenTrack {
    name: String,
    line_number: usize,
    type_name: Option<String>,
    /// `notes pitched` or `notes midi_note`.
    notes: Option<TrackNotes>,
    clef: Option<Clef>,
    lead: bool,
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
    /// `dynamics` written after the track's name: the line holds levels.
    dynamics: bool,
    /// `tonic_octave n`: the octave of the tonic a pitched line's intervals
    /// are measured from.
    tonic_octave: Option<i8>,
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
    /// `key section`: on a song composition entry, play the section's own
    /// key whatever key line stands above it. Refused anywhere else.
    plays_section_key: bool,
    time_signature: Option<TimeSignature>,
    /// `grouping 2+2+3` written after the meter, as written: the pulses of
    /// each group.
    beat_grouping: Option<Vec<u8>>,
    tempo: Option<WrittenTempo>,
    lines: Vec<RawLine>,
    /// True right after a `line` or its continuation: a text line starting
    /// with `|` then adds bars to that line.
    continuing_line: bool,
    errors_at_start: usize,
}

impl OpenSection {
    /// A block with nothing written yet.
    fn new(name: &str, line_number: usize, errors_at_start: usize) -> OpenSection {
        OpenSection {
            name: name.to_string(),
            line_number,
            count_in: false,
            repeat_count: None,
            key: None,
            plays_section_key: false,
            time_signature: None,
            beat_grouping: None,
            tempo: None,
            lines: Vec::new(),
            continuing_line: false,
            errors_at_start,
        }
    }
}

/// One entry of the `song_composition` block: a section to play, how many
/// times, and the key or tempo it plays in if the entry changes them.
struct CompositionEntry {
    name: String,
    line_number: usize,
    repeat_count: u32,
    key: Option<Key>,
    /// `key section`: the section's own key, whatever key line is above.
    plays_section_key: bool,
    tempo: Option<WrittenTempo>,
    /// True for a `key` or `tempo` line standing between entries: it plays
    /// nothing, and every entry after it takes its values as the base.
    rebase: bool,
}

/// The `song_composition` block while it is being read: the key, meter and
/// tempo the song starts in (read like a section's settings), then the
/// entries in play order.
struct OpenComposition {
    line_number: usize,
    start: OpenSection,
    entries: Vec<CompositionEntry>,
    closed: bool,
}

/// What an entry of the composition changes in the section it plays: the
/// key and tempo, which win over the section's own.
#[derive(Default)]
struct EntryOverrides {
    key: Option<Key>,
    tempo: Option<WrittenTempo>,
    /// The key comes from a key line between entries, not the entry itself.
    key_from_key_line: bool,
    /// Where the entry is written, for a key override that is refused. Only
    /// read when `key` is given, so the default 0 of no entry is never used.
    line_number: usize,
}

/// A tempo as the song file writes it. How long a beat is comes from the
/// section's meter, which may be written after the tempo, so the conversion
/// waits until the section closes.
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

/// The key and tempo the composition is in at an entry: its starting values,
/// then those of each `key` or `tempo` line above the entry. A section with
/// no key or tempo of its own plays in these. The tempo is kept with the
/// meter whose beat it counts, so it can be given the beat of a section's
/// meter and stay the same beats per minute.
#[derive(Clone)]
struct CompositionBase {
    /// The key a section with no key of its own plays in.
    key: Key,
    /// The key of the last key line between entries: an override for every
    /// entry below it (Song Notation, section 5). The starting key is not.
    key_line: Option<Key>,
    tempo: Tempo,
    /// The meter whose beat `tempo` counts: the composition's own meter.
    beat_meter: TimeSignature,
}

/// A tempo kept as beats per minute and given the beat of another meter. In
/// the same meter this is the same tempo.
///
/// ticks/beat of the old meter: `mbeats/min = mq/min × 5040 ticks/q ÷ L_beat
/// ticks/beat` (exact: the old tempo was a whole number of mq/min made from
/// whole mbeats/min).
fn tempo_in_meter(
    base_tempo: Tempo,
    base_time_signature: TimeSignature,
    time_signature: TimeSignature,
) -> Result<Tempo, String> {
    let old_beat_ticks: u64 = base_time_signature.beat_length().ticks();
    let milli_beats_per_minute: u64 = base_tempo
        .milli_quarters_per_minute()
        .saturating_mul(TICKS_PER_QUARTER_NOTE)
        / old_beat_ticks;
    let text: String = milli_beats_text(milli_beats_per_minute);
    WrittenTempo::BeatsPerMinute(MilliBeats {
        milli_beats: milli_beats_per_minute,
        text,
    })
    .to_tempo(time_signature)
}

/// 120000 → "120", 112500 → "112.5": a tempo's number as the song writes it.
fn milli_beats_text(milli_beats: u64) -> String {
    let whole: u64 = milli_beats / 1000;
    let thousandths: u64 = milli_beats % 1000;
    if thousandths == 0 {
        whole.to_string()
    } else {
        let decimals: String = format!("{thousandths:03}");
        format!("{whole}.{}", decimals.trim_end_matches('0'))
    }
}

/// `key section` is only for a song composition entry.
const SECTION_KEY_ON_ENTRY_ONLY: &str = "key section belongs on a song composition entry, where it \
     plays the section's own key. Here write a key, e.g. key C major";

struct SongReader<'a> {
    file_name: &'a str,
    errors: Vec<ParseError>,
    song_name: Option<(String, usize)>,
    /// The `chord_numeral_standard` line, once read.
    numeral_standard: Option<NumeralStandard>,
    tracks: Vec<Track>,
    /// Tracks that were written but are not usable, so their lines are not
    /// also reported.
    failed_track_names: Vec<String>,
    track: Option<OpenTrack>,
    /// The sections that play, in play order: built from the definitions
    /// when the file has been read, as the composition names them.
    sections: Vec<Section>,
    /// Every section as written. A section is built when the composition
    /// plays it, since its key, meter and tempo may come from what plays
    /// before it or from the entry.
    definitions: Vec<OpenSection>,
    section: Option<OpenSection>,
    /// Sections that were written but are not usable, so an entry naming one
    /// is not also reported.
    failed_section_names: Vec<String>,
    /// The `song_composition` block, once started.
    composition: Option<OpenComposition>,
    /// True while the lines of a refused second `song_composition` are being
    /// skipped, so they are not reported one by one.
    skipping_second_composition: bool,
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
        numeral_standard: None,
        tracks: Vec::new(),
        failed_track_names: Vec::new(),
        track: None,
        sections: Vec::new(),
        definitions: Vec::new(),
        section: None,
        failed_section_names: Vec::new(),
        composition: None,
        skipping_second_composition: false,
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
        if self.skipping_second_composition {
            if keyword_rest(line, "song_composition_close").is_some() {
                self.skipping_second_composition = false;
            }
            return;
        }
        let result: Result<(), String> = if keyword_rest(line, "song_composition_close").is_some() {
            self.close_composition(line_number)
        } else if let Some(rest) = keyword_rest(line, "song_composition") {
            self.start_composition(line_number, rest)
        } else if self
            .composition
            .as_ref()
            .is_some_and(|c: &OpenComposition| !c.closed)
        {
            self.read_composition_line(line_number, line)
        } else if let Some(rest) = keyword_rest(line, "song") {
            self.start_song(line_number, rest)
        } else if let Some(rest) = keyword_rest(line, "chord_numeral_standard") {
            self.read_numeral_standard(rest)
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

    /// `chord_numeral_standard key_dependent`, on the line after `song`.
    fn read_numeral_standard(&mut self, rest: &str) -> Result<(), String> {
        if self.song_name.is_none() || self.track.is_some() || !self.tracks.is_empty() {
            return Err(String::from(
                "write chord_numeral_standard on the line after song, before the first track",
            ));
        }
        if self.numeral_standard.is_some() {
            return Err(String::from(
                "a second chord_numeral_standard line. A song has one",
            ));
        }
        let standard: NumeralStandard = NumeralStandard::from_text(rest).ok_or_else(|| {
            format!(
                "\"{rest}\" is not a chord numeral standard. Write chord_numeral_standard \
                 key_dependent or chord_numeral_standard major_referential"
            )
        })?;
        self.numeral_standard = Some(standard);
        Ok(())
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
        if !self.definitions.is_empty() || self.composition.is_some() {
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
            notes: None,
            clef: None,
            lead: false,
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
        } else if let Some(rest) = keyword_rest(line, "notes") {
            track.notes = Some(match rest {
                "pitched" => TrackNotes::Pitched,
                "midi_note" => TrackNotes::MidiNote,
                other => {
                    return Err(format!(
                        "\"{other}\" is not what a track's notes can be. Write notes pitched \
                         for music, or notes midi_note for notes a device uses to pick a gate \
                         or a sample"
                    ));
                }
            });
        } else if let Some(rest) = keyword_rest(line, "clef") {
            track.clef = Some(parse_clef(rest)?);
        } else if line == "lead" {
            track.lead = true;
        } else if keyword_rest(line, "home_octave").is_some() {
            return Err(String::from(
                "home_octave no longer exists: the clef sets the octave. Move a passage with +8va \
                 or -8va at the start of a bar, and return with loco",
            ));
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
                "\"{line}\" is not understood in a track. Expected type, notes, clef, lead, \
                 voice_layer or cue"
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
        let mut section: OpenSection = OpenSection::new(name, line_number, self.errors.len());
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
            let (mut tonic_octave, bars_text): (Option<i8>, &str) =
                match read_tonic_octave(bars_text) {
                    Ok(read) => read,
                    Err(message) => {
                        section.continuing_line = false;
                        return Err(message);
                    }
                };
            let (sample_source, mut bars_text): (bool, &str) = read_sample_source(bars_text);
            if tonic_octave.is_none() {
                (tonic_octave, bars_text) = match read_tonic_octave(bars_text) {
                    Ok(read) => read,
                    Err(message) => {
                        section.continuing_line = false;
                        return Err(message);
                    }
                };
            }
            let (dynamics, bars_text): (bool, &str) = read_dynamics(bars_text);
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
                dynamics,
                tonic_octave,
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

    /// Keeps the open section as a definition, once its settings and lines
    /// have been read. It is built when the composition plays it, or checked
    /// when no entry does.
    fn finish_section(&mut self) {
        let Some(open) = self.section.take() else {
            return;
        };
        if self.errors.len() > open.errors_at_start {
            self.failed_section_names.push(open.name);
            return;
        }
        if open.plays_section_key {
            self.failed_section_names.push(open.name.clone());
            self.report(open.line_number, String::from(SECTION_KEY_ON_ENTRY_ONLY));
            return;
        }
        if open.repeat_count.is_some() {
            self.failed_section_names.push(open.name.clone());
            self.report(
                open.line_number,
                format!(
                    "section \"{0}\" has a repeat. A section is a definition: write repeat n on \
                     its entry in song_composition, e.g. \"{0}\" repeat 2",
                    open.name
                ),
            );
            return;
        }
        if open.lines.is_empty() {
            self.failed_section_names.push(open.name.clone());
            self.report(
                open.line_number,
                format!(
                    "section \"{0}\" has no lines. Write its lines. To play a section again, \
                     name it in song_composition, e.g. \"{0}\" repeat 2",
                    open.name
                ),
            );
            return;
        }
        if self
            .definitions
            .iter()
            .any(|defined: &OpenSection| defined.name == open.name)
        {
            self.report(
                open.line_number,
                format!(
                    "two sections are named \"{0}\". Give each section its own name; to play \
                     \"{0}\" again, name it twice in song_composition",
                    open.name
                ),
            );
            return;
        }
        self.definitions.push(open);
    }

    /// Builds a section as the composition plays it: with the key and tempo
    /// the entry gives, else the section's own, else the composition's
    /// current ones. Answers the section, or `None` after reporting why not.
    // One pass over a section's settings, lines and the composition's values; split
    // apart it would pass the same state between several helpers.
    #[allow(clippy::too_many_lines)]
    fn build_section(
        &mut self,
        open: &OpenSection,
        repeat_count: u32,
        overrides: &EntryOverrides,
        base: &CompositionBase,
    ) -> Option<Section> {
        let errors_before: usize = self.errors.len();
        // The entry's tempo wins over the section's own.
        let written_tempo: Option<&WrittenTempo> = overrides.tempo.as_ref().or(open.tempo.as_ref());
        let Ok(pattern) = self.meter_pattern_of(open) else {
            return None;
        };
        let bar_meters: BarMeters = match (pattern, open.time_signature) {
            (Some(_), Some(_)) => {
                self.report(
                    open.line_number,
                    format!(
                        "section \"{}\" writes a meter and also has a meter_control line. A \
                         section has one or the other: remove the meter, or the meter_control \
                         line",
                        open.name
                    ),
                );
                return None;
            }
            (Some(pattern), None) => pattern,
            (None, Some(time_signature)) => BarMeters::uniform(time_signature),
            (None, None) => {
                // A section never takes its meter from what plays before it,
                // so it can be edited without knowing the order of play.
                self.report(
                    open.line_number,
                    format!(
                        "section \"{}\" has no meter. Add meter 4/4 (or the meter you want), \
                         or a meter_control line",
                        open.name
                    ),
                );
                return None;
            }
        };
        // Every meter of a section has the same beat, so the first bar's
        // meter gives the beat of the section's tempo.
        let time_signature: TimeSignature = bar_meters.first();
        let tempo: Tempo = match written_tempo {
            Some(written) => match written.to_tempo(time_signature) {
                Ok(tempo) => tempo,
                Err(message) => {
                    self.report(
                        open.line_number,
                        format!("section \"{}\": {message}", open.name),
                    );
                    return None;
                }
            },
            None => match tempo_in_meter(base.tempo, base.beat_meter, time_signature) {
                Ok(tempo) => tempo,
                Err(message) => {
                    self.report(
                        open.line_number,
                        format!(
                            "section \"{}\": the composition's tempo does not fit this \
                             meter: {message}. Write a tempo for this section",
                            open.name
                        ),
                    );
                    return None;
                }
            },
        };
        // A pitched line's intervals are measured from the section's own
        // key; a count-in has no harmony and needs none.
        if open.key.is_none() && !open.count_in {
            let pitched: Option<&RawLine> = open.lines.iter().find(|raw: &&RawLine| {
                self.track_notes(&raw.track_name) == Some(TrackNotes::Pitched)
            });
            if let Some(raw) = pitched {
                self.report(
                    open.line_number,
                    format!(
                        "section \"{}\" has a line on track \"{}\", whose notes are pitched, and \
                         no key of its own. Add its key, e.g. key C major: its notes and chords \
                         are measured from the key's tonic",
                        open.name, raw.track_name
                    ),
                );
                return None;
            }
        }
        // An override changes the tonic only, so a major section stays major.
        if let (Some(over), Some(own)) = (overrides.key, open.key)
            && over.mode() != own.mode()
        {
            let message: String = if overrides.key_from_key_line {
                format!(
                    "\"{}\" is in {own}, and the key line above it overrides it with {over}. An \
                     override keeps major or minor: write key section on this entry to play \
                     {own}, or give it a {} key, e.g. \"{}\"  key {} {}",
                    open.name,
                    own.mode().text(),
                    open.name,
                    over.tonic(),
                    own.mode().text()
                )
            } else {
                format!(
                    "\"{}\" is in {own}. Its key override must be {} too, e.g. {} {}",
                    open.name,
                    own.mode().text(),
                    over.tonic(),
                    own.mode().text()
                )
            };
            self.report(overrides.line_number, message);
            return None;
        }
        // A count-in section needs no key, so it does not take the song's.
        let key: Option<Key> = if open.count_in {
            overrides.key.or(open.key)
        } else {
            Some(overrides.key.or(open.key).unwrap_or(base.key))
        };
        let lines: Vec<Line> = self.build_lines(open, &bar_meters, key, open.key);
        if self.errors.len() > errors_before {
            return None;
        }
        // A grouping goes with the meter it is written with.
        let beat_grouping: Option<Vec<u8>> = open.beat_grouping.clone();
        let built: Result<Section, SongError> = Section::new(
            &open.name,
            open.count_in,
            repeat_count,
            key,
            bar_meters.clone(),
            tempo,
            lines,
        )
        .and_then(|section: Section| match beat_grouping {
            Some(group_pulses) => section.with_beat_grouping(group_pulses),
            None => Ok(section),
        })
        .map(|section: Section| section.with_own_key(open.key));
        match built {
            Ok(section) => Some(section),
            Err(error) => {
                self.report(open.line_number, error.to_string());
                None
            }
        }
    }

    /// `song_composition`, with the song's starting key, meter and tempo on
    /// the line or the lines below it.
    fn start_composition(&mut self, line_number: usize, rest: &str) -> Result<(), String> {
        self.finish_track();
        self.finish_section();
        if self.composition.is_some() {
            self.skipping_second_composition = true;
            return Err(String::from(
                "a second song_composition. A song has only one song_composition",
            ));
        }
        let mut composition: OpenComposition = OpenComposition {
            line_number,
            start: OpenSection::new("song_composition", line_number, self.errors.len()),
            entries: Vec::new(),
            closed: false,
        };
        let result: Result<(), String> = if rest.is_empty() {
            Ok(())
        } else {
            read_composition_start(&mut composition.start, rest)
        };
        self.composition = Some(composition);
        result
    }

    /// A line inside the `song_composition` block: an entry (a section's name
    /// in quotes, then `repeat n`, `key` or `tempo` if it changes them), or
    /// the starting key, meter and tempo.
    fn read_composition_line(&mut self, line_number: usize, line: &str) -> Result<(), String> {
        if keyword_rest(line, "section").is_some() || keyword_rest(line, "track").is_some() {
            return Err(String::from(
                "end song_composition with song_composition_close before a track or a section",
            ));
        }
        let Some(composition) = self.composition.as_mut() else {
            return Ok(());
        };
        if !line.starts_with('"') {
            if composition.entries.is_empty() {
                read_composition_start(&mut composition.start, line)?;
                if composition.start.plays_section_key {
                    return Err(String::from(SECTION_KEY_ON_ENTRY_ONLY));
                }
                return Ok(());
            }
            // After the first entry, a key or tempo line sets a new base for
            // the entries below it.
            let mut written: OpenSection = OpenSection::new("", line_number, 0);
            read_composition_start(&mut written, line)?;
            if written.time_signature.is_some() {
                return Err(String::from(
                    "the meter cannot change in song_composition. Write the meter on a section",
                ));
            }
            if written.plays_section_key {
                return Err(String::from(SECTION_KEY_ON_ENTRY_ONLY));
            }
            composition.entries.push(CompositionEntry {
                name: String::new(),
                line_number,
                repeat_count: 1,
                key: written.key,
                plays_section_key: false,
                tempo: written.tempo,
                rebase: true,
            });
            return Ok(());
        }
        let Some((name, settings)) = quoted_prefix(line) else {
            return Err(String::from(
                "write the section's name in quotes, e.g. \"Verse\" repeat 2",
            ));
        };
        let mut written: OpenSection = OpenSection::new(name, line_number, 0);
        read_settings(&mut written, settings)?;
        if written.time_signature.is_some() {
            return Err(format!(
                "\"{name}\": an entry may change the key and the tempo, not the meter. \
                 Write the meter on the section"
            ));
        }
        if written.count_in {
            return Err(format!(
                "\"{name}\": count-in belongs on the section, not on its entry"
            ));
        }
        composition.entries.push(CompositionEntry {
            name: name.to_string(),
            line_number,
            repeat_count: written.repeat_count.unwrap_or(1),
            key: written.key,
            plays_section_key: written.plays_section_key,
            tempo: written.tempo,
            rebase: false,
        });
        Ok(())
    }

    fn close_composition(&mut self, line_number: usize) -> Result<(), String> {
        match self.composition.as_mut() {
            Some(composition) if !composition.closed => {
                composition.closed = true;
                Ok(())
            }
            _ => {
                let _ = line_number;
                Err(String::from(
                    "song_composition_close with no song_composition above it",
                ))
            }
        }
    }

    /// The key, meter and tempo the composition starts in, or `None` after
    /// reporting what is missing or wrong.
    fn starting_values_of(&mut self, composition: &OpenComposition) -> Option<CompositionBase> {
        let start: &OpenSection = &composition.start;
        let (Some(key), Some(time_signature), Some(written_tempo)) =
            (start.key, start.time_signature, start.tempo.clone())
        else {
            self.report(
                composition.line_number,
                String::from(
                    "song_composition needs the key, meter and tempo the song starts in, e.g. \
                     key C major  meter 4/4  tempo 100 beats/min",
                ),
            );
            return None;
        };
        let tempo: Tempo = match written_tempo.to_tempo(time_signature) {
            Ok(tempo) => tempo,
            Err(message) => {
                self.report(
                    composition.line_number,
                    format!("song_composition: {message}"),
                );
                return None;
            }
        };
        Some(CompositionBase {
            key,
            key_line: None,
            tempo,
            beat_meter: time_signature,
        })
    }

    /// Plays the composition's entries in order: each is built from its
    /// definition with the key and tempo the entry gives, else its own, else
    /// the composition's current ones. A definition no entry plays is checked once, as
    /// written, against the composition's starting key, meter and tempo.
    fn build_composition(&mut self, song_line: usize) {
        let Some(composition) = self.composition.take() else {
            self.report(
                song_line,
                String::from(
                    "the song has no song_composition. List the sections that play between \
                     song_composition and song_composition_close",
                ),
            );
            return;
        };
        if !composition.closed {
            self.report(
                composition.line_number,
                String::from("song_composition is not closed. End it with song_composition_close"),
            );
            return;
        }
        let Some(starting) = self.starting_values_of(&composition) else {
            return;
        };
        let definitions: Vec<OpenSection> = std::mem::take(&mut self.definitions);
        let mut base: CompositionBase = starting.clone();
        let mut played_names: Vec<String> = Vec::new();
        let mut failed_names: Vec<String> = Vec::new();
        for entry in &composition.entries {
            if entry.rebase {
                self.rebase(&mut base, entry);
                continue;
            }
            let Some(definition) = definitions
                .iter()
                .find(|defined: &&OpenSection| defined.name == entry.name)
            else {
                if self.failed_section_names.contains(&entry.name) {
                    continue; // already reported where it is written
                }
                self.report(
                    entry.line_number,
                    format!(
                        "\"{0}\" is not a section of this song. Define it with section \"{0}\" \
                         and its lines, or fix the name",
                        entry.name
                    ),
                );
                continue;
            };
            if failed_names.contains(&entry.name) {
                continue;
            }
            // The entry's own override, else (unless it plays its section's
            // key) the key line above it.
            let key_from_key_line: bool =
                entry.key.is_none() && !entry.plays_section_key && base.key_line.is_some();
            let overrides: EntryOverrides = EntryOverrides {
                key: if entry.plays_section_key {
                    None
                } else {
                    entry.key.or(base.key_line)
                },
                tempo: entry.tempo.clone(),
                key_from_key_line,
                line_number: entry.line_number,
            };
            // repeat 0 leaves the entry out, as if commented out: it is
            // checked where it stands, but does not play or change anything.
            if entry.repeat_count == 0 {
                if self
                    .build_section(definition, 1, &overrides, &base)
                    .is_some()
                {
                    played_names.push(entry.name.clone());
                } else {
                    failed_names.push(entry.name.clone());
                }
                continue;
            }
            match self.build_section(definition, entry.repeat_count, &overrides, &base) {
                Some(section) => {
                    self.sections.push(section);
                    played_names.push(entry.name.clone());
                }
                None => failed_names.push(entry.name.clone()),
            }
        }
        for definition in &definitions {
            if played_names.contains(&definition.name) || failed_names.contains(&definition.name) {
                continue;
            }
            // Not played: still checked, so a mistake is found when written.
            let _ = self.build_section(definition, 1, &EntryOverrides::default(), &starting);
        }
        if self.errors.is_empty() && self.sections.is_empty() {
            let message: &str = if composition.entries.is_empty() {
                "song_composition lists no sections. Name the sections that play, e.g. \"Verse\""
            } else {
                "every entry in song_composition has repeat 0, so nothing plays. Give one of \
                 them repeat 1 or more"
            };
            self.report(composition.line_number, String::from(message));
        }
    }

    /// A `key` or `tempo` line between entries: the new base for the entries
    /// below it.
    fn rebase(&mut self, base: &mut CompositionBase, entry: &CompositionEntry) {
        if let Some(key) = entry.key {
            base.key = key;
            base.key_line = Some(key);
        }
        let Some(written) = entry.tempo.clone() else {
            return;
        };
        match written.to_tempo(base.beat_meter) {
            Ok(tempo) => base.tempo = tempo,
            Err(message) => self.report(entry.line_number, format!("song_composition: {message}")),
        }
    }

    /// The meter pattern of the section's `meter_control` line, if it has one.
    /// A mistake is reported and answered with `Err`.
    fn meter_pattern_of(&mut self, open: &OpenSection) -> Result<Option<BarMeters>, ()> {
        let mut found: Option<BarMeters> = None;
        for raw in &open.lines {
            let is_meter_line: bool = self.tracks.iter().any(|track: &Track| {
                track.name() == raw.track_name && matches!(track.kind(), TrackKind::MeterControl)
            });
            if !is_meter_line {
                continue;
            }
            if found.is_some() {
                self.report(
                    raw.line_number,
                    format!(
                        "a second meter_control line in section \"{}\". A section has one",
                        open.name
                    ),
                );
                return Err(());
            }
            if raw.voice_layer.is_some()
                || raw.sample_source
                || raw.dynamics
                || raw.tonic_octave.is_some()
            {
                self.report(
                    raw.line_number,
                    String::from(
                        "a meter_control line takes only its bars, e.g. line \"Meter\" | mp [3_4 5_4] |",
                    ),
                );
                return Err(());
            }
            let text: String = raw
                .parts
                .iter()
                .map(|part: &RawBarsText| part.text.as_str())
                .collect::<Vec<&str>>()
                .join(" ");
            let line_number: usize = raw
                .parts
                .first()
                .map_or(raw.line_number, |part| part.line_number);
            match parse_meter_pattern(&text) {
                Ok(pattern) => found = Some(pattern),
                Err(message) => {
                    self.report(
                        line_number,
                        format!("line \"{}\": {message}", raw.track_name),
                    );
                    return Err(());
                }
            }
        }
        Ok(found)
    }

    /// A voice track's `notes`, by the track's name.
    fn track_notes(&self, track_name: &str) -> Option<TrackNotes> {
        self.tracks
            .iter()
            .find(|track: &&Track| track.name() == track_name)
            .and_then(|track: &Track| match track.kind() {
                TrackKind::Voice { notes, .. } => Some(*notes),
                _ => None,
            })
    }

    /// How `raw`'s notes are read: a pitched line's intervals from `key`'s
    /// tonic nearest `own_key`'s in its `tonic_octave`, a MIDI note track's
    /// note names. A
    /// `tonic_octave` on a track that takes note names is refused.
    fn note_reading(
        raw: &RawLine,
        track: &Track,
        key: Option<Key>,
        own_key: Option<Key>,
    ) -> Result<NoteReading, String> {
        let reading: NoteReading = match (track.kind(), raw.tonic_octave) {
            (
                TrackKind::Voice {
                    notes: TrackNotes::Pitched,
                    ..
                },
                Some(octave),
            ) => match key {
                Some(key) => {
                    // The section's own tonic in the line's octave; under an
                    // override, the override's tonic nearest it.
                    let own_tonic: Pitch = Pitch::new(own_key.unwrap_or(key).tonic(), octave);
                    NoteReading::Intervals {
                        tonic: Some(nearest_tonic(own_tonic, key.tonic())),
                        missing: "",
                    }
                }
                None => NoteReading::Intervals {
                    tonic: None,
                    missing: "the section has no key, so an interval has no tonic. Add the \
                              section's key, e.g. key C major",
                },
            },
            (
                TrackKind::Voice {
                    notes: TrackNotes::Pitched,
                    ..
                },
                None,
            ) => NoteReading::Intervals {
                tonic: None,
                missing: "this line writes intervals and has no tonic_octave. Write the \
                          octave of the tonic they are measured from after the track's \
                          name, e.g. tonic_octave 2",
            },
            (_, Some(_)) => {
                return Err(format!(
                    "line \"{}\": tonic_octave is for a track whose notes are pitched. \
                     Remove it: this track's notes are written as note names",
                    raw.track_name
                ));
            }
            (_, None) => NoteReading::NoteNames,
        };
        Ok(reading)
    }

    /// The lines of a section as an entry plays it: a pitched line's
    /// intervals are read from `key`'s tonic, the key the entry plays in.
    fn build_lines(
        &mut self,
        open: &OpenSection,
        bar_meters: &BarMeters,
        key: Option<Key>,
        own_key: Option<Key>,
    ) -> Vec<Line> {
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
            // The meter pattern is read before the lines; it is not a line.
            if matches!(track.kind(), TrackKind::MeterControl) {
                continue;
            }
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
                (None, _) if raw.dynamics => LineRole::Dynamics,
                (number, _) if raw.sample_source => {
                    LineRole::SampleSource(number.unwrap_or(VoiceLayerNumber::FIRST))
                }
                (Some(number), _) => LineRole::VoiceLayer(number),
                (None, TrackKind::CueControl { .. }) => LineRole::CuePlacement,
                (
                    None,
                    TrackKind::Voice { .. }
                    | TrackKind::ClockAndTransport
                    | TrackKind::MeterControl,
                ) => LineRole::VoiceLayer(VoiceLayerNumber::FIRST),
            };
            let reading: NoteReading = match Self::note_reading(raw, track, key, own_key) {
                Ok(reading) => reading,
                Err(message) => {
                    self.report(raw.line_number, message);
                    continue;
                }
            };
            match build_line(
                track,
                role,
                raw.line_number,
                &parts,
                bar_meters.clone(),
                reading,
            ) {
                Ok(line) => {
                    let tonic: Option<Pitch> = match reading {
                        NoteReading::Intervals { tonic, .. } => tonic,
                        NoteReading::NoteNames => None,
                    };
                    lines.push(line.with_tonic(raw.tonic_octave, tonic));
                }
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
        self.build_composition(name_line);
        if !self.errors.is_empty() {
            // The sections are built at the end, so put the mistakes in line order.
            self.errors.sort_by_key(ParseError::line_number);
            return Err(self.errors);
        }
        let numeral_standard: Option<NumeralStandard> = self.numeral_standard;
        match Song::new(&name, self.tracks, self.sections)
            .and_then(|song: Song| song.with_numeral_standard(numeral_standard))
        {
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
            let Some(notes) = open.notes else {
                return Err(format!(
                    "track \"{}\" does not say what its notes are. Add notes pitched for music, \
                     or notes midi_note for notes a device uses to pick a gate or a sample",
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
                notes,
                clef,
                lead: open.lead,
                voice_layer_names: open.voice_layer_names.clone(),
            }
        }
        _ if open.clef.is_some() => return Err(voice_only("clef")),
        _ if open.notes.is_some() => return Err(voice_only("notes")),
        _ if !open.voice_layer_names.is_empty() => return Err(voice_only("voice_layer")),
        _ if open.lead => return Err(voice_only("lead")),
        "clock_and_transport" => TrackKind::ClockAndTransport,
        "cue_control" => TrackKind::CueControl {
            cues: open.cues.clone(),
        },
        "meter_control" => TrackKind::MeterControl,
        "tempo_control" => {
            return Err(format!(
                "track \"{}\": tempo_control no longer exists. Write a tempo mark at the start of a \
                 bar of the lead track's line, e.g. | tempo 130 beats/min I:q / / / |",
                open.name
            ));
        }
        "musician_voice" | "automation_control" => {
            return Err(format!(
                "track \"{}\": {type_name} is planned but not in V1",
                open.name
            ));
        }
        other => {
            return Err(format!(
                "track \"{}\": \"{other}\" is not a track type. Use poly_voice, mono_voice, \
                 clock_and_transport, cue_control or meter_control",
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
            || keyword_rest(after_name.trim(), "tonic_octave").is_some()
            || keyword_rest(after_name.trim(), "dynamics").is_some()
        {
            return Ok((None, after_name));
        }
        return Err(format!(
            "\"{}\" after the track's name is not understood. Write voice_layer and its \
             number, tonic_octave, sample_source, dynamics, or the bars, e.g. line \"Keys\" voice_layer 2 | C4:q D4 E4 F4 |",
            after_name.trim()
        ));
    };
    // Nothing after voice_layer reads as an empty number, which the next
    // step reports as not a number: no value is invented.
    let number_text: &str = rest.split_whitespace().next().unwrap_or("");
    let number: VoiceLayerNumber = voice_layer_number(number_text)?;
    Ok((Some(number), rest[number_text.len()..].trim_start()))
}

/// `tonic_octave 2` and the text after it; the text as it is when the word
/// is not there.
fn read_tonic_octave(after: &str) -> Result<(Option<i8>, &str), String> {
    let Some(rest) = keyword_rest(after.trim(), "tonic_octave") else {
        return Ok((None, after));
    };
    // Nothing after the word reads as an empty number, which is reported
    // as not a number: no value is invented.
    let number_text: &str = rest.split_whitespace().next().unwrap_or("");
    let octave: i8 = number_text
        .parse::<i8>()
        .ok()
        .filter(|octave: &i8| (-1..=9).contains(octave))
        .ok_or_else(|| {
            format!(
                "\"{number_text}\" is not an octave for tonic_octave. Write -1 to 9, e.g. \
                 tonic_octave 2"
            )
        })?;
    Ok((Some(octave), rest[number_text.len()..].trim_start()))
}

/// `sample_source` right after the track's name or its voice layer, and the
/// text after it; the text as it is when the word is not there.
fn read_sample_source(after: &str) -> (bool, &str) {
    match keyword_rest(after.trim(), "sample_source") {
        Some(rest) => (true, rest),
        None => (false, after),
    }
}

/// `dynamics` right after the track's name, and the text after it; the text
/// as it is when the word is not there.
fn read_dynamics(after: &str) -> (bool, &str) {
    match keyword_rest(after.trim(), "dynamics") {
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

/// The key, meter and tempo of the `song_composition` block, read like a
/// section's settings. `repeat` and `count-in` belong to entries and
/// sections.
fn read_composition_start(start: &mut OpenSection, text: &str) -> Result<(), String> {
    read_settings(start, text)?;
    if start.repeat_count.is_some() {
        return Err(String::from(
            "repeat belongs on an entry, e.g. \"Verse\" repeat 2",
        ));
    }
    if start.count_in {
        return Err(String::from("count-in belongs on a section"));
    }
    Ok(())
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
    if let [word, rest @ ..] = after
        && word.text == "section"
    {
        if section.key.is_some() || section.plays_section_key {
            return Err(String::from("key is written twice here"));
        }
        section.plays_section_key = true;
        return Ok(rest);
    }
    let [tonic, mode, rest @ ..] = after else {
        return Err(String::from(
            "key needs a note and a mode, e.g. key C major",
        ));
    };
    let key: Key = parse_key(&tonic.text, &mode.text)?;
    if section.plays_section_key || section.key.replace(key).is_some() {
        return Err(String::from("key is written twice in this section"));
    }
    Ok(rest)
}

fn read_meter<'a>(section: &mut OpenSection, after: &'a [Word]) -> Result<&'a [Word], String> {
    let Some((meter, mut rest)) = after.split_first() else {
        return Err(String::from("meter needs two numbers, e.g. meter 4/4"));
    };
    let time_signature: TimeSignature = parse_meter(&meter.text)?;
    if section.time_signature.replace(time_signature).is_some() {
        return Err(String::from("meter is written twice in this section"));
    }
    // `grouping 2+2+3` belongs to the meter it follows.
    if let Some((word, after_grouping)) = rest.split_first()
        && word.text == "grouping"
        && !word.quoted
    {
        let Some((groups, after_groups)) = after_grouping.split_first() else {
            return Err(String::from(
                "grouping needs the sizes of the groups, e.g. meter 7/8 grouping 2+2+3",
            ));
        };
        section.beat_grouping = Some(parse_grouping(&groups.text)?);
        rest = after_groups;
    }
    Ok(rest)
}

/// `2+2+3` → the pulses of each group. Whether they fill the bar is checked
/// when the section is built, against its meter.
fn parse_grouping(text: &str) -> Result<Vec<u8>, String> {
    let wrong = || {
        format!(
            "\"{text}\" is not a grouping. Write the pulses of each group with a plus between, \
             e.g. grouping 2+2+3"
        )
    };
    text.split('+')
        .map(|group: &str| group.parse::<u8>().map_err(|_| wrong()))
        .collect()
}

/// A tempo's number as whole milli-beats: digits, then optionally a point
/// and one to three decimals (`120`, `112.5`, `0.125`). `None` for anything
/// else, including a fourth decimal, which would not be held exactly.
pub(crate) fn parse_milli_beats(text: &str) -> Option<u64> {
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

    const DEMO: &str = include_str!(
        "../../../songs_canonical/count_in_and_verse/song_test_count_in_and_verse.song"
    );

    fn errors_of(text: &str) -> Vec<String> {
        parse_song("test.song", text)
            .unwrap_err()
            .iter()
            .map(ToString::to_string)
            .collect()
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

    // Every song below is a file in songs_canonical/song_file/. Each file
    // starts with two comment lines, so a line number in a message is the
    // number the file shows.

    #[test]
    fn a_line_for_an_undeclared_track_names_the_track() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/undeclared_track.song"
        ));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("test.song line 17:"));
        assert!(errors[0].contains("\"Drums\" is not a track of the song"));
    }

    #[test]
    fn a_chord_on_the_bass_is_refused_with_the_place() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/chord_on_bass.song"
        ));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("bar 1 beat 4:"), "{}", errors[0]);
    }

    #[test]
    fn lines_of_different_length_are_reported_at_the_section() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/lines_of_different_length.song"
        ));
        assert!(errors[0].contains("same number of bars"), "{}", errors[0]);
    }

    #[test]
    fn a_song_without_a_lead_or_clock_is_refused() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/no_lead_or_clock.song"
        ));
        assert!(errors[0].contains("clock and transport"), "{}", errors[0]);
    }

    #[test]
    fn a_composition_without_meter_and_tempo_is_refused() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/missing_meter_and_tempo.song"
        ));
        assert!(
            errors[0].contains("song_composition needs the key, meter and tempo"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn mistakes_in_a_track_block_are_all_reported() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/track_block_mistakes.song"
        ));
        // A third error follows (section "S" has no usable line, as track "A"
        // failed): the two track mistakes come first.
        assert!(errors.len() >= 2, "{errors:?}");
        assert!(errors[0].contains("test.song line 8:"));
        assert!(errors[1].contains("\"bass_synth\" is not a track type"));
    }

    #[test]
    fn a_cue_not_defined_is_refused() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/cue_not_defined.song"
        ));
        assert!(errors[0].contains("no cue named \"Stop\""), "{}", errors[0]);
    }

    /// (name, repeat count, tempo in milli-quarters/min) of each section that plays.
    fn plays_of(text: &str) -> Vec<(String, u32, u64)> {
        parse_song("test.song", text)
            .unwrap()
            .sections()
            .iter()
            .map(|section: &Section| {
                (
                    section.name().to_string(),
                    section.repeat_count(),
                    section.tempo().milli_quarters_per_minute(),
                )
            })
            .collect()
    }

    fn named_plays(plays: &[(&str, u32, u64)]) -> Vec<(String, u32, u64)> {
        plays
            .iter()
            .map(|&(name, repeats, tempo)| (name.to_string(), repeats, tempo))
            .collect()
    }

    #[test]
    fn a_section_with_repeat_0_is_left_out_as_if_commented_out() {
        // B (140 beats/min) is not played and does not change the tempo for the entries after it:
        // C plays at A's 100 beats/min = 100000 milli-quarters/min.
        assert_eq!(
            plays_of(include_str!(
                "../../../songs_canonical/song_file/section_repeat_0.song"
            )),
            named_plays(&[("A", 2, 100_000), ("C", 1, 100_000)])
        );
    }

    #[test]
    fn a_section_named_twice_in_the_composition_plays_twice() {
        assert_eq!(
            plays_of(include_str!(
                "../../../songs_canonical/song_file/section_played_again.song"
            )),
            named_plays(&[
                ("A", 1, 100_000),
                ("B", 1, 140_000),
                // B's own tempo affects B only.
                ("A", 3, 100_000),
                ("D", 1, 100_000),
            ])
        );
    }

    #[test]
    fn a_section_with_repeat_0_can_be_played_by_name_later() {
        assert_eq!(
            plays_of(include_str!(
                "../../../songs_canonical/song_file/section_repeat_0_then_again.song"
            )),
            named_plays(&[("A", 2, 100_000)])
        );
    }

    #[test]
    fn a_composition_entry_must_name_a_defined_section() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/composition_names_unknown_section.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("\"B\" is not a section of this song"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_section_with_a_repeat_is_refused() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/section_with_repeat.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("section \"A\" has a repeat"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn two_sections_with_lines_may_not_share_a_name() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/section_two_with_one_name.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("two sections are named \"A\""),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_song_whose_sections_all_have_repeat_0_is_refused() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/section_all_repeat_0.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("every entry in song_composition has repeat 0"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_composition_entry_may_change_key_and_tempo() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/composition_entry_overrides.song"),
        )
        .unwrap();
        let played: Vec<(String, u64)> = song
            .sections()
            .iter()
            .map(|section: &Section| {
                (
                    format!("{:?}", section.key()),
                    section.tempo().milli_quarters_per_minute(),
                )
            })
            .collect();
        assert_eq!(played.len(), 3);
        assert!(
            played[0].0.contains('C') && played[0].1 == 100_000,
            "{played:?}"
        );
        assert!(
            played[1].0.contains('D') && played[1].1 == 120_000,
            "{played:?}"
        );
        assert_eq!(
            played[2], played[0],
            "an entry's override holds for that entry only"
        );
    }

    #[test]
    fn a_key_and_tempo_line_between_entries_sets_the_base_for_those_below() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/composition_rebase.song"),
        )
        .unwrap();
        let played: Vec<(String, u64)> = song
            .sections()
            .iter()
            .map(|section: &Section| {
                (
                    format!("{:?}", section.key()),
                    section.tempo().milli_quarters_per_minute(),
                )
            })
            .collect();
        assert_eq!(played.len(), 3);
        assert!(
            played[0].0.contains('C') && played[0].1 == 100_000,
            "{played:?}"
        );
        assert_eq!(played[1], played[2], "both follow the new base");
        // The key line overrides "Riff"'s own C major, and the tempo line
        // sets the tempo.
        assert!(
            played[1].0.contains('D') && played[1].1 == 190_000,
            "{played:?}"
        );
    }

    /// Each pitch a track's line plays, entry by entry.
    fn pitches_of_track(song: &Song, track: &str) -> Vec<Vec<String>> {
        song.sections()
            .iter()
            .map(|section: &Section| {
                section
                    .lines()
                    .iter()
                    .find(|line: &&Line| line.track_name() == track)
                    .unwrap()
                    .elements()
                    .iter()
                    .filter_map(|line_element: &vocabulary::song::LineElement| {
                        match line_element.contents() {
                            Some(ElementContents::Notes(notes)) => Some(notes[0].to_string()),
                            _ => None,
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_key_override_keeps_the_sections_intervals() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/key_override_keeps_intervals.song"),
        )
        .unwrap();
        // One section, composed once: the override plays it on the new
        // tonic, the chromatic A1 included (C#2 becomes E2, not Fb2).
        assert_eq!(
            pitches_of_track(&song, "Bass"),
            vec![
                vec!["C2", "C#2", "G2", "B1"],
                vec!["Eb2", "E2", "Bb2", "D2"]
            ]
        );
    }

    #[test]
    fn a_midi_note_track_is_never_transposed() {
        let song: Song = parse_song(
            "test.song",
            include_str!(
                "../../../songs_canonical/song_file/midi_note_track_with_several_notes.song"
            ),
        )
        .unwrap();
        assert_eq!(
            pitches_of_track(&song, "Beat"),
            vec![vec!["C4", "D4", "A4", "A4"], vec!["C4", "D4", "A4", "A4"]]
        );
        assert_eq!(
            pitches_of_track(&song, "Keys"),
            vec![vec!["C4", "E4"], vec!["Eb4", "G4"]]
        );
    }

    #[test]
    fn a_key_line_overrides_the_entries_below_it() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/key_line_overrides.song"),
        )
        .unwrap();
        let keys: Vec<String> = song
            .sections()
            .iter()
            .map(|section: &Section| {
                section
                    .key()
                    .map_or_else(String::new, |key| key.to_string())
            })
            .collect();
        assert_eq!(keys, ["C major", "F major", "E major", "C major"]);
        assert_eq!(
            pitches_of_track(&song, "Bass"),
            vec![vec!["C2"], vec!["F2"], vec!["E2"], vec!["C2"]]
        );
    }

    #[test]
    fn an_override_moves_to_the_nearest_tonic() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/key_override_nearest_tonic.song"),
        )
        .unwrap();
        assert_eq!(
            pitches_of_track(&song, "Bass"),
            vec![vec!["C2"], vec!["B1"], vec!["F#1"], vec!["E2"]]
        );
    }

    #[test]
    fn a_key_line_of_the_other_mode_is_refused_at_the_entry() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/key_line_other_mode.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 21:"), "{errors:?}");
        assert!(errors[0].contains("write key section"), "{errors:?}");
    }

    #[test]
    fn key_section_is_only_for_an_entry() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/key_section_on_a_section.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 14:"), "{errors:?}");
        assert!(errors[0].contains("key section belongs on a song composition entry"));
    }

    #[test]
    fn a_key_override_of_the_other_mode_is_refused() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/key_override_other_mode.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(
            errors[0],
            "test.song line 20: \"Riff\" is in C major. Its key override must be major too, \
             e.g. A major"
        );
    }

    #[test]
    fn a_voice_track_must_say_what_its_notes_are() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/track_without_notes.song"
        ));
        assert!(errors[0].starts_with("test.song line 8:"), "{errors:?}");
        assert!(errors[0].contains("Add notes pitched"), "{errors:?}");
    }

    #[test]
    fn a_section_with_a_pitched_line_needs_its_own_key() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/pitched_section_without_key.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 14:"), "{errors:?}");
        assert!(errors[0].contains("Add its key"), "{errors:?}");
    }

    #[test]
    fn each_track_takes_only_its_own_notation() {
        let name_on_pitched: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/note_name_on_pitched_track.song"
        ));
        assert_eq!(name_on_pitched.len(), 1, "{name_on_pitched:?}");
        assert!(
            name_on_pitched[0].starts_with("test.song line 15:")
                && name_on_pitched[0].contains("\"C2\" is a note name"),
            "{name_on_pitched:?}"
        );
        let interval_on_midi: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/interval_on_midi_note_track.song"
        ));
        assert_eq!(interval_on_midi.len(), 1, "{interval_on_midi:?}");
        assert!(
            interval_on_midi[0].starts_with("test.song line 21:")
                && interval_on_midi[0].contains("\"P5\" is an interval"),
            "{interval_on_midi:?}"
        );
    }

    #[test]
    fn a_section_without_a_meter_is_refused() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/section_without_meter.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("section \"B\" has no meter"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_composition_entry_may_not_change_the_meter() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/composition_entry_meter.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("an entry may change the key and the tempo, not the meter"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_section_defined_after_the_composition_plays() {
        assert_eq!(
            plays_of(include_str!(
                "../../../songs_canonical/song_file/composition_forward_references.song"
            )),
            named_plays(&[
                ("Verse", 1, 100_000),
                ("Chorus", 1, 100_000),
                ("Verse", 1, 100_000),
            ])
        );
    }

    #[test]
    fn a_song_has_exactly_one_composition() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/composition_two_blocks.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("only one song_composition"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_song_without_a_composition_is_refused() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/composition_missing.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("has no song_composition"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_composition_must_be_closed() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/composition_unclosed.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("End it with song_composition_close"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_count_in_is_only_allowed_on_the_first_section_played() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/composition_late_count_in.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("count-in"), "{}", errors[0]);
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
            tempos_of(include_str!(
                "../../../songs_canonical/song_file/tempo_4_4_and_6_8.song"
            )),
            vec![120_000, 90_000]
        );
        // 3/8: eighth-note beats. 30 beats/min × 0.5 q/beat = 15 q/min.
        assert_eq!(
            tempos_of(include_str!(
                "../../../songs_canonical/song_file/tempo_3_8_and_9_8.song"
            )),
            vec![15_000, 90_000]
        );
    }

    #[test]
    fn a_tempo_may_come_before_its_meter_and_in_beats_per_second() {
        // 2 beats/s × 60 s/min = 120 dotted quarters/min × 1.5 q = 180 q/min.
        assert_eq!(
            tempos_of(include_str!(
                "../../../songs_canonical/song_file/tempo_before_meter_and_per_second.song"
            )),
            vec![120_000, 180_000]
        );
    }

    #[test]
    fn home_octave_is_refused_and_says_what_replaced_it() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/home_octave_removed.song"
        ));
        // The refused track leaves its section without a line: a second
        // message follows from the first, so only the first is checked.
        assert!(errors[0].starts_with("test.song line 12:"), "{}", errors[0]);
        assert!(errors[0].contains("+8va"), "{}", errors[0]);
    }

    #[test]
    fn the_composition_tempo_keeps_its_beats_per_minute_across_meters() {
        // 120 beats/min in 4/4 stays 120 beats/min in 3/4 (a quarter note
        // beat both times: 120 q/min) and in 6/8 (a dotted quarter beat:
        // 120 beats/min × 1.5 q/beat = 180 q/min), and back in 4/4.
        assert_eq!(
            tempos_of(include_str!(
                "../../../songs_canonical/song_file/tempo_across_meter_change.song"
            )),
            vec![120_000, 120_000, 180_000, 120_000]
        );
    }

    #[test]
    fn a_note_value_tempo_says_to_write_beats() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/tempo_in_note_values.song"
        ));
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
        // "tempo 120", "tempo 112.1234 beats/min", "tempo .5 beats/min",
        // "tempo 112. beats/min", "tempo 120 bpm" and "tempo 0 beats/min".
        for (written, text) in [
            (
                "tempo 120",
                include_str!("../../../songs_canonical/song_file/tempo_refused_1.song"),
            ),
            (
                "tempo 112.1234 beats/min",
                include_str!("../../../songs_canonical/song_file/tempo_refused_2.song"),
            ),
            (
                "tempo .5 beats/min",
                include_str!("../../../songs_canonical/song_file/tempo_refused_3.song"),
            ),
            (
                "tempo 112. beats/min",
                include_str!("../../../songs_canonical/song_file/tempo_refused_4.song"),
            ),
            (
                "tempo 120 bpm",
                include_str!("../../../songs_canonical/song_file/tempo_refused_5.song"),
            ),
            (
                "tempo 0 beats/min",
                include_str!("../../../songs_canonical/song_file/tempo_refused_6.song"),
            ),
        ] {
            let errors: Vec<String> = errors_of(text);
            assert_eq!(errors.len(), 1, "{written}: {errors:?}");
            assert!(errors[0].starts_with("test.song line 16:"), "{}", errors[0]);
        }
    }

    #[test]
    fn a_fractional_tempo_is_held_exactly() {
        // 4/4: 112.5 beats/min × 1 q/beat = 112500 mq/min. 6/8: 100.5
        // beats/min × 1.5 q/beat = 150.75 q/min = 150750 mq/min.
        for (text, expected_milli_quarters_per_minute) in [
            (
                include_str!("../../../songs_canonical/song_file/fractional_tempo_4_4_112_5.song"),
                112_500_u64,
            ),
            (
                include_str!(
                    "../../../songs_canonical/song_file/fractional_tempo_4_4_112_125.song"
                ),
                112_125,
            ),
            (
                include_str!("../../../songs_canonical/song_file/fractional_tempo_6_8_100_5.song"),
                150_750,
            ),
            (
                include_str!("../../../songs_canonical/song_file/fractional_tempo_per_second.song"),
                30_000,
            ),
        ] {
            let song: Song = parse_song("test.song", text).unwrap();
            assert_eq!(
                song.sections()[0].tempo().milli_quarters_per_minute(),
                expected_milli_quarters_per_minute,
                "{text}"
            );
        }
    }

    #[test]
    fn a_fractional_tempo_that_is_not_whole_milli_quarters_is_refused() {
        // 16th-note beats (x/16 meters): 0.001 beats/min × 0.25 q/beat is
        // 0.25 mq/min, not whole.
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/fractional_tempo_not_whole.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        // Reported at the section's line, where the meter and tempo meet.
        assert!(errors[0].starts_with("test.song line 15:"), "{}", errors[0]);
    }

    #[test]
    fn a_compound_meter_above_twelve_is_refused_at_its_line() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/meter_15_8.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 16:"), "{}", errors[0]);
        assert!(errors[0].contains("15/8 is not read yet"), "{}", errors[0]);
    }

    #[test]
    fn beat_grouping_is_kept_with_its_meter() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/grouping_kept.song"),
        )
        .unwrap();
        let groupings: Vec<Option<String>> = song
            .sections()
            .iter()
            .map(|section: &Section| section.beat_grouping().map(ToString::to_string))
            .collect();
        // B writes 7/8 and the same grouping; C writes its own; D has a new
        // meter and so no grouping.
        assert_eq!(
            groupings,
            vec![
                Some(String::from("2+2+3")),
                Some(String::from("2+2+3")),
                Some(String::from("3+2")),
                None
            ]
        );
    }

    #[test]
    fn a_grouping_that_does_not_fill_the_bar_is_refused_at_its_section() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/grouping_not_filling_the_bar.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 11:"), "{}", errors[0]);
        assert!(
            errors[0].contains("the grouping 2+2+2 adds up to 6 pulses, but a bar of 7/8 has 7"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_grouping_that_is_not_numbers_says_how_to_write_one() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/grouping_not_numbers.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 12:"), "{}", errors[0]);
        assert!(
            errors[0].contains("\"2+x\" is not a grouping"),
            "{}",
            errors[0]
        );
    }

    /// The chords or notes of the `track`'s line in each section, as text.
    fn contents_of_track(song: &Song, track: &str) -> Vec<Vec<String>> {
        song.sections()
            .iter()
            .map(|section: &Section| {
                section
                    .lines()
                    .iter()
                    .find(|line: &&Line| line.track_name() == track)
                    .unwrap()
                    .elements()
                    .iter()
                    .map(|line_element: &vocabulary::song::LineElement| {
                        match line_element.contents() {
                            Some(ElementContents::Chord(chord)) => chord.to_string(),
                            Some(ElementContents::Notes(notes)) => notes[0].to_string(),
                            other => format!("{other:?}"),
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn repeat_signs_are_written_out() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/repeat_signs.song"),
        )
        .unwrap();
        let bar_counts: Vec<u32> = song.sections().iter().map(Section::bar_count).collect();
        assert_eq!(bar_counts, vec![6, 3, 3]);
        let keys: Vec<Vec<String>> = contents_of_track(&song, "Keys");
        assert_eq!(keys[0], vec!["I", "ii7", "V", "ii7", "V", "IV"]);
        assert_eq!(keys[1], vec!["I", "I", "I"]);
        // A `/` at the start of a group plays what came before the sign.
        assert_eq!(keys[2], vec!["I", "I", "I"]);
        let bass: Vec<Vec<String>> = contents_of_track(&song, "Bass");
        assert_eq!(bass[0], vec!["C2", "G2", "D2", "C2", "G2", "D2"]);
        assert_eq!(bass[2], vec!["C2", "C2", "C2"]);
    }

    #[test]
    fn adjacent_repeat_signs_share_a_bar_line() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/repeat_signs_adjacent.song"),
        )
        .unwrap();
        let keys: Vec<Vec<String>> = contents_of_track(&song, "Keys");
        assert_eq!(keys[0], vec!["I", "V", "I", "V", "ii", "V", "ii", "V"]);
    }

    fn repeat_sign_error(text: &str) -> String {
        let errors: Vec<String> = errors_of(text);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 13:"), "{}", errors[0]);
        errors[0].clone()
    }

    #[test]
    fn a_repeat_sign_inside_another_is_refused() {
        let message: String = repeat_sign_error(include_str!(
            "../../../songs_canonical/song_file/repeat_sign_nested.song"
        ));
        assert!(message.contains("Repeat signs do not nest"), "{message}");
    }

    #[test]
    fn a_repeat_sign_not_closed_is_refused() {
        let message: String = repeat_sign_error(include_str!(
            "../../../songs_canonical/song_file/repeat_sign_not_closed.song"
        ));
        assert!(message.contains("opened and not closed"), "{message}");
    }

    #[test]
    fn a_repeat_sign_not_opened_is_refused() {
        let message: String = repeat_sign_error(include_str!(
            "../../../songs_canonical/song_file/repeat_sign_not_opened.song"
        ));
        assert!(
            message.contains("closes a group that was not opened"),
            "{message}"
        );
    }

    #[test]
    fn a_repeat_sign_with_zero_plays_is_refused() {
        let message: String = repeat_sign_error(include_str!(
            "../../../songs_canonical/song_file/repeat_sign_zero.song"
        ));
        assert!(message.contains("Use |%2 for twice"), "{message}");
    }

    #[test]
    fn a_repeat_sign_with_no_bars_is_refused() {
        let message: String = repeat_sign_error(include_str!(
            "../../../songs_canonical/song_file/repeat_sign_empty.song"
        ));
        assert!(message.contains("no bars between"), "{message}");
    }

    /// (start ticks, length ticks, sounding, accented) of each element of
    /// the `track`'s first line in the first section.
    fn places_of_track(
        song: &Song,
        track: &str,
        rhythm_line_only: bool,
    ) -> Vec<(u64, u64, bool, bool)> {
        let lines: Vec<&Line> = song.sections()[0]
            .lines()
            .iter()
            .filter(|line: &&Line| line.track_name() == track)
            .collect();
        let line: &Line = if rhythm_line_only { lines[1] } else { lines[0] };
        line.elements()
            .iter()
            .map(|line_element: &vocabulary::song::LineElement| {
                let element = line_element.element();
                (
                    element.start().ticks(),
                    element.length().ticks(),
                    element.kind() == vocabulary::rhythm_pattern::ElementKind::Sounding,
                    element.marks().accent() == vocabulary::rhythm_pattern::AccentLevel::Accent,
                )
            })
            .collect()
    }

    #[test]
    fn euclid_spreads_hits_over_the_bar() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/euclid.song"),
        )
        .unwrap();
        // 4/4 bar: 20160 ticks. euclid(3,8) is x..x..x. with steps of 2520 ticks;
        // euclid(2,5) is x.x.. with steps of 4032 ticks, in the second bar.
        let expected: Vec<(u64, u64, bool, bool)> = vec![
            (0, 2520, true, false),
            (2520, 5040, false, false),
            (7560, 2520, true, false),
            (10_080, 5040, false, false),
            (15_120, 2520, true, false),
            (17_640, 2520, false, false),
            (20_160, 4032, true, true),
            (24_192, 4032, false, false),
            (28_224, 4032, true, true),
            (32_256, 8064, false, false),
        ];
        let bass: Vec<(u64, u64, bool, bool)> = places_of_track(&song, "Bass", false);
        // The first bar's hits carry no accent; the second bar's carry it.
        assert_eq!(bass, expected);
        // A hit on a rhythm line with nothing before the colon takes its contents from the source.
        let keys: Vec<(u64, u64, bool, bool)> = places_of_track(&song, "Keys", true);
        assert_eq!(keys[..6], expected[..6]);
    }

    fn euclid_error(text: &str) -> String {
        let errors: Vec<String> = errors_of(text);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("test.song line 17:"), "{}", errors[0]);
        errors[0].clone()
    }

    #[test]
    fn euclid_with_no_hits_is_refused() {
        let message: String = euclid_error(include_str!(
            "../../../songs_canonical/song_file/euclid_no_hits.song"
        ));
        assert!(
            message.contains("euclid(0,8) needs at least 1 hit and no more hits than steps"),
            "{message}"
        );
    }

    #[test]
    fn euclid_with_more_hits_than_steps_is_refused() {
        let message: String = euclid_error(include_str!(
            "../../../songs_canonical/song_file/euclid_more_hits_than_steps.song"
        ));
        assert!(
            message.contains("euclid(9,8) needs at least 1 hit"),
            "{message}"
        );
    }

    #[test]
    fn euclid_beside_another_element_is_refused() {
        let message: String = euclid_error(include_str!(
            "../../../songs_canonical/song_file/euclid_not_alone.song"
        ));
        assert!(message.contains("euclid fills the whole bar"), "{message}");
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
        // The Keys line over two text lines with a comment between, the Bass
        // line written under its bare name.
        assert_eq!(
            bar_counts_of(include_str!(
                "../../../songs_canonical/song_file/line_continues_below.song"
            )),
            vec![4]
        );
    }

    #[test]
    fn a_mistake_in_a_continuation_names_its_own_text_line() {
        // Bar 3, on line 16, is short; the message counts bars along the line.
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/continuation_bar_short.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("test.song line 18: line \"Keys\": bar 3 is 4/4 and short"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn bars_not_under_a_line_are_refused() {
        // Before any line in the section (line 17).
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/bars_before_any_line.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("test.song line 17: bars with no line above them"),
            "{}",
            errors[0]
        );
        // After a setting (line 19), which ends the line above it.
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/bars_after_a_setting.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].starts_with("test.song line 19: bars with no line above them"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn joined_lines_of_different_lengths_are_reported() {
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/joined_lines_different_lengths.song"
        ));
        assert_eq!(errors.len(), 1, "{errors:?}");
    }

    #[test]
    fn voice_layers_are_lines_of_their_own_on_one_track() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/voice_layers.song"),
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
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/voice_layer_1_twice.song"
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
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/voice_layer_not_a_number.song"
        ));
        assert!(
            errors[0].contains("\"two\" is not a voice layer number"),
            "{}",
            errors[0]
        );
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/voice_layer_misspelt.song"
        ));
        assert!(
            errors[0].contains("after the track's name is not understood"),
            "{}",
            errors[0]
        );
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/voice_layer_on_bass.song"
        ));
        assert!(errors[0].contains("only voice layer 1"), "{}", errors[0]);
        let errors: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/voice_layer_zero.song"
        ));
        assert!(errors[0].contains("counted from 1"), "{}", errors[0]);
    }

    #[test]
    fn a_voice_layer_may_be_named_in_its_track() {
        let song: Song = parse_song(
            "test.song",
            include_str!("../../../songs_canonical/song_file/voice_layer_named.song"),
        )
        .unwrap();
        let keys: &Track = &song.tracks()[1];
        let two: VoiceLayerNumber = VoiceLayerNumber::new(2).unwrap();
        let three: VoiceLayerNumber = VoiceLayerNumber::new(3).unwrap();
        assert_eq!(keys.voice_layer_display_name(two), "Inner");
        assert_eq!(keys.voice_layer_display_name(three), "3");
        let unnamed: Vec<String> = errors_of(include_str!(
            "../../../songs_canonical/song_file/voice_layer_used_but_not_named.song"
        ));
        assert!(
            unnamed[0].contains("voice_layer 2 \"Inner\""),
            "{}",
            unnamed[0]
        );
    }
}
