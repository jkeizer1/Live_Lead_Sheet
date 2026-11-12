//! A song realised in a studio: the notes each voice plays and the cues each
//! control track sends, at their places in the whole song with every repeat
//! written out (EFS-1.6), ready for [`crate::schedule::schedule`].
//!
//! The song says what to play (chords, notes, accents, cues); the studio
//! configuration says where and how loud (port, channel, velocity,
//! octave offset). This module joins them.

use std::fmt;

use vocabulary::control_track::ControlTrack;
use vocabulary::key::Key;
use vocabulary::midi_values::NoteNumber;
use vocabulary::midi_values::Velocity;
use vocabulary::phrase_cue::{
    ChannelMessage, ControlCueMessage, OutputPortName, PhraseCue, PhraseCueError, TransportMessage,
};
use vocabulary::rhythm_pattern::ElementKind;
use vocabulary::score_length::ScoreLength;
use vocabulary::score_position::ScorePosition;
use vocabulary::song::{
    CueDefinition, CueDestination, CueMessage, ElementContents, Line, LineElement, LineRole,
    Section, Song, Track, TrackKind,
};
use vocabulary::tempo::Tempo;

use crate::note_track::{NoteTrack, NoteTrackError, TimedNote};
use crate::sample_source::held_stretches;
use crate::studio_configuration::{StudioConfiguration, TrackOutput, UnmetNeed};
use crate::voicing::{VoicingError, plain_voicing};

/// A staccato note sounds for half its written length (Song Notation,
/// section 6).
const STACCATO_DIVISOR: u64 = 2;

const SEMITONES_PER_OCTAVE: i16 = 12;

/// Everything the schedule and the sender need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongPlan {
    pub clock_port: OutputPortName,
    pub note_tracks: Vec<NoteTrack>,
    /// The cue control tracks, and the clock and transport track's Start at
    /// the first position and Stop at the end.
    pub control_tracks: Vec<ControlTrack>,
    pub stretch_length: ScoreLength,
    pub tempo: Tempo,
    /// Settings of the studio that the player does not apply yet, said
    /// aloud so nothing is silently ignored.
    pub not_yet_applied: Vec<String>,
}

/// One play of a section, with where it starts.
struct Pass<'a> {
    section: &'a Section,
    start_ticks: u64,
}

/// Plans `song` for `studio`.
///
/// # Errors
///
/// When the studio does not give the song what it needs; the sections have
/// different tempi (a tempo change is not played yet); a chord or note
/// lies outside the MIDI range; a tie runs into a different chord or note;
/// or a position cannot be counted.
pub fn plan_song(song: &Song, studio: &StudioConfiguration) -> Result<SongPlan, PlanError> {
    let unmet: Vec<UnmetNeed> = studio.unmet_needs(song);
    if !unmet.is_empty() {
        return Err(PlanError::StudioDoesNotCover { unmet });
    }
    let tempo: Tempo = common_tempo(song)?;
    let (passes, stretch_length): (Vec<Pass>, ScoreLength) = order_passes(song)?;

    let mut note_tracks: Vec<NoteTrack> = Vec::new();
    let mut control_tracks: Vec<ControlTrack> = Vec::new();
    let mut clock_port: Option<OutputPortName> = None;
    for track in song.tracks() {
        match track.kind() {
            TrackKind::Voice { .. } => {
                note_tracks.push(voice_track(track, studio, &passes)?);
            }
            TrackKind::CueControl { cues } => {
                control_tracks.push(cue_track(track, cues, song, studio, &passes)?);
            }
            TrackKind::ClockAndTransport => {
                let port: OutputPortName = port_of(studio, track)?;
                control_tracks.push(transport_track(track, &port, stretch_length)?);
                clock_port = Some(port);
            }
        }
    }
    let Some(clock_port) = clock_port else {
        // Song::new guarantees exactly one clock track.
        return Err(PlanError::NoClockTrack);
    };
    Ok(SongPlan {
        clock_port,
        note_tracks,
        control_tracks,
        stretch_length,
        tempo,
        not_yet_applied: not_yet_applied(song, studio),
    })
}

fn common_tempo(song: &Song) -> Result<Tempo, PlanError> {
    let mut sections = song.sections().iter();
    let Some(first) = sections.next() else {
        return Err(PlanError::NoSections);
    };
    for section in sections {
        if section.tempo() != first.tempo() {
            return Err(PlanError::TempoChange {
                section_name: section.name().to_string(),
            });
        }
    }
    Ok(first.tempo())
}

