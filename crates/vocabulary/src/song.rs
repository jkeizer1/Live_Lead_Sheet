//! The song as a composition (docs/Architecture/Song Notation.md, section
//! 5, and D-11): tracks with a type and a clef, sections, and for each
//! track in a section a line of elements carrying what they play. A song
//! names no port, channel or device; those belong to a studio
//! configuration.
//!
//! Every constructor refuses what the notation refuses, so a `Song` that
//! exists is a well-formed one.

use std::fmt;

use crate::bar_meters::BarMeters;
use crate::beat_grouping::{BeatGrouping, BeatGroupingError};
use crate::dynamics::DynamicMark;
use crate::key::{Key, Mode};
use crate::midi_values::{ControllerNumber, ControllerValue, NoteNumber, ProgramNumber, Velocity};
use crate::numeral_standard::NumeralStandard;
use crate::pitch::Pitch;
use crate::rhythm_pattern::{
    ElementKind, RhythmElement, RhythmPatternError, check_elements_fill_bars,
};
use crate::roman_chord::{Inversion, RomanChord};
use crate::tempo::Tempo;
use crate::tempo_mark::{TempoMark, TempoMarkKind};
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

/// What a voice track's notes are (Song Notation, section 5, `notes`).
/// Every voice track says which; nothing can tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackNotes {
    /// Music: written as intervals from the section's tonic, so they follow
    /// every key override.
    Pitched,
    /// MIDI notes the receiver uses (a gate, a sample): written as note
    /// names and never transposed.
    MidiNote,
}

impl TrackNotes {
    /// As written in the files: `pitched`, `midi_note`.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            TrackNotes::Pitched => "pitched",
            TrackNotes::MidiNote => "midi_note",
        }
    }
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

/// Which voice layer of its track a line is: 1, 2, 3 and so on (Song
/// Notation, section 5). A voice track's layers are independent rhythmic
/// lines on one instrument: they share the track's port and channel and
/// are drawn on its staff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoiceLayerNumber(u8);

impl VoiceLayerNumber {
    /// The layer a line is when it names none.
    pub const FIRST: VoiceLayerNumber = VoiceLayerNumber(1);

    /// # Errors
    ///
    /// When the number is 0: layers are counted from 1.
    pub fn new(number: u8) -> Result<VoiceLayerNumber, SongError> {
        if number == 0 {
            return Err(SongError::VoiceLayerZero);
        }
        Ok(VoiceLayerNumber(number))
    }

    #[must_use]
    pub fn value(self) -> u8 {
        self.0
    }
}

/// A name given to one voice layer of a track, e.g. layer 2 "Inner". A
/// layer without a name is called by its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceLayerName {
    number: VoiceLayerNumber,
    name: String,
}

impl VoiceLayerName {
    /// # Errors
    ///
    /// When the name is empty.
    pub fn new(number: VoiceLayerNumber, name: &str) -> Result<VoiceLayerName, SongError> {
        if name.trim().is_empty() {
            return Err(SongError::EmptyName {
                what: "voice layer",
            });
        }
        Ok(VoiceLayerName {
            number,
            name: name.to_string(),
        })
    }

    #[must_use]
    pub fn number(&self) -> VoiceLayerNumber {
        self.number
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// What a line does for its track. A section holds at most one line per
/// track and role. More roles (the decorating streams of
/// `docs/Concept/concept_idea.md`) are to come.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineRole {
    /// One of a voice track's layers.
    VoiceLayer(VoiceLayerNumber),
    /// The line of a cue control track, placing its cues.
    CuePlacement,
    /// The chords or notes a voice layer's rhythm line takes what it plays
    /// from (Song Notation, section 5, "Sample sources and uncoupled
    /// lines"). Its elements carry contents and no lengths of their own: the
    /// elements of a bar share the bar evenly. It plays nothing itself.
    SampleSource(VoiceLayerNumber),
    /// The loudness of a voice track's elements, in rhythm notation (Song
    /// Notation, section 5, "Dynamics, in rhythm notation"). One per track
    /// in a section; it covers all the track's voice layers.
    Dynamics,
}

impl fmt::Display for LineRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineRole::VoiceLayer(number) => write!(f, "voice layer {}", number.value()),
            LineRole::CuePlacement => write!(f, "cue placement"),
            LineRole::Dynamics => write!(f, "dynamics"),
            LineRole::SampleSource(number) => {
                write!(f, "sample source of voice layer {}", number.value())
            }
        }
    }
}

/// Names given to voice layers: each number and each name at most once, and
/// a mono voice has only layer 1.
fn check_voice_layer_names(
    track_name: &str,
    voice: VoiceKind,
    names: &[VoiceLayerName],
) -> Result<(), SongError> {
    for (index, named) in names.iter().enumerate() {
        if voice == VoiceKind::Mono && named.number() != VoiceLayerNumber::FIRST {
            return Err(SongError::VoiceLayerOnMonoVoice {
                track_name: track_name.to_string(),
                number: named.number().value(),
            });
        }
        if names[..index].iter().any(|earlier: &VoiceLayerName| {
            earlier.number() == named.number() || earlier.name() == named.name()
        }) {
            return Err(SongError::VoiceLayerNamedTwice {
                track_name: track_name.to_string(),
                number: named.number().value(),
                name: named.name().to_string(),
            });
        }
    }
    Ok(())
}

