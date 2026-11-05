//! The song as a composition (docs/Architecture/Song Notation.md, section
//! 5, and D-11): tracks with a type and a clef, sections, and for each
//! track in a section a line of elements carrying what they play. A song
//! names no port, channel or device; those belong to a studio
//! configuration.
//!
//! Every constructor refuses what the notation refuses, so a `Song` that
//! exists is a well-formed one.

use std::fmt;

use crate::key::Key;
use crate::midi_values::{ControllerNumber, ControllerValue, NoteNumber, ProgramNumber, Velocity};
use crate::pitch::Pitch;
use crate::rhythm_pattern::{
    ElementKind, RhythmElement, RhythmPatternError, check_elements_fill_bars,
};
use crate::roman_chord::RomanChord;
use crate::tempo::Tempo;
use crate::time_signature::TimeSignature;

/// A count-in section is one or two bars (EFS-2.7).
pub const MAX_COUNT_IN_BARS: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceKind {
    /// Several notes at once: chords and notes.
    Poly,
    /// One note at a time.
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clef {
    Treble,
    Bass,
    Alto,
    Tenor,
    Percussion,
    /// Treble and bass braced.
    Grand,
}

impl Clef {
    /// Where a plain voicing puts the root, and apply puts a bass line's
    /// roots, when the track gives no home octave (Song Notation, Q14).
    /// A percussion clef has no pitches; 4 is only a placeholder.
    #[must_use]
    pub const fn default_home_octave(self) -> i8 {
        match self {
            Clef::Treble | Clef::Grand | Clef::Percussion => 4,
            Clef::Alto | Clef::Tenor => 3,
            Clef::Bass => 2,
        }
    }
}

/// Where a cue goes, by name; the studio configuration maps the name to a
/// port and channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CueDestination {
    /// A destination named in the studio configuration, e.g. "SQ-1".
    Named(String),
    /// A voice track's own port and channel.
    VoiceTrack(String),
}

/// What a cue sends (EFS-4.3). A note's length is the written duration of
/// the element that places the cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueMessage {
    Note {
        note: NoteNumber,
        velocity: Velocity,
    },
    ControlChange {
        controller: ControllerNumber,
        value: ControllerValue,
    },
    ProgramChange {
        program: ProgramNumber,
    },
    Start,
    Stop,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueDefinition {
    name: String,
    destination: CueDestination,
    message: CueMessage,
}