/// The sections in play order, each repeat written out, and the total length.
fn order_passes(song: &Song) -> Result<(Vec<Pass<'_>>, ScoreLength), PlanError> {
    let mut passes: Vec<Pass> = Vec::new();
    let mut position_ticks: u64 = 0;
    for section in song.sections() {
        // bars × ticks/bar → ticks.
        let section_ticks: u64 =
            u64::from(section.bar_count()) * section.time_signature().bar_length().ticks();
        for _ in 0..section.repeat_count() {
            passes.push(Pass {
                section,
                start_ticks: position_ticks,
            });
            position_ticks = position_ticks
                .checked_add(section_ticks)
                .ok_or(PlanError::TooLong)?;
        }
    }
    Ok((passes, ScoreLength::from_ticks(position_ticks)))
}

fn port_of(studio: &StudioConfiguration, track: &Track) -> Result<OutputPortName, PlanError> {
    let output: &TrackOutput = output_of(studio, track)?;
    OutputPortName::new(&output.port_name).map_err(PlanError::Cue)
}

fn output_of<'a>(
    studio: &'a StudioConfiguration,
    track: &Track,
) -> Result<&'a TrackOutput, PlanError> {
    studio
        .output_of(track.name())
        .ok_or_else(|| PlanError::StudioDoesNotCover {
            unmet: vec![UnmetNeed::NoTrackBlock {
                track_name: track.name().to_string(),
            }],
        })
}

/// Start at the first position and Stop at the end, on the clock port: a
/// device that runs its clock from transport (the FH-2) needs both.
fn transport_track(
    track: &Track,
    port: &OutputPortName,
    stretch_length: ScoreLength,
) -> Result<ControlTrack, PlanError> {
    let end: ScorePosition = ScorePosition::from_ticks(stretch_length.ticks());
    let mut cues: Vec<PhraseCue> = Vec::new();
    for (name, position, message) in [
        ("Start", ScorePosition::ZERO, TransportMessage::Start),
        ("Stop", end, TransportMessage::Stop),
    ] {
        cues.push(
            PhraseCue::new(
                name,
                position,
                port.clone(),
                ControlCueMessage::Transport(message),
            )
            .map_err(PlanError::Cue)?,
        );
    }
    ControlTrack::new(track.name(), cues).map_err(|e| PlanError::ControlTrack(e.to_string()))
}

/// What the studio asks for that the player does not do yet.
fn not_yet_applied(song: &Song, studio: &StudioConfiguration) -> Vec<String> {
    let mut notes: Vec<String> = Vec::new();
    for track in song.tracks() {
        let Some(output) = studio.output_of(track.name()) else {
            continue;
        };
        if output.settle_delay.ns() > 0 {
            notes.push(format!(
                "track \"{}\": settle_delay {} ms is not applied yet",
                track.name(),
                output.settle_delay.ns() / 1_000_000
            ));
        }
    }
    notes
}

fn voice_track(
    track: &Track,
    studio: &StudioConfiguration,
    passes: &[Pass],
) -> Result<NoteTrack, PlanError> {
    let TrackKind::Voice {
        clef, home_octave, ..
    } = track.kind()
    else {
        return Err(PlanError::NoClockTrack);
    };
    let output: &TrackOutput = output_of(studio, track)?;
    let Some(channel) = output.channel else {
        return Err(PlanError::StudioDoesNotCover {
            unmet: vec![UnmetNeed::NoChannel {
                track_name: track.name().to_string(),
            }],
        });
    };
    let home_octave: i8 = home_octave.unwrap_or_else(|| clef.default_home_octave());
    let mut notes: Vec<TimedNote> = Vec::new();
    for pass in passes {
        for line in pass.section.lines() {
            if line.track_name() != track.name() {
                continue;
            }
            // A sample source plays nothing itself: the rhythm line of its
            // voice layer takes what it holds.
            let LineRole::VoiceLayer(layer) = line.role() else {
                continue;
            };
            let source: Option<&Line> = pass.section.lines().iter().find(|other: &&Line| {
                other.track_name() == track.name() && other.role() == LineRole::SampleSource(layer)
            });
            notes.extend(line_notes(
                track,
                line,
                source,
                pass,
                output,
                home_octave,
                pass.section.key(),
            )?);
        }
    }
    NoteTrack::new(
        track.name(),
        OutputPortName::new(&output.port_name).map_err(PlanError::Cue)?,
        channel,
        notes,
    )
    .map(|note_track: NoteTrack| note_track.with_retrigger_gap(output.retrigger_gap))
    .map_err(PlanError::NoteTrack)
}