/// What a track is and can do (Song Notation, section 5). The later types
/// (musician voice, automation control) are not here yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackKind {
    Voice {
        voice: VoiceKind,
        notes: TrackNotes,
        clef: Clef,
        /// The song's harmony track: the chord symbols show its chords.
        lead: bool,
        /// Names given to some of its voice layers; a layer without one is
        /// called by its number.
        voice_layer_names: Vec<VoiceLayerName>,
    },
    /// Timing Clock and Stop; has a port and no channel.
    ClockAndTransport,
    CueControl {
        cues: Vec<CueDefinition>,
    },
    /// The meter pattern of each section (Song Notation, "Meter Control");
    /// sends no MIDI.
    MeterControl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    name: String,
    kind: TrackKind,
}

impl Track {
    /// # Errors
    ///
    /// When the name is empty, or the voice layer names repeat a number or
    /// a name, or name a layer above 1 on a mono voice.
    pub fn new(name: &str, kind: TrackKind) -> Result<Track, SongError> {
        if name.trim().is_empty() {
            return Err(SongError::EmptyName { what: "track" });
        }
        if let TrackKind::Voice {
            voice,
            voice_layer_names,
            ..
        } = &kind
        {
            check_voice_layer_names(name, *voice, voice_layer_names)?;
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

    /// What a voice layer of this track is called: its given name, or
    /// else its number ("2"). Not a voice track: the number.
    #[must_use]
    pub fn voice_layer_display_name(&self, number: VoiceLayerNumber) -> String {
        let given: Option<&VoiceLayerName> = match &self.kind {
            TrackKind::Voice {
                voice_layer_names, ..
            } => voice_layer_names
                .iter()
                .find(|named: &&VoiceLayerName| named.number() == number),
            TrackKind::ClockAndTransport
            | TrackKind::CueControl { .. }
            | TrackKind::MeterControl => None,
        };
        given.map_or_else(
            || number.value().to_string(),
            |named| named.name().to_string(),
        )
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
    /// A chord by name, in an inversion (`ii7[1stInversion]`).
    InvertedChord {
        chord: RomanChord,
        inversion: Inversion,
    },
    /// A level, crescendo or diminuendo on a dynamics line.
    Dynamic(DynamicMark),
    /// The name of a cue defined on a cue control track.
    Cue(String),
}

/// A rhythm element with what it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineElement {
    element: RhythmElement,
    contents: Option<ElementContents>,
    /// The octave annotation in force at the element (`+8va` is 1, `-8va` is
    /// -1, `loco` and none are 0): every pitch it plays moves this many
    /// octaves (Song Notation, section 4).
    octave_shift_octaves: i8,
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
        Ok(LineElement {
            element,
            contents,
            octave_shift_octaves: 0,
        })
    }

    /// The element under an octave annotation.
    #[must_use]
    pub fn with_octave_shift_octaves(mut self, octave_shift_octaves: i8) -> LineElement {
        self.octave_shift_octaves = octave_shift_octaves;
        self
    }

    /// Octaves every pitch of the element moves, from `+8va`, `-8va` or `loco`.
    #[must_use]
    pub const fn octave_shift_octaves(&self) -> i8 {
        self.octave_shift_octaves
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

/// One line of a track across the bars of a section: its rhythm and what
/// each element plays, in the line's role for the track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    track_name: String,
    role: LineRole,
    bar_count: u32,
    elements: Vec<LineElement>,
    /// Tempo marks, in bar order; only the lead track's first voice layer
    /// has any.
    tempo_marks: Vec<TempoMark>,
    /// `tonic_octave n` on a pitched line: the octave of the section's own
    /// tonic its intervals are measured from, as written.
    tonic_octave: Option<i8>,
    /// The tonic the intervals were read from as this entry plays the
    /// section: the section's own, or under a key override the override's
    /// nearest it. Kept so an edit writes the same intervals back.
    tonic: Option<Pitch>,
}

impl Line {
    /// Makes the line of `track` in `role` for `bar_count` bars, refusing
    /// elements that do not fill whole bars and contents the track cannot
    /// play.
    ///
    /// # Errors
    ///
    /// When the elements do not tile the bars; when a track that takes no
    /// line is given one; when the role does not suit the track (a voice
    /// layer on a cue track, layer 2 on a mono voice); or when a sounding
    /// element's contents do not fit the track (a chord on a mono voice, a
    /// cue not defined on the track, nothing on a voice or cue).
    pub fn new(
        track: &Track,
        role: LineRole,
        bar_meters: impl Into<BarMeters>,
        bar_count: u32,
        elements: Vec<LineElement>,
    ) -> Result<Line, SongError> {
        let bar_meters: BarMeters = bar_meters.into();
        let rhythm: Vec<RhythmElement> = elements
            .iter()
            .map(|line_element: &LineElement| *line_element.element())
            .collect();
        check_elements_fill_bars(
            &format!("line of track {:?}", track.name()),
            &bar_meters,
            bar_count,
            &rhythm,
        )
        .map_err(SongError::Rhythm)?;
        check_role(track, role)?;
        for line_element in &elements {
            match (role, line_element.element().kind()) {
                (LineRole::SampleSource(_), ElementKind::Rest) => {
                    return Err(SongError::RestInSampleSource {
                        track_name: track.name().to_string(),
                        start_ticks: line_element.element().start().ticks(),
                    });
                }
                // A rhythm element with no contents takes them from the
                // sample source; the section checks that one exists.
                (LineRole::VoiceLayer(_), ElementKind::Sounding)
                    if line_element.contents().is_none()
                        && matches!(track.kind(), TrackKind::Voice { .. }) => {}
                (LineRole::Dynamics, ElementKind::Sounding) => {
                    if !matches!(line_element.contents(), Some(ElementContents::Dynamic(_))) {
                        return Err(SongError::DynamicsNeedsAMark {
                            track_name: track.name().to_string(),
                            start_ticks: line_element.element().start().ticks(),
                        });
                    }
                }
                (_, ElementKind::Sounding) => check_contents(track, line_element)?,
                (_, ElementKind::Rest) => {}
            }
        }
        if role == LineRole::Dynamics {
            check_ramps_have_targets(track, &elements)?;
        }
        Ok(Line {
            track_name: track.name().to_string(),
            role,
            bar_count,
            elements,
            tempo_marks: Vec::new(),
            tonic_octave: None,
            tonic: None,
        })
    }

    /// The line with its `tonic_octave` as written, and the tonic its
    /// intervals were read from as the entry plays it.
    #[must_use]
    pub fn with_tonic(self, tonic_octave: Option<i8>, tonic: Option<Pitch>) -> Line {
        Line {
            tonic_octave,
            tonic,
            ..self
        }
    }

    /// The tonic the line's intervals were read from, as the entry plays
    /// the section.
    #[must_use]
    pub fn tonic(&self) -> Option<Pitch> {
        self.tonic
    }

    /// `tonic_octave n`, on a pitched line that writes notes.
    #[must_use]
    pub fn tonic_octave(&self) -> Option<i8> {
        self.tonic_octave
    }

    /// The line with tempo marks, in bar order.
    ///
    /// # Errors
    ///
    /// When a mark is in a bar the line does not have, a ramp does not
    /// finish inside the line, or a mark is in a bar a ramp covers.
    pub fn with_tempo_marks(self, tempo_marks: Vec<TempoMark>) -> Result<Line, SongError> {
        for (index, mark) in tempo_marks.iter().enumerate() {
            if let TempoMarkKind::Ramp { bar_count, .. } = mark.kind()
                && mark.end_bar_index() > self.bar_count
            {
                return Err(SongError::TempoRampPastSectionEnd {
                    bar_number: mark.bar_index() + 1,
                    ramp_bar_count: bar_count,
                    bars_left: self.bar_count.saturating_sub(mark.bar_index()),
                });
            }
            if let Some(ramp) = tempo_marks[..index].iter().find(|earlier: &&TempoMark| {
                matches!(earlier.kind(), TempoMarkKind::Ramp { .. })
                    && mark.bar_index() < earlier.end_bar_index()
            }) {
                return Err(SongError::TempoMarkInsideRamp {
                    bar_number: mark.bar_index() + 1,
                    ramp_bar_number: ramp.bar_index() + 1,
                });
            }
        }
        Ok(Line {
            tempo_marks,
            ..self
        })
    }

    #[must_use]
    pub fn tempo_marks(&self) -> &[TempoMark] {
        &self.tempo_marks
    }

    #[must_use]
    pub fn track_name(&self) -> &str {
        &self.track_name
    }

    #[must_use]
    pub fn role(&self) -> LineRole {
        self.role
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

/// The role against the kind of track: voice layers on a voice track (only
/// layer 1 on a mono voice), cue placement on a cue control track, and no
/// line at all on the clock and transport track.
fn check_role(track: &Track, role: LineRole) -> Result<(), SongError> {
    match (track.kind(), role) {
        (TrackKind::ClockAndTransport, _) => Err(SongError::LineOnClockTrack {
            track_name: track.name().to_string(),
        }),
        (TrackKind::MeterControl, _) => Err(SongError::LineOnMeterControlTrack {
            track_name: track.name().to_string(),
        }),
        (
            TrackKind::Voice {
                voice: VoiceKind::Mono,
                ..
            },
            LineRole::VoiceLayer(number) | LineRole::SampleSource(number),
        ) if number != VoiceLayerNumber::FIRST => Err(SongError::VoiceLayerOnMonoVoice {
            track_name: track.name().to_string(),
            number: number.value(),
        }),
        (
            TrackKind::Voice { .. },
            LineRole::VoiceLayer(_) | LineRole::SampleSource(_) | LineRole::Dynamics,
        )
        | (TrackKind::CueControl { .. }, LineRole::CuePlacement) => Ok(()),
        (TrackKind::Voice { .. } | TrackKind::CueControl { .. }, _) => {
            Err(SongError::RoleNotForTrack {
                track_name: track.name().to_string(),
                role,
            })
        }
    }
}

/// A crescendo or diminuendo ends where the next level begins, which may be
/// bars later (`r` stretches in between do not change that). One with no
/// level written after it, or followed directly by another crescendo or
/// diminuendo, is refused (Song Notation, section 8, "Decided for the marks
/// and the dynamics line").
fn check_ramps_have_targets(track: &Track, elements: &[LineElement]) -> Result<(), SongError> {
    let mut open_ramp_ticks: Option<u64> = None;
    for line_element in elements {
        let Some(ElementContents::Dynamic(mark)) = line_element.contents() else {
            continue;
        };
        if let Some(start_ticks) = open_ramp_ticks
            && mark.is_ramp()
        {
            return Err(SongError::DynamicRampWithoutTarget {
                track_name: track.name().to_string(),
                start_ticks,
            });
        }
        open_ramp_ticks = if mark.is_ramp() {
            Some(line_element.element().start().ticks())
        } else {
            None
        };
    }
    match open_ramp_ticks {
        Some(start_ticks) => Err(SongError::DynamicRampWithoutTarget {
            track_name: track.name().to_string(),
            start_ticks,
        }),
        None => Ok(()),
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
        // The clock and transport and meter control tracks are refused for
        // the whole line.
        (TrackKind::ClockAndTransport | TrackKind::MeterControl, _) => Ok(()),
        (TrackKind::Voice { voice, notes, .. }, Some(contents)) => match (voice, contents) {
            (_, ElementContents::Cue(_)) => Err(wrong("a cue belongs on a cue control track")),
            (_, ElementContents::Dynamic(_)) => Err(wrong(
                "a level or crescendo belongs on a dynamics line, e.g. line \"Keys\" dynamics | p:w | f |",
            )),
            (VoiceKind::Mono, ElementContents::Notes(notes)) if notes.len() == 1 => Ok(()),
            (VoiceKind::Mono, _) => Err(wrong(
                "a mono voice plays one note at a time; write a single note such as C2",
            )),
            (
                _,
                ElementContents::Chord(_)
                | ElementContents::ChordWithNotes { .. }
                | ElementContents::InvertedChord { .. },
            ) if *notes == TrackNotes::MidiNote => Err(wrong(
                "its notes are midi_note, so a chord numeral means nothing to it; write the \
                 notes, e.g. C4",
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
    /// The key the section writes itself, which its intervals are measured
    /// from; `None` for a section that writes none.
    own_key: Option<Key>,
    /// The meter of every bar: the section's `meter`, or the repeating
    /// pattern of its `meter_control` line.
    bar_meters: BarMeters,
    /// How the pulses of an irregular bar are grouped (`grouping 2+2+3`);
    /// `None` when the song writes none. It changes nothing played.
    beat_grouping: Option<BeatGrouping>,
    tempo: Tempo,
    lines: Vec<Line>,
}

impl Section {
    /// # Errors
    ///
    /// When the name is empty; the repeat count is 0; there are no lines;
    /// the lines differ in length (Song Notation, polymeter is deferred);
    /// two lines are for one track in one role; a count-in section is longer than two
    /// bars; or a line carries a chord and no key is in effect.
    #[allow(clippy::too_many_arguments)] // one value per thing a section states
    pub fn new(
        name: &str,
        count_in: bool,
        repeat_count: u32,
        key: Option<Key>,
        bar_meters: impl Into<BarMeters>,
        tempo: Tempo,
        lines: Vec<Line>,
    ) -> Result<Section, SongError> {
        let bar_meters: BarMeters = bar_meters.into();
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
        if !(bar_count as usize).is_multiple_of(bar_meters.pattern_bar_count()) {
            return Err(SongError::BarCountNotMultipleOfPattern {
                section_name: name.to_string(),
                bar_count,
                pattern_bar_count: bar_meters.pattern_bar_count(),
            });
        }
        for (index, line) in lines.iter().enumerate() {
            if line.bar_count() != bar_count {
                return Err(SongError::LinesOfDifferentLength {
                    section_name: name.to_string(),
                    track_name: line.track_name().to_string(),
                    bar_count: line.bar_count(),
                    expected_bar_count: bar_count,
                });
            }
            if lines[..index].iter().any(|earlier: &Line| {
                earlier.track_name() == line.track_name() && earlier.role() == line.role()
            }) {
                return Err(SongError::TwoLinesForOneRole {
                    section_name: name.to_string(),
                    track_name: line.track_name().to_string(),
                    role: line.role(),
                });
            }
        }
        check_sample_sources(name, &lines)?;
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
            own_key: None,
            bar_meters,
            beat_grouping: None,
            tempo,
            lines,
        })
    }

    /// The section with a beat grouping for its meter.
    ///
    /// # Errors
    ///
    /// When the groups do not add up to the pulses in the section's bar.
    pub fn with_beat_grouping(self, group_pulses: Vec<u8>) -> Result<Section, SongError> {
        let beat_grouping: BeatGrouping = BeatGrouping::new(group_pulses, self.bar_meters.first())
            .map_err(|error: BeatGroupingError| SongError::BeatGrouping {
                section_name: self.name.clone(),
                error,
            })?;
        Ok(Section {
            beat_grouping: Some(beat_grouping),
            ..self
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

    /// The same section played `repeat_count` times: a section written again
    /// by name in a song (Song Notation, section 5, "Sections").
    ///
    /// # Errors
    ///
    /// When the repeat count is 0; a section that plays is played at least once.
    pub fn played_again(&self, repeat_count: u32) -> Result<Section, SongError> {
        if repeat_count == 0 {
            return Err(SongError::RepeatCountZero {
                section_name: self.name.clone(),
            });
        }
        Ok(Section {
            repeat_count,
            ..self.clone()
        })
    }

    #[must_use]
    pub fn key(&self) -> Option<Key> {
        self.key
    }

    /// The section with the key it writes itself.
    #[must_use]
    pub fn with_own_key(self, own_key: Option<Key>) -> Section {
        Section { own_key, ..self }
    }

    /// The key the section writes itself; `key` is the one it plays in.
    #[must_use]
    pub fn own_key(&self) -> Option<Key> {
        self.own_key
    }

    /// The meter of the first bar. Every meter of a section has the same
    /// beat, so this is the meter whose beat the section's tempo counts.
    #[must_use]
    pub fn time_signature(&self) -> TimeSignature {
        self.bar_meters.first()
    }

    /// The meter of each bar of the section.
    #[must_use]
    pub fn bar_meters(&self) -> &BarMeters {
        &self.bar_meters
    }

    /// The length of one play of the section, in ticks.
    #[must_use]
    pub fn length_ticks(&self) -> u64 {
        self.bar_meters.length_ticks(self.bar_count())
    }

    #[must_use]
    pub fn beat_grouping(&self) -> Option<&BeatGrouping> {
        self.beat_grouping.as_ref()
    }

    #[must_use]
    pub fn tempo(&self) -> Tempo {
        self.tempo
    }

    #[must_use]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The tempo marks of the section, in bar order: those of the lead
    /// track's first voice layer, which is the only line that has any.
    #[must_use]
    pub fn tempo_marks(&self) -> &[TempoMark] {
        self.lines
            .iter()
            .map(Line::tempo_marks)
            .find(|marks: &&[TempoMark]| !marks.is_empty())
            .unwrap_or(&[])
    }

    /// The tempo in force at the end of the section's last bar: what a
    /// following section that writes no tempo carries on.
    #[must_use]
    pub fn end_tempo(&self) -> Tempo {
        self.tempo_marks()
            .last()
            .map_or(self.tempo, TempoMark::end_tempo)
    }

    /// The section's length in bars, once (not counting repeats).
    #[must_use]
    pub fn bar_count(&self) -> u32 {
        self.lines.first().map_or(0, Line::bar_count)
    }
}

/// Each sample source needs the rhythm line of its voice layer, and each
/// rhythm element that writes no contents needs a sample source: a source
/// plays nothing alone, and a rhythm with nothing to take is a mistake the
/// song says so (Song Notation, section 5).
fn check_sample_sources(section_name: &str, lines: &[Line]) -> Result<(), SongError> {
    for line in lines {
        match line.role() {
            LineRole::SampleSource(number) => {
                let has_rhythm: bool = lines.iter().any(|other: &Line| {
                    other.track_name() == line.track_name()
                        && other.role() == LineRole::VoiceLayer(number)
                });
                if !has_rhythm {
                    return Err(SongError::SampleSourceWithoutRhythm {
                        section_name: section_name.to_string(),
                        track_name: line.track_name().to_string(),
                        number: number.value(),
                    });
                }
            }
            LineRole::VoiceLayer(number) => {
                let takes_from_source: bool = line.elements().iter().any(|line_element| {
                    line_element.element().kind() == ElementKind::Sounding
                        && line_element.contents().is_none()
                });
                let has_source: bool = lines.iter().any(|other: &Line| {
                    other.track_name() == line.track_name()
                        && other.role() == LineRole::SampleSource(number)
                });
                if takes_from_source && !has_source {
                    return Err(SongError::RhythmWithoutSampleSource {
                        section_name: section_name.to_string(),
                        track_name: line.track_name().to_string(),
                        number: number.value(),
                    });
                }
            }
            LineRole::CuePlacement | LineRole::Dynamics => {}
        }
    }
    Ok(())
}

fn line_has_chord(line: &Line) -> bool {
    line.elements().iter().any(|line_element: &LineElement| {
        matches!(
            line_element.contents(),
            Some(
                ElementContents::Chord(_)
                    | ElementContents::ChordWithNotes { .. }
                    | ElementContents::InvertedChord { .. }
            )
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    name: String,
    tracks: Vec<Track>,
    sections: Vec<Section>,
    /// What the song declared with `chord_numeral_standard`, if anything.
    numeral_standard: Option<NumeralStandard>,
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
        let meter_control_tracks: usize = tracks
            .iter()
            .filter(|track: &&Track| matches!(track.kind(), TrackKind::MeterControl))
            .count();
        if meter_control_tracks > 1 {
            return Err(SongError::MeterControlTracks {
                found: meter_control_tracks,
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
            numeral_standard: None,
        })
    }

    /// The song with its `chord_numeral_standard` (or none, when it wrote
    /// none).
    ///
    /// # Errors
    ///
    /// When there is none and a section in a minor key writes a chord
    /// numeral: its root would depend on a practice nobody declared.
    pub fn with_numeral_standard(
        mut self,
        numeral_standard: Option<NumeralStandard>,
    ) -> Result<Song, SongError> {
        if numeral_standard.is_none() {
            for section in &self.sections {
                let minor: bool = section
                    .key()
                    .is_some_and(|key: Key| key.mode() == Mode::Minor);
                let writes_numerals: bool = section.lines().iter().any(|line: &Line| {
                    line.elements().iter().any(|element: &LineElement| {
                        matches!(
                            element.contents(),
                            Some(
                                ElementContents::Chord(_)
                                    | ElementContents::InvertedChord { .. }
                                    | ElementContents::ChordWithNotes { .. }
                            )
                        )
                    })
                });
                if minor && writes_numerals {
                    return Err(SongError::MinorKeyNeedsNumeralStandard {
                        section_name: section.name().to_string(),
                    });
                }
            }
        }
        self.numeral_standard = numeral_standard;
        Ok(self)
    }

    /// What the song declared with `chord_numeral_standard`.
    #[must_use]
    pub fn numeral_standard(&self) -> Option<NumeralStandard> {
        self.numeral_standard
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
    MeterControlTracks {
        found: usize,
    },
    LineOnMeterControlTrack {
        track_name: String,
    },
    BarCountNotMultipleOfPattern {
        section_name: String,
        bar_count: u32,
        pattern_bar_count: usize,
    },
    TempoRampPastSectionEnd {
        bar_number: u32,
        ramp_bar_count: u32,
        bars_left: u32,
    },
    MinorKeyNeedsNumeralStandard {
        section_name: String,
    },
    TempoMarkInsideRamp {
        bar_number: u32,
        ramp_bar_number: u32,
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
    TwoLinesForOneRole {
        section_name: String,
        track_name: String,
        role: LineRole,
    },
    VoiceLayerZero,
    VoiceLayerOnMonoVoice {
        track_name: String,
        number: u8,
    },
    VoiceLayerNamedTwice {
        track_name: String,
        number: u8,
        name: String,
    },
    RoleNotForTrack {
        track_name: String,
        role: LineRole,
    },
    LineForUnknownTrack {
        section_name: String,
        track_name: String,
    },
    LineOnClockTrack {
        track_name: String,
    },
    BeatGrouping {
        section_name: String,
        error: BeatGroupingError,
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
    RestInSampleSource {
        track_name: String,
        start_ticks: u64,
    },
    DynamicsNeedsAMark {
        track_name: String,
        start_ticks: u64,
    },
    DynamicRampWithoutTarget {
        track_name: String,
        start_ticks: u64,
    },
    SampleSourceWithoutRhythm {
        section_name: String,
        track_name: String,
        number: u8,
    },
    RhythmWithoutSampleSource {
        section_name: String,
        track_name: String,
        number: u8,
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
            SongError::MeterControlTracks { found } => write!(
                f,
                "the song has {found} meter_control tracks. It needs at most 1"
            ),
            SongError::LineOnMeterControlTrack { track_name } => write!(
                f,
                "{track_name:?} is a meter_control track: its line holds a meter pattern, e.g. \
                 | mp [3_4 5_4] |"
            ),
            SongError::BarCountNotMultipleOfPattern {
                section_name,
                bar_count,
                pattern_bar_count,
            } => write!(
                f,
                "section {section_name:?} has {bar_count} bars, but its meter pattern is \
                 {pattern_bar_count} bars long. The section's length must be a multiple of it"
            ),
            SongError::TempoRampPastSectionEnd {
                bar_number,
                ramp_bar_count,
                bars_left,
            } => write!(
                f,
                "bar {bar_number}: the tempo ramp runs {ramp_bar_count} bars but the section has \
                 {bars_left} left from here. Shorten the ramp or move it earlier"
            ),
            SongError::MinorKeyNeedsNumeralStandard { section_name } => write!(
                f,
                "section \"{section_name}\" is in a minor key and writes chord numerals. Add \
                 chord_numeral_standard key_dependent or chord_numeral_standard \
                 major_referential on the line after song"
            ),
            SongError::TempoMarkInsideRamp {
                bar_number,
                ramp_bar_number,
            } => write!(
                f,
                "bar {bar_number}: a tempo mark in a bar the ramp from bar {ramp_bar_number} \
                 covers. Remove the mark, or shorten the ramp to end before this bar"
            ),
            SongError::LeadTracks { found } => write!(
                f,
                "{found} voice tracks are marked lead. Exactly 1 must be: write lead \
                 under the voice track that is the song's harmony track"
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
            SongError::TwoLinesForOneRole {
                section_name,
                track_name,
                role,
            } => match role {
                LineRole::VoiceLayer(number) => write!(
                    f,
                    "section {section_name:?} has two lines for {track_name:?} voice layer {}. \
                     Keep one, or give the other its own number: voice_layer {}",
                    number.value(),
                    number.value().saturating_add(1)
                ),
                LineRole::CuePlacement => write!(
                    f,
                    "section {section_name:?} has two lines for {track_name:?}. Keep one"
                ),
                LineRole::SampleSource(number) => write!(
                    f,
                    "section {section_name:?} has two sample_source lines for {track_name:?} \
                     voice layer {}. Keep one",
                    number.value()
                ),
                LineRole::Dynamics => write!(
                    f,
                    "section {section_name:?} has two dynamics lines for {track_name:?}. Keep \
                     one: it covers all the track's voice layers"
                ),
            },
            SongError::VoiceLayerZero => write!(
                f,
                "voice layers are counted from 1. Write voice_layer 1, 2, 3 ..."
            ),
            SongError::VoiceLayerOnMonoVoice { track_name, number } => write!(
                f,
                "{track_name:?} is a mono voice, so it has only voice layer 1, not {number}. \
                 Make it type poly_voice, or give the line its own track"
            ),
            SongError::VoiceLayerNamedTwice {
                track_name,
                number,
                name,
            } => write!(
                f,
                "track {track_name:?}: voice layer {number} \"{name}\" repeats a number or a \
                 name given above it. Give each layer its own number and name"
            ),
            SongError::RoleNotForTrack { track_name, role } => write!(
                f,
                "track {track_name:?} cannot have a {role} line. Voice layers are for voice \
                 tracks"
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
            SongError::BeatGrouping {
                section_name,
                error,
            } => write!(f, "section {section_name:?}: {error}"),
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
            SongError::RestInSampleSource { track_name, .. } => write!(
                f,
                "track {track_name:?}: a sample_source line holds chords or notes, not rests. \
                 Put the rests on the rhythm line below it, e.g. r:q"
            ),
            SongError::DynamicsNeedsAMark { track_name, .. } => write!(
                f,
                "track {track_name:?}: a dynamics line holds levels (pp, p, mp, mf, f, ff), \
                 < and >, and r for no change"
            ),
            SongError::DynamicRampWithoutTarget { track_name, .. } => write!(
                f,
                "track {track_name:?}: a crescendo or diminuendo needs the level it ends on, \
                 written after it. Write the target level, e.g. | < | f |"
            ),
            SongError::SampleSourceWithoutRhythm {
                section_name,
                track_name,
                number,
            } => write!(
                f,
                "section {section_name:?} has a sample_source line for {track_name:?} voice \
                 layer {number} and no rhythm line for it. Add line {track_name:?} with the \
                 lengths, e.g. | :q :q :q :q |"
            ),
            SongError::RhythmWithoutSampleSource {
                section_name,
                track_name,
                number,
            } => write!(
                f,
                "section {section_name:?}: {track_name:?} voice layer {number} has an element \
                 with nothing before its colon (e.g. :q), which takes what it plays from a \
                 sample_source line, and the section has none for it. Add line {track_name:?} \
                 sample_source | ... |, or write what the element plays, e.g. C4:q"
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

    fn layer_one() -> LineRole {
        LineRole::VoiceLayer(VoiceLayerNumber::FIRST)
    }

    fn track(name: &str, kind: TrackKind) -> Track {
        Track::new(name, kind).unwrap()
    }

    fn keys() -> Track {
        track(
            "Keys",
            TrackKind::Voice {
                voice: VoiceKind::Poly,
                notes: TrackNotes::Pitched,
                clef: Clef::Grand,
                lead: true,
                voice_layer_names: Vec::new(),
            },
        )
    }

    fn bass() -> Track {
        track(
            "Bass",
            TrackKind::Voice {
                voice: VoiceKind::Mono,
                notes: TrackNotes::Pitched,
                clef: Clef::Bass,
                lead: false,
                voice_layer_names: Vec::new(),
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
        assert!(
            Line::new(
                &keys(),
                layer_one(),
                four_four(),
                1,
                four_quarters(&one_chord())
            )
            .is_ok()
        );
        assert!(Line::new(&bass(), layer_one(), four_four(), 1, four_quarters(&c2())).is_ok());
    }

    #[test]
    fn a_chord_on_a_mono_voice_is_refused() {
        let error: SongError = Line::new(
            &bass(),
            layer_one(),
            four_four(),
            1,
            four_quarters(&one_chord()),
        )
        .unwrap_err();
        assert!(matches!(error, SongError::ContentsNotAllowed { .. }));
    }

    fn quarters_of(contents: Option<&ElementContents>, kind: ElementKind) -> Vec<LineElement> {
        (0..4)
            .map(|index| quarter(index, kind, contents.cloned()))
            .collect()
    }

    /// A voice element with no contents takes them from a sample source, so
    /// the line accepts it and the section asks for the source.
    #[test]
    fn a_voice_element_with_no_contents_needs_a_sample_source_in_its_section() {
        let rhythm: Line = Line::new(
            &keys(),
            layer_one(),
            four_four(),
            1,
            quarters_of(None, ElementKind::Sounding),
        )
        .unwrap();
        assert!(matches!(
            section_of(vec![rhythm.clone()]).unwrap_err(),
            SongError::RhythmWithoutSampleSource { .. }
        ));
        let source: Line = Line::new(
            &keys(),
            LineRole::SampleSource(VoiceLayerNumber::FIRST),
            four_four(),
            1,
            quarters_of(Some(&one_chord()), ElementKind::Sounding),
        )
        .unwrap();
        assert!(section_of(vec![rhythm, source]).is_ok());
    }

    #[test]
    fn a_sample_source_needs_its_rhythm_line_and_holds_no_rests() {
        let source: Line = Line::new(
            &keys(),
            LineRole::SampleSource(VoiceLayerNumber::FIRST),
            four_four(),
            1,
            quarters_of(Some(&one_chord()), ElementKind::Sounding),
        )
        .unwrap();
        assert!(matches!(
            section_of(vec![source]).unwrap_err(),
            SongError::SampleSourceWithoutRhythm { .. }
        ));
        assert!(matches!(
            Line::new(
                &keys(),
                LineRole::SampleSource(VoiceLayerNumber::FIRST),
                four_four(),
                1,
                quarters_of(None, ElementKind::Rest),
            )
            .unwrap_err(),
            SongError::RestInSampleSource { .. }
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
            Line::new(&keys(), layer_one(), four_four(), 1, elements).unwrap_err(),
            SongError::Rhythm(_)
        ));
    }

    #[test]
    fn lines_of_one_section_must_have_the_same_length() {
        let one_bar: Line = Line::new(
            &keys(),
            layer_one(),
            four_four(),
            1,
            four_quarters(&one_chord()),
        )
        .unwrap();
        let two_bars: Line = Line::new(
            &bass(),
            layer_one(),
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
        let line: Line = Line::new(
            &keys(),
            layer_one(),
            four_four(),
            1,
            four_quarters(&one_chord()),
        )
        .unwrap();
        assert!(matches!(
            Section::new("Verse", false, 1, None, four_four(), tempo(), vec![line]).unwrap_err(),
            SongError::ChordsWithoutKey { .. }
        ));
    }

    #[test]
    fn a_song_needs_one_clock_track_and_one_lead() {
        let clock: Track = track("Clock", TrackKind::ClockAndTransport);
        let line: Line = Line::new(
            &keys(),
            layer_one(),
            four_four(),
            1,
            four_quarters(&one_chord()),
        )
        .unwrap();
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
                notes: TrackNotes::Pitched,
                clef: Clef::Treble,
                lead: false,
                voice_layer_names: Vec::new(),
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
                LineRole::CuePlacement,
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

    fn layer(number: u8) -> LineRole {
        LineRole::VoiceLayer(VoiceLayerNumber::new(number).unwrap())
    }

    fn section_of(lines: Vec<Line>) -> Result<Section, SongError> {
        Section::new(
            "Verse",
            false,
            1,
            Some(c_major()),
            four_four(),
            tempo(),
            lines,
        )
    }

    #[test]
    fn a_poly_voice_may_have_several_voice_layers_in_a_section() {
        let first: Line = Line::new(
            &keys(),
            layer(1),
            four_four(),
            1,
            four_quarters(&one_chord()),
        )
        .unwrap();
        let second: Line =
            Line::new(&keys(), layer(2), four_four(), 1, four_quarters(&c2())).unwrap();
        let section: Section = section_of(vec![first, second]).unwrap();
        assert_eq!(section.lines().len(), 2);
        assert_eq!(section.lines()[1].role(), layer(2));
    }

    #[test]
    fn two_lines_for_one_voice_layer_are_refused() {
        let first: Line = Line::new(
            &keys(),
            layer(2),
            four_four(),
            1,
            four_quarters(&one_chord()),
        )
        .unwrap();
        let again: Line = Line::new(
            &keys(),
            layer(2),
            four_four(),
            1,
            four_quarters(&one_chord()),
        )
        .unwrap();
        let error: SongError = section_of(vec![first, again]).unwrap_err();
        assert!(matches!(error, SongError::TwoLinesForOneRole { .. }));
        assert!(error.to_string().contains("voice layer 2"));
    }

    #[test]
    fn a_mono_voice_has_only_voice_layer_one() {
        let error: SongError =
            Line::new(&bass(), layer(2), four_four(), 1, four_quarters(&c2())).unwrap_err();
        assert!(matches!(
            error,
            SongError::VoiceLayerOnMonoVoice { number: 2, .. }
        ));
    }

    #[test]
    fn roles_must_suit_the_track() {
        let error: SongError = Line::new(
            &keys(),
            LineRole::CuePlacement,
            four_four(),
            1,
            four_quarters(&one_chord()),
        )
        .unwrap_err();
        assert!(matches!(error, SongError::RoleNotForTrack { .. }));
    }

    #[test]
    fn voice_layers_count_from_one() {
        assert!(matches!(
            VoiceLayerNumber::new(0).unwrap_err(),
            SongError::VoiceLayerZero
        ));
    }

    fn manual(names: Vec<VoiceLayerName>, voice: VoiceKind) -> Result<Track, SongError> {
        Track::new(
            "Manual",
            TrackKind::Voice {
                voice,
                notes: TrackNotes::Pitched,
                clef: Clef::Grand,
                lead: true,
                voice_layer_names: names,
            },
        )
    }

    fn named(number: u8, name: &str) -> VoiceLayerName {
        VoiceLayerName::new(VoiceLayerNumber::new(number).unwrap(), name).unwrap()
    }

    #[test]
    fn a_voice_layer_is_called_by_its_name_or_else_its_number() {
        let track: Track = manual(vec![named(2, "Inner")], VoiceKind::Poly).unwrap();
        let two: VoiceLayerNumber = VoiceLayerNumber::new(2).unwrap();
        let three: VoiceLayerNumber = VoiceLayerNumber::new(3).unwrap();
        assert_eq!(track.voice_layer_display_name(two), "Inner");
        assert_eq!(track.voice_layer_display_name(three), "3");
    }

    #[test]
    fn voice_layer_names_must_not_repeat() {
        assert!(matches!(
            manual(vec![named(2, "Inner"), named(2, "Alto")], VoiceKind::Poly).unwrap_err(),
            SongError::VoiceLayerNamedTwice { .. }
        ));
        assert!(matches!(
            manual(vec![named(2, "Inner"), named(3, "Inner")], VoiceKind::Poly).unwrap_err(),
            SongError::VoiceLayerNamedTwice { .. }
        ));
        assert!(matches!(
            manual(vec![named(2, "Inner")], VoiceKind::Mono).unwrap_err(),
            SongError::VoiceLayerOnMonoVoice { .. }
        ));
    }
}