impl CueDefinition {
    /// # Errors
    ///
    /// When the name is empty.
    pub fn new(
        name: &str,
        destination: CueDestination,
        message: CueMessage,
    ) -> Result<CueDefinition, SongError> {
        if name.trim().is_empty() {
            return Err(SongError::EmptyName { what: "cue" });
        }
        Ok(CueDefinition {
            name: name.to_string(),
            destination,
            message,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn destination(&self) -> &CueDestination {
        &self.destination
    }

    #[must_use]
    pub fn message(&self) -> CueMessage {
        self.message
    }
}

/// What a track is and can do (Song Notation, section 5). The later types
/// (tempo control, musician voice, automation control) are not here yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackKind {
    Voice {
        voice: VoiceKind,
        clef: Clef,
        /// Its chords drive the chord symbols and the key wheel (EFS-3.7).
        lead: bool,
        home_octave: Option<i8>,
    },
    /// Timing Clock and Stop; has a port and no channel.
    ClockAndTransport,
    CueControl {
        cues: Vec<CueDefinition>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    name: String,
    kind: TrackKind,
}

impl Track {
    /// # Errors
    ///
    /// When the name is empty.
    pub fn new(name: &str, kind: TrackKind) -> Result<Track, SongError> {
        if name.trim().is_empty() {
            return Err(SongError::EmptyName { what: "track" });
        }
        Ok(Track {
            name: name.to_string(),
            kind,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn kind(&self) -> &TrackKind {
        &self.kind
    }
}

/// What a sounding element carries (Song Notation, section 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementContents {
    /// A chord by name, voiced plainly.
    Chord(RomanChord),
    /// Notes, written out.
    Notes(Vec<Pitch>),
    /// A chord with its own voicing.
    ChordWithNotes {
        chord: RomanChord,
        notes: Vec<Pitch>,
    },
    /// The name of a cue defined on a cue control track.
    Cue(String),
}

/// A rhythm element with what it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineElement {
    element: RhythmElement,
    contents: Option<ElementContents>,
}

impl LineElement {
    /// # Errors
    ///
    /// When a rest carries contents.
    pub fn new(
        element: RhythmElement,
        contents: Option<ElementContents>,
    ) -> Result<LineElement, SongError> {
        if element.kind() == ElementKind::Rest && contents.is_some() {
            return Err(SongError::ContentsOnRest {
                start_ticks: element.start().ticks(),
            });
        }
        Ok(LineElement { element, contents })
    }

    #[must_use]
    pub fn element(&self) -> &RhythmElement {
        &self.element
    }

    #[must_use]
    pub fn contents(&self) -> Option<&ElementContents> {
        self.contents.as_ref()
    }
}

/// One track's rhythm and contents across the bars of a section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    track_name: String,
    bar_count: u32,
    elements: Vec<LineElement>,
}

impl Line {
    /// Makes the line of `track` for `bar_count` bars, refusing elements
    /// that do not fill whole bars and contents the track cannot play.
    ///
    /// # Errors
    ///
    /// When the elements do not tile the bars; when a track that takes no
    /// line is given one; or when a sounding element's contents do not fit
    /// the track (a chord on a mono voice, a cue not defined on the track,
    /// nothing on a voice or cue).
    pub fn new(
        track: &Track,
        time_signature: TimeSignature,
        bar_count: u32,
        elements: Vec<LineElement>,
    ) -> Result<Line, SongError> {
        let rhythm: Vec<RhythmElement> = elements
            .iter()
            .map(|line_element: &LineElement| *line_element.element())
            .collect();
        check_elements_fill_bars(
            &format!("line of track {:?}", track.name()),
            time_signature,
            bar_count,
            &rhythm,
        )
        .map_err(SongError::Rhythm)?;
        for line_element in &elements {
            if line_element.element().kind() == ElementKind::Sounding {
                check_contents(track, line_element)?;
            }
        }
        if matches!(track.kind(), TrackKind::ClockAndTransport) {
            return Err(SongError::LineOnClockTrack {
                track_name: track.name().to_string(),
            });
        }
        Ok(Line {
            track_name: track.name().to_string(),
            bar_count,
            elements,
        })
    }

    #[must_use]
    pub fn track_name(&self) -> &str {
        &self.track_name
    }

    #[must_use]
    pub fn bar_count(&self) -> u32 {
        self.bar_count
    }

    #[must_use]
    pub fn elements(&self) -> &[LineElement] {
        &self.elements
    }
}

/// One sounding element's contents against the kind of track.
fn check_contents(track: &Track, line_element: &LineElement) -> Result<(), SongError> {
    let start_ticks: u64 = line_element.element().start().ticks();
    let wrong = |why: &'static str| SongError::ContentsNotAllowed {
        track_name: track.name().to_string(),
        start_ticks,
        why,
    };
    match (track.kind(), line_element.contents()) {
        (TrackKind::Voice { .. } | TrackKind::CueControl { .. }, None) => {
            Err(SongError::ElementWithoutContents {
                track_name: track.name().to_string(),
                start_ticks,
            })
        }
        // The clock and transport track is refused for the whole line.
        (TrackKind::ClockAndTransport, _) => Ok(()),
        (TrackKind::Voice { voice, .. }, Some(contents)) => match (voice, contents) {
            (_, ElementContents::Cue(_)) => Err(wrong("a cue belongs on a cue control track")),
            (VoiceKind::Mono, ElementContents::Notes(notes)) if notes.len() == 1 => Ok(()),
            (VoiceKind::Mono, _) => Err(wrong(
                "a mono voice plays one note at a time; write a single note such as C2",
            )),
            (
                VoiceKind::Poly,
                ElementContents::Notes(notes) | ElementContents::ChordWithNotes { notes, .. },
            ) if notes.is_empty() => Err(wrong("write at least one note in the brackets")),
            (VoiceKind::Poly, _) => Ok(()),
        },
        (TrackKind::CueControl { cues }, Some(contents)) => match contents {
            ElementContents::Cue(name) => {
                if cues.iter().any(|cue: &CueDefinition| cue.name() == name) {
                    Ok(())
                } else {
                    Err(SongError::UnknownCue {
                        track_name: track.name().to_string(),
                        cue_name: name.clone(),
                    })
                }
            }
            _ => Err(wrong(
                "a cue control track carries cue names, e.g. \"Start SQ-1\"",
            )),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    name: String,
    count_in: bool,
    repeat_count: u32,
    /// The key in effect, written or carried on; `None` until a key is
    /// given (a count-in section needs none).
    key: Option<Key>,
    time_signature: TimeSignature,
    tempo: Tempo,
    lines: Vec<Line>,
}

impl Section {
    /// # Errors
    ///
    /// When the name is empty; the repeat count is 0; there are no lines;
    /// the lines differ in length (Song Notation, polymeter is deferred);
    /// two lines are for one track; a count-in section is longer than two
    /// bars; or a line carries a chord and no key is in effect.
    #[allow(clippy::too_many_arguments)] // one value per thing a section states
    pub fn new(
        name: &str,
        count_in: bool,
        repeat_count: u32,
        key: Option<Key>,
        time_signature: TimeSignature,
        tempo: Tempo,
        lines: Vec<Line>,
    ) -> Result<Section, SongError> {
        if name.trim().is_empty() {
            return Err(SongError::EmptyName { what: "section" });
        }
        if repeat_count == 0 {
            return Err(SongError::RepeatCountZero {
                section_name: name.to_string(),
            });
        }
        let Some(first) = lines.first() else {
            return Err(SongError::SectionWithoutLines {
                section_name: name.to_string(),
            });
        };
        let bar_count: u32 = first.bar_count();
        for (index, line) in lines.iter().enumerate() {
            if line.bar_count() != bar_count {
                return Err(SongError::LinesOfDifferentLength {
                    section_name: name.to_string(),
                    track_name: line.track_name().to_string(),
                    bar_count: line.bar_count(),
                    expected_bar_count: bar_count,
                });
            }
            if lines[..index]
                .iter()
                .any(|earlier: &Line| earlier.track_name() == line.track_name())
            {
                return Err(SongError::TwoLinesForOneTrack {
                    section_name: name.to_string(),
                    track_name: line.track_name().to_string(),
                });
            }
        }
        if count_in && bar_count > MAX_COUNT_IN_BARS {
            return Err(SongError::CountInTooLong {
                section_name: name.to_string(),
                bar_count,
            });
        }
        if key.is_none() && lines.iter().any(line_has_chord) {
            return Err(SongError::ChordsWithoutKey {
                section_name: name.to_string(),
            });
        }
        Ok(Section {
            name: name.to_string(),
            count_in,
            repeat_count,
            key,
            time_signature,
            tempo,
            lines,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn count_in(&self) -> bool {
        self.count_in
    }

    #[must_use]
    pub fn repeat_count(&self) -> u32 {
        self.repeat_count
    }

    #[must_use]
    pub fn key(&self) -> Option<Key> {
        self.key
    }

    #[must_use]
    pub fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    #[must_use]
    pub fn tempo(&self) -> Tempo {
        self.tempo
    }

    #[must_use]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The section's length in bars, once (not counting repeats).
    #[must_use]
    pub fn bar_count(&self) -> u32 {
        self.lines.first().map_or(0, Line::bar_count)
    }
}

fn line_has_chord(line: &Line) -> bool {
    line.elements().iter().any(|line_element: &LineElement| {
        matches!(
            line_element.contents(),
            Some(ElementContents::Chord(_) | ElementContents::ChordWithNotes { .. })
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    name: String,
    tracks: Vec<Track>,
    sections: Vec<Section>,
}

impl Song {
    /// # Errors
    ///
    /// When the name is empty; track names repeat; there is not exactly one
    /// clock and transport track; there are voice tracks and not exactly
    /// one is lead; there are no sections; a count-in section is not the
    /// first; a line is for a track the song lacks; or a section has no
    /// line for a track that is not allowed to rest.
    pub fn new(name: &str, tracks: Vec<Track>, sections: Vec<Section>) -> Result<Song, SongError> {
        if name.trim().is_empty() {
            return Err(SongError::EmptyName { what: "song" });
        }
        for (index, track) in tracks.iter().enumerate() {
            if tracks[..index]
                .iter()
                .any(|earlier: &Track| earlier.name() == track.name())
            {
                return Err(SongError::DuplicateTrackName {
                    track_name: track.name().to_string(),
                });
            }
        }
        let clock_tracks: usize = tracks
            .iter()
            .filter(|track: &&Track| matches!(track.kind(), TrackKind::ClockAndTransport))
            .count();
        if clock_tracks != 1 {
            return Err(SongError::ClockAndTransportTracks {
                found: clock_tracks,
            });
        }
        let voice_tracks: usize = tracks
            .iter()
            .filter(|track: &&Track| matches!(track.kind(), TrackKind::Voice { .. }))
            .count();
        let lead_tracks: usize = tracks
            .iter()
            .filter(|track: &&Track| matches!(track.kind(), TrackKind::Voice { lead: true, .. }))
            .count();
        if voice_tracks > 0 && lead_tracks != 1 {
            return Err(SongError::LeadTracks { found: lead_tracks });
        }
        if sections.is_empty() {
            return Err(SongError::NoSections);
        }
        for (index, section) in sections.iter().enumerate() {
            if section.count_in() && index > 0 {
                return Err(SongError::CountInNotFirst {
                    section_name: section.name().to_string(),
                });
            }
            for line in section.lines() {
                if !tracks
                    .iter()
                    .any(|track: &Track| track.name() == line.track_name())
                {
                    return Err(SongError::LineForUnknownTrack {
                        section_name: section.name().to_string(),
                        track_name: line.track_name().to_string(),
                    });
                }
            }
        }
        Ok(Song {
            name: name.to_string(),
            tracks,
            sections,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SongError {
    EmptyName {
        what: &'static str,
    },
    DuplicateTrackName {
        track_name: String,
    },
    ClockAndTransportTracks {
        found: usize,
    },
    LeadTracks {
        found: usize,
    },
    NoSections,
    CountInNotFirst {
        section_name: String,
    },
    CountInTooLong {
        section_name: String,
        bar_count: u32,
    },
    RepeatCountZero {
        section_name: String,
    },
    SectionWithoutLines {
        section_name: String,
    },
    LinesOfDifferentLength {
        section_name: String,
        track_name: String,
        bar_count: u32,
        expected_bar_count: u32,
    },
    TwoLinesForOneTrack {
        section_name: String,
        track_name: String,
    },
    LineForUnknownTrack {
        section_name: String,
        track_name: String,
    },
    LineOnClockTrack {
        track_name: String,
    },
    ChordsWithoutKey {
        section_name: String,
    },
    ContentsOnRest {
        start_ticks: u64,
    },
    ElementWithoutContents {
        track_name: String,
        start_ticks: u64,
    },
    ContentsNotAllowed {
        track_name: String,
        start_ticks: u64,
        why: &'static str,
    },
    UnknownCue {
        track_name: String,
        cue_name: String,
    },
    Rhythm(RhythmPatternError),
}

// One message per error variant; splitting them would only scatter the wording.
#[allow(clippy::too_many_lines)]
impl fmt::Display for SongError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SongError::EmptyName { what } => write!(f, "a {what} has no name. Give it one"),
            SongError::DuplicateTrackName { track_name } => write!(
                f,
                "two tracks are named {track_name:?}. Give each track its own name"
            ),
            SongError::ClockAndTransportTracks { found } => write!(
                f,
                "the song has {found} clock and transport tracks. It needs exactly 1: \
                 add a track with type clock_and_transport"
            ),
            SongError::LeadTracks { found } => write!(
                f,
                "{found} voice tracks are marked lead. Exactly 1 must be: write lead \
                 under the voice track whose chords drive the key wheel"
            ),
            SongError::NoSections => write!(f, "the song has no sections. Add a section"),
            SongError::CountInNotFirst { section_name } => write!(
                f,
                "section {section_name:?} is a count-in but is not the first section. \
                 Only the first section can be the count-in"
            ),
            SongError::CountInTooLong {
                section_name,
                bar_count,
            } => write!(
                f,
                "the count-in section {section_name:?} is {bar_count} bars. A count-in is \
                 1 or {MAX_COUNT_IN_BARS} bars"
            ),
            SongError::RepeatCountZero { section_name } => write!(
                f,
                "section {section_name:?} repeats 0 times. Use repeat 1 or more"
            ),
            SongError::SectionWithoutLines { section_name } => write!(
                f,
                "section {section_name:?} has no lines. Add a line for at least one track"
            ),
            SongError::LinesOfDifferentLength {
                section_name,
                track_name,
                bar_count,
                expected_bar_count,
            } => write!(
                f,
                "section {section_name:?}: the line for {track_name:?} has {bar_count} bars \
                 but the lines before it have {expected_bar_count}. All lines of a section \
                 must have the same number of bars"
            ),
            SongError::TwoLinesForOneTrack {
                section_name,
                track_name,
            } => write!(
                f,
                "section {section_name:?} has two lines for {track_name:?}. Keep one"
            ),
            SongError::LineForUnknownTrack {
                section_name,
                track_name,
            } => write!(
                f,
                "section {section_name:?} has a line for {track_name:?}, which is not a \
                 track of the song. Declare the track first, or fix the name"
            ),
            SongError::LineOnClockTrack { track_name } => write!(
                f,
                "{track_name:?} is the clock and transport track, which takes no line. \
                 Place Start and Stop with a cue control track"
            ),
            SongError::ChordsWithoutKey { section_name } => write!(
                f,
                "section {section_name:?} has chords but no key. Add key to the section, \
                 e.g. key C major"
            ),
            SongError::ContentsOnRest { start_ticks } => write!(
                f,
                "the rest at {start_ticks} ticks from the line's start carries something. \
                 Only a note can; take the part after the colon off"
            ),
            SongError::ElementWithoutContents {
                track_name,
                start_ticks,
            } => write!(
                f,
                "track {track_name:?}: the note at {start_ticks} ticks from the line's \
                 start carries nothing. Write a colon and what it plays, e.g. q:C2"
            ),
            SongError::ContentsNotAllowed {
                track_name,
                start_ticks,
                why,
            } => write!(
                f,
                "track {track_name:?}: the note at {start_ticks} ticks from the line's \
                 start cannot carry that: {why}"
            ),
            SongError::UnknownCue {
                track_name,
                cue_name,
            } => write!(
                f,
                "track {track_name:?} has no cue named {cue_name:?}. Define it under the \
                 track with: cue \"{cue_name}\" destination \"...\" start"
            ),
            SongError::Rhythm(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SongError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Accidental, NoteLetter, NoteName};
    use crate::rhythm_pattern::ElementMarks;
    use crate::roman_chord::{ChordKind, ScaleDegree};
    use crate::score_length::ScoreLength;

    fn four_four() -> TimeSignature {
        TimeSignature::new(4, 4).unwrap()
    }

    fn tempo() -> Tempo {
        Tempo::from_quarter_notes_per_minute(120).unwrap()
    }

    fn c_major() -> Key {
        Key::new(
            NoteName::new(NoteLetter::C, Accidental::Natural),
            crate::key::Mode::Major,
        )
    }

    fn quarter(index: u64, kind: ElementKind, contents: Option<ElementContents>) -> LineElement {
        LineElement::new(
            RhythmElement::new(
                ScoreLength::from_ticks(index * 5040),
                ScoreLength::from_ticks(5040),
                kind,
                ElementMarks::NONE,
                false,
            )
            .unwrap(),
            contents,
        )
        .unwrap()
    }

    fn track(name: &str, kind: TrackKind) -> Track {
        Track::new(name, kind).unwrap()
    }

    fn keys() -> Track {
        track(
            "Keys",
            TrackKind::Voice {
                voice: VoiceKind::Poly,
                clef: Clef::Grand,
                lead: true,
                home_octave: None,
            },
        )
    }

    fn bass() -> Track {
        track(
            "Bass",
            TrackKind::Voice {
                voice: VoiceKind::Mono,
                clef: Clef::Bass,
                lead: false,
                home_octave: None,
            },
        )
    }

    fn one_chord() -> ElementContents {
        ElementContents::Chord(RomanChord::new(
            Accidental::Natural,
            ScaleDegree::First,
            ChordKind::Major,
        ))
    }

    fn c2() -> ElementContents {
        ElementContents::Notes(vec![Pitch::new(
            NoteName::new(NoteLetter::C, Accidental::Natural),
            2,
        )])
    }

    fn four_quarters(contents: &ElementContents) -> Vec<LineElement> {
        (0..4)
            .map(|index| quarter(index, ElementKind::Sounding, Some(contents.clone())))
            .collect()
    }

    #[test]
    fn a_chord_on_a_poly_voice_and_a_note_on_a_mono_voice_are_accepted() {
        assert!(Line::new(&keys(), four_four(), 1, four_quarters(&one_chord())).is_ok());
        assert!(Line::new(&bass(), four_four(), 1, four_quarters(&c2())).is_ok());
    }

    #[test]
    fn a_chord_on_a_mono_voice_is_refused() {
        let error: SongError =
            Line::new(&bass(), four_four(), 1, four_quarters(&one_chord())).unwrap_err();
        assert!(matches!(error, SongError::ContentsNotAllowed { .. }));
    }

    #[test]
    fn a_sounding_element_on_a_voice_must_carry_something() {
        let elements: Vec<LineElement> = (0..4)
            .map(|index| quarter(index, ElementKind::Sounding, None))
            .collect();
        assert!(matches!(
            Line::new(&keys(), four_four(), 1, elements).unwrap_err(),
            SongError::ElementWithoutContents { .. }
        ));
    }

    #[test]
    fn a_rest_may_not_carry_contents() {
        let rest = LineElement::new(
            RhythmElement::new(
                ScoreLength::ZERO,
                ScoreLength::from_ticks(5040),
                ElementKind::Rest,
                ElementMarks::NONE,
                false,
            )
            .unwrap(),
            Some(one_chord()),
        );
        assert!(matches!(
            rest.unwrap_err(),
            SongError::ContentsOnRest { .. }
        ));
    }

    #[test]
    fn a_short_line_is_refused() {
        let elements: Vec<LineElement> = (0..3)
            .map(|index| quarter(index, ElementKind::Sounding, Some(one_chord())))
            .collect();
        assert!(matches!(
            Line::new(&keys(), four_four(), 1, elements).unwrap_err(),
            SongError::Rhythm(_)
        ));
    }

    #[test]
    fn lines_of_one_section_must_have_the_same_length() {
        let one_bar: Line =
            Line::new(&keys(), four_four(), 1, four_quarters(&one_chord())).unwrap();
        let two_bars: Line = Line::new(
            &bass(),
            four_four(),
            2,
            (0..8)
                .map(|index| {
                    let bar: u64 = index / 4;
                    let _ = bar;
                    quarter(index, ElementKind::Sounding, Some(c2()))
                })
                .collect(),
        )
        .unwrap();
        let error = Section::new(
            "Verse",
            false,
            1,
            Some(c_major()),
            four_four(),
            tempo(),
            vec![one_bar, two_bars],
        )
        .unwrap_err();
        assert!(matches!(error, SongError::LinesOfDifferentLength { .. }));
    }

    #[test]
    fn chords_need_a_key() {
        let line: Line = Line::new(&keys(), four_four(), 1, four_quarters(&one_chord())).unwrap();
        assert!(matches!(
            Section::new("Verse", false, 1, None, four_four(), tempo(), vec![line]).unwrap_err(),
            SongError::ChordsWithoutKey { .. }
        ));
    }

    #[test]
    fn a_song_needs_one_clock_track_and_one_lead() {
        let clock: Track = track("Clock", TrackKind::ClockAndTransport);
        let line: Line = Line::new(&keys(), four_four(), 1, four_quarters(&one_chord())).unwrap();
        let section = || {
            Section::new(
                "Verse",
                false,
                1,
                Some(c_major()),
                four_four(),
                tempo(),
                vec![line.clone()],
            )
            .unwrap()
        };
        assert!(Song::new("Demo", vec![clock.clone(), keys()], vec![section()]).is_ok());
        assert!(matches!(
            Song::new("Demo", vec![keys()], vec![section()]).unwrap_err(),
            SongError::ClockAndTransportTracks { found: 0 }
        ));
        assert!(matches!(
            Song::new(
                "Demo",
                vec![clock.clone(), clock.clone(), keys()],
                vec![section()]
            )
            .unwrap_err(),
            SongError::DuplicateTrackName { .. }
        ));
        let not_lead: Track = track(
            "Keys",
            TrackKind::Voice {
                voice: VoiceKind::Poly,
                clef: Clef::Treble,
                lead: false,
                home_octave: None,
            },
        );
        assert!(matches!(
            Song::new("Demo", vec![clock, not_lead], vec![section()]).unwrap_err(),
            SongError::LeadTracks { found: 0 }
        ));
    }

    #[test]
    fn a_cue_must_be_defined_on_its_track() {
        let cues: Track = track(
            "Cues",
            TrackKind::CueControl {
                cues: vec![
                    CueDefinition::new(
                        "Start SQ-1",
                        CueDestination::Named("SQ-1".to_string()),
                        CueMessage::Start,
                    )
                    .unwrap(),
                ],
            },
        );
        let place = |name: &str| {
            Line::new(
                &cues,
                four_four(),
                1,
                vec![
                    quarter(
                        0,
                        ElementKind::Sounding,
                        Some(ElementContents::Cue(name.to_string())),
                    ),
                    quarter(1, ElementKind::Rest, None),
                    quarter(2, ElementKind::Rest, None),
                    quarter(3, ElementKind::Rest, None),
                ],
            )
        };
        assert!(place("Start SQ-1").is_ok());
        assert!(matches!(
            place("Start SQ-2").unwrap_err(),
            SongError::UnknownCue { .. }
        ));
    }
}