/// The notes of one line in one pass of a section: a note, or all of a
/// chord's notes, at each sounding element that no tie leads into. An
/// element with no contents of its own takes them from `source`, and follows
/// the source if it changes while the element sounds.
fn line_notes(
    track: &Track,
    line: &Line,
    source: Option<&Line>,
    pass: &Pass,
    output: &TrackOutput,
    home_octave: i8,
    key: Option<Key>,
) -> Result<Vec<TimedNote>, PlanError> {
    let elements: &[LineElement] = line.elements();
    let mut notes: Vec<TimedNote> = Vec::new();
    for (index, line_element) in elements.iter().enumerate() {
        if line_element.element().kind() == ElementKind::Rest {
            continue;
        }
        let continues_a_tie: bool = index > 0 && elements[index - 1].element().tied_to_next();
        if continues_a_tie {
            continue;
        }
        let written_ticks: u64 = tied_length(elements, index, track, pass)?;
        let element = line_element.element();
        let sounding_ticks: u64 = if element.marks().staccato() {
            (written_ticks / STACCATO_DIVISOR).max(1)
        } else {
            written_ticks
        };
        let velocity: Velocity = output.velocity.for_level(element.marks().accent());
        // Ticks from the start of the line, then from the start of the song.
        let line_start_ticks: u64 = element.start().ticks();
        let stretches: Vec<(u64, u64, &ElementContents)> =
            if let Some(contents) = line_element.contents() {
                vec![(line_start_ticks, sounding_ticks, contents)]
            } else {
                let source: &Line = source.ok_or_else(|| PlanError::NoSampleSource {
                    track_name: track.name().to_string(),
                    section_name: pass.section.name().to_string(),
                    start_ticks: line_start_ticks,
                })?;
                held_stretches(source, line_start_ticks, sounding_ticks)
                    .into_iter()
                    .map(|stretch| (stretch.start_ticks, stretch.length_ticks, stretch.contents))
                    .collect()
            };
        // Indexes in `notes` of the notes sounding in the stretch before.
        let mut sounding: Vec<usize> = Vec::new();
        for (stretch_start_ticks, stretch_length_ticks, contents) in stretches {
            let mut now_sounding: Vec<usize> = Vec::new();
            for note in note_numbers(contents, key, home_octave, output, track)? {
                // A pitch the stretch before also plays is held, not struck
                // again: on one channel a note off for it, right after its
                // new note on, would end the new note.
                let held: Option<usize> = sounding
                    .iter()
                    .copied()
                    .find(|&index: &usize| notes[index].note == note);
                if let Some(index) = held {
                    notes[index].length_ticks += stretch_length_ticks;
                    now_sounding.push(index);
                } else {
                    now_sounding.push(notes.len());
                    notes.push(TimedNote {
                        start_ticks: pass.start_ticks + stretch_start_ticks,
                        length_ticks: stretch_length_ticks,
                        note,
                        velocity,
                        legato_to_next: false,
                    });
                }
            }
            // What stops here is released after the new notes start: the
            // source followed the element with no new gate.
            for &index in &sounding {
                if !now_sounding.contains(&index) {
                    notes[index].legato_to_next = true;
                }
            }
            sounding = now_sounding;
        }
    }
    Ok(notes)
}

/// The written length of the element at `index` and those tied after it.
/// Every element in a tie must carry the same contents.
fn tied_length(
    elements: &[LineElement],
    index: usize,
    track: &Track,
    pass: &Pass,
) -> Result<u64, PlanError> {
    let mut written_ticks: u64 = elements[index].element().length().ticks();
    let mut last: usize = index;
    while elements[last].element().tied_to_next() {
        let Some(next) = elements.get(last + 1) else {
            break;
        };
        if next.contents() != elements[index].contents() {
            return Err(PlanError::TieIntoOtherContents {
                track_name: track.name().to_string(),
                section_name: pass.section.name().to_string(),
                start_ticks: next.element().start().ticks(),
            });
        }
        written_ticks += next.element().length().ticks();
        last += 1;
    }
    Ok(written_ticks)
}

/// The MIDI notes of a chord or of written notes, shifted by the device's
/// octave offset.
fn note_numbers(
    contents: &ElementContents,
    key: Option<Key>,
    home_octave: i8,
    output: &TrackOutput,
    track: &Track,
) -> Result<Vec<NoteNumber>, PlanError> {
    let unshifted: Vec<NoteNumber> = match contents {
        ElementContents::Chord(chord) => {
            let key: Key = key.ok_or_else(|| PlanError::NoKey {
                track_name: track.name().to_string(),
            })?;
            plain_voicing(*chord, key, home_octave).map_err(PlanError::Voicing)?
        }
        ElementContents::Notes(pitches)
        | ElementContents::ChordWithNotes { notes: pitches, .. } => pitches
            .iter()
            .map(|pitch| {
                pitch
                    .note_number()
                    .map_err(|e| PlanError::Pitch(e.to_string()))
            })
            .collect::<Result<Vec<NoteNumber>, PlanError>>()?,
        ElementContents::Cue(_) => Vec::new(),
    };
    let shift: i16 = SEMITONES_PER_OCTAVE * i16::from(output.octave_offset_octaves);
    unshifted
        .into_iter()
        .map(|note: NoteNumber| {
            let shifted: i16 = i16::from(note.value()) + shift;
            u8::try_from(shifted)
                .ok()
                .and_then(|byte: u8| NoteNumber::new(byte).ok())
                .ok_or_else(|| PlanError::OffsetOutOfRange {
                    track_name: track.name().to_string(),
                    note: note.value(),
                    shifted,
                })
        })
        .collect()
}

fn cue_track(
    track: &Track,
    definitions: &[CueDefinition],
    song: &Song,
    studio: &StudioConfiguration,
    passes: &[Pass],
) -> Result<ControlTrack, PlanError> {
    let mut cues: Vec<PhraseCue> = Vec::new();
    for pass in passes {
        for line in pass.section.lines() {
            if line.track_name() != track.name() {
                continue;
            }
            for line_element in line.elements() {
                let Some(ElementContents::Cue(name)) = line_element.contents() else {
                    continue;
                };
                let Some(definition) = definitions.iter().find(|d| d.name() == name) else {
                    continue; // Line::new already refused an undefined cue
                };
                let position: ScorePosition = ScorePosition::from_ticks(
                    pass.start_ticks + line_element.element().start().ticks(),
                );
                let length: ScoreLength = line_element.element().length();
                cues.push(
                    PhraseCue::new(
                        name,
                        position,
                        port_for(definition.destination(), song, studio)?,
                        cue_message(definition, song, studio, length)?,
                    )
                    .map_err(PlanError::Cue)?,
                );
            }
        }
    }
    ControlTrack::new(track.name(), cues).map_err(|e| PlanError::ControlTrack(e.to_string()))
}

/// The port and channel a cue destination resolves to.
fn destination_output(
    destination: &CueDestination,
    song: &Song,
    studio: &StudioConfiguration,
) -> Result<(String, Option<vocabulary::midi_values::MidiChannel>), PlanError> {
    match destination {
        CueDestination::Named(name) => studio
            .destination(name)
            .map(|d| (d.port_name.clone(), d.channel))
            .ok_or_else(|| PlanError::StudioDoesNotCover {
                unmet: vec![UnmetNeed::NoDestination {
                    destination_name: name.clone(),
                    cue_track_name: String::new(),
                }],
            }),
        CueDestination::VoiceTrack(name) => {
            let track: &Track = song
                .tracks()
                .iter()
                .find(|track| track.name() == name)
                .ok_or_else(|| PlanError::StudioDoesNotCover {
                    unmet: vec![UnmetNeed::CueToUnknownVoice {
                        track_name: name.clone(),
                        cue_track_name: String::new(),
                    }],
                })?;
            let output: &TrackOutput = output_of(studio, track)?;
            Ok((output.port_name.clone(), output.channel))
        }
    }
}

fn port_for(
    destination: &CueDestination,
    song: &Song,
    studio: &StudioConfiguration,
) -> Result<OutputPortName, PlanError> {
    let (port_name, _channel) = destination_output(destination, song, studio)?;
    OutputPortName::new(&port_name).map_err(PlanError::Cue)
}

fn cue_message(
    definition: &CueDefinition,
    song: &Song,
    studio: &StudioConfiguration,
    length: ScoreLength,
) -> Result<ControlCueMessage, PlanError> {
    let (_port, channel) = destination_output(definition.destination(), song, studio)?;
    let with_channel = |message: ChannelMessage| -> Result<ControlCueMessage, PlanError> {
        let channel = channel.ok_or_else(|| PlanError::CueNeedsChannel {
            cue_name: definition.name().to_string(),
        })?;
        Ok(ControlCueMessage::Channel { channel, message })
    };
    match definition.message() {
        CueMessage::Start => Ok(ControlCueMessage::Transport(TransportMessage::Start)),
        CueMessage::Stop => Ok(ControlCueMessage::Transport(TransportMessage::Stop)),
        CueMessage::Continue => Ok(ControlCueMessage::Transport(TransportMessage::Continue)),
        CueMessage::Note { note, velocity } => with_channel(ChannelMessage::Note {
            note,
            velocity,
            length,
        }),
        CueMessage::ControlChange { controller, value } => {
            with_channel(ChannelMessage::ControlChange { controller, value })
        }
        CueMessage::ProgramChange { program } => {
            with_channel(ChannelMessage::ProgramChange { program })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    StudioDoesNotCover {
        unmet: Vec<UnmetNeed>,
    },
    TempoChange {
        section_name: String,
    },
    TooLong,
    NoClockTrack,
    NoSections,
    NoKey {
        track_name: String,
    },
    TieIntoOtherContents {
        track_name: String,
        section_name: String,
        start_ticks: u64,
    },
    /// A rhythm element takes its contents from a sample source the section
    /// does not have. A song refuses this when it is made, so it is
    /// reported and not guessed.
    NoSampleSource {
        track_name: String,
        section_name: String,
        start_ticks: u64,
    },
    OffsetOutOfRange {
        track_name: String,
        note: u8,
        shifted: i16,
    },
    CueNeedsChannel {
        cue_name: String,
    },
    Voicing(VoicingError),
    Pitch(String),
    Cue(PhraseCueError),
    ControlTrack(String),
    NoteTrack(NoteTrackError),
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::StudioDoesNotCover { unmet } => {
                write!(f, "the studio does not cover the song:")?;
                for need in unmet {
                    write!(f, " {need}.")?;
                }
                Ok(())
            }
            PlanError::TempoChange { section_name } => write!(
                f,
                "section \"{section_name}\" has a different tempo from the first. Tempo changes \
                 between sections are not played yet; give every section the same tempo"
            ),
            PlanError::TooLong => write!(f, "the song is too long to count in ticks"),
            PlanError::NoClockTrack => write!(f, "the song has no clock and transport track"),
            PlanError::NoSections => write!(f, "the song has no sections"),
            PlanError::NoKey { track_name } => write!(
                f,
                "track \"{track_name}\" has a chord in a section with no key. Add a key"
            ),
            PlanError::TieIntoOtherContents {
                track_name,
                section_name,
                start_ticks,
            } => write!(
                f,
                "track \"{track_name}\", section \"{section_name}\": a tie runs into a note \
                 that plays something else (at {start_ticks} ticks into the line). Remove the \
                 ~ or give both notes the same contents"
            ),
            PlanError::NoSampleSource {
                track_name,
                section_name,
                start_ticks,
            } => write!(
                f,
                "track \"{track_name}\", section \"{section_name}\": an element with nothing \
                 before its colon (at {start_ticks} ticks into the line) has no sample_source \
                 line to take its contents from. Add one, or write what the element plays"
            ),
            PlanError::OffsetOutOfRange {
                track_name,
                note,
                shifted,
            } => write!(
                f,
                "track \"{track_name}\": MIDI note {note} shifted by the studio's octave_offset \
                 is {shifted}, outside 0 to 127. Change the octave_offset"
            ),
            PlanError::CueNeedsChannel { cue_name } => write!(
                f,
                "cue \"{cue_name}\" sends a channel message but its destination has no channel. \
                 Add a channel to the destination in the studio"
            ),
            PlanError::Voicing(error) => write!(f, "{error}"),
            PlanError::Pitch(message) | PlanError::ControlTrack(message) => {
                write!(f, "{message}")
            }
            PlanError::Cue(error) => write!(f, "{error}"),
            PlanError::NoteTrack(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PlanError {}
