//! A song realised in a studio: the notes each voice plays and the cues each
//! control track sends, at their places in the whole song with every repeat
//! written out (EFS-1.6), ready for [`crate::schedule::schedule`].
//!
//! The song says what to play (chords, notes, accents, cues); the studio
//! configuration says where and how loud (port, channel, velocity,
//! octave offset). This module joins them.

use std::fmt;

use vocabulary::bar_meters::BarMeters;
use vocabulary::control_track::ControlTrack;
use vocabulary::key::Key;
use vocabulary::midi_values::NoteNumber;
use vocabulary::midi_values::Velocity;
use vocabulary::numeral_standard::NumeralStandard;
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
use vocabulary::tempo_mark::TempoMarkKind;
use vocabulary::time_signature::TimeSignature;

use crate::dynamics_timeline::DynamicsTimeline;
use crate::layer_merge::{LayerNote, merge_equal_pitches_across_layers};
use crate::note_track::{NoteTrack, NoteTrackError, TimedNote};
use crate::sample_source::held_stretches;
use crate::schedule::TICKS_PER_CLOCK_PULSE;
use crate::studio_configuration::{StudioConfiguration, TrackOutput, UnmetNeed};
use crate::voicing::{VoicingError, inverted_voicing, plain_voicing};

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
    /// The tempo the song starts at.
    pub tempo: Tempo,
    /// Where the written tempo changes later, in position order. The player
    /// makes a new anchor at each (Musical Time and Clock, section 8): the
    /// instant of its position is computed from the anchor before it.
    pub tempo_changes: Vec<TempoChange>,
    /// Each play of each section in play order, so a display can announce
    /// the section, its key, meter and tempo, and its pass, as it starts.
    pub section_passes: Vec<SectionPass>,
    /// Settings of the studio that the player does not apply yet, said
    /// aloud so nothing is silently ignored.
    pub not_yet_applied: Vec<String>,
}

/// The written tempo changing at a score position: a section whose tempo
/// differs from the section played before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempoChange {
    pub position: ScorePosition,
    pub tempo: Tempo,
}

/// One play of a section, with where it starts.
struct Pass<'a> {
    section: &'a Section,
    /// Counted from 1.
    number: u32,
    start_ticks: u64,
}

/// What a display says when one play of a section starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionPass {
    pub section_name: String,
    pub count_in: bool,
    /// Counted from 1, out of `pass_count`.
    pub pass_number: u32,
    pub pass_count: u32,
    pub key: Option<Key>,
    /// The meter of the first bar.
    pub time_signature: TimeSignature,
    /// The meter of every bar, and the number of bars in one play.
    pub bar_meters: BarMeters,
    pub bar_count: u32,
    pub tempo: Tempo,
    /// Ticks from the start of the song.
    pub start_ticks: u64,
}

/// Plans `song` for `studio`.
///
/// # Errors
///
/// When the studio does not give the song what it needs; a chord or note
/// lies outside the MIDI range; a tie runs into a different chord or note;
/// or a position cannot be counted.
pub fn plan_song(song: &Song, studio: &StudioConfiguration) -> Result<SongPlan, PlanError> {
    let unmet: Vec<UnmetNeed> = studio.unmet_needs(song);
    if !unmet.is_empty() {
        return Err(PlanError::StudioDoesNotCover { unmet });
    }
    let (passes, stretch_length): (Vec<Pass>, ScoreLength) = order_passes(song)?;
    let (tempo, tempo_changes): (Tempo, Vec<TempoChange>) = tempo_map_of(&passes)?;

    let mut note_tracks: Vec<NoteTrack> = Vec::new();
    let mut control_tracks: Vec<ControlTrack> = Vec::new();
    let mut clock_port: Option<OutputPortName> = None;
    for track in song.tracks() {
        match track.kind() {
            TrackKind::Voice { .. } => {
                note_tracks.push(voice_track(
                    track,
                    studio,
                    song.numeral_standard(),
                    &passes,
                )?);
            }
            TrackKind::CueControl { cues } => {
                control_tracks.push(cue_track(track, cues, song, studio, &passes)?);
            }
            // The meter pattern is read from the section (Section::length_ticks);
            // the track sends nothing.
            TrackKind::MeterControl => {}
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
        tempo_changes,
        section_passes: passes.iter().map(section_pass_of).collect(),
        not_yet_applied: not_yet_applied(song, studio),
    })
}

fn section_pass_of(pass: &Pass) -> SectionPass {
    SectionPass {
        section_name: pass.section.name().to_string(),
        count_in: pass.section.count_in(),
        pass_number: pass.number,
        pass_count: pass.section.repeat_count(),
        key: pass.section.key(),
        time_signature: pass.section.time_signature(),
        bar_meters: pass.section.bar_meters().clone(),
        bar_count: pass.section.bar_count(),
        tempo: pass.section.tempo(),
        start_ticks: pass.start_ticks,
    }
}

/// The tempo the song starts at, and each later place where the tempo
/// changes: the start of a section play whose tempo differs from the tempo
/// in force, each tempo mark of the lead line, and, along a ramp, one place
/// at every clock pulse (Musical Time and Clock, section 8). A section play
/// starts at the section's own tempo, so a repeat after a ramp starts again
/// from the section's tempo.
fn tempo_map_of(passes: &[Pass]) -> Result<(Tempo, Vec<TempoChange>), PlanError> {
    if passes.is_empty() {
        return Err(PlanError::NoSections);
    }
    // Every place a tempo is set, in order; a later setting at the same
    // position (a mark at the section's first bar) replaces the earlier.
    let mut settings: Vec<(u64, Tempo)> = Vec::new();
    for pass in passes {
        let bar_meters: &BarMeters = pass.section.bar_meters();
        set_tempo(&mut settings, pass.start_ticks, pass.section.tempo());
        for mark in pass.section.tempo_marks() {
            let mark_start_ticks: u64 =
                pass.start_ticks + bar_meters.bar_start_ticks(mark.bar_index() as usize);
            match mark.kind() {
                TempoMarkKind::Step { tempo } => {
                    set_tempo(&mut settings, mark_start_ticks, tempo);
                }
                TempoMarkKind::Ramp { start, end, .. } => {
                    let ramp_end_ticks: u64 = pass.start_ticks
                        + bar_meters.bar_start_ticks(mark.end_bar_index() as usize);
                    let ramp_length_ticks: u64 = ramp_end_ticks - mark_start_ticks;
                    let pulse_count: u64 = ramp_length_ticks / TICKS_PER_CLOCK_PULSE;
                    for pulse in 0..=pulse_count {
                        let into_ramp_ticks: u64 = pulse * TICKS_PER_CLOCK_PULSE;
                        let tempo: Tempo =
                            ramp_tempo_at(start, end, into_ramp_ticks, ramp_length_ticks)?;
                        set_tempo(&mut settings, mark_start_ticks + into_ramp_ticks, tempo);
                    }
                    // The end tempo holds from the end of the ramp, which
                    // falls between clock pulses when the ramp is not a
                    // whole number of them.
                    set_tempo(&mut settings, ramp_end_ticks, end);
                }
            }
        }
    }
    let first_tempo: Tempo = settings[0].1;
    let mut changes: Vec<TempoChange> = Vec::new();
    let mut in_force: Tempo = first_tempo;
    for &(position_ticks, tempo) in &settings[1..] {
        if tempo != in_force {
            changes.push(TempoChange {
                position: ScorePosition::from_ticks(position_ticks),
                tempo,
            });
            in_force = tempo;
        }
    }
    Ok((first_tempo, changes))
}

/// Records `tempo` from `position_ticks`, replacing a setting already made
/// at that position.
fn set_tempo(settings: &mut Vec<(u64, Tempo)>, position_ticks: u64, tempo: Tempo) {
    if let Some(last) = settings.last_mut()
        && last.0 == position_ticks
    {
        last.1 = tempo;
        return;
    }
    settings.push((position_ticks, tempo));
}

/// The tempo `into_ramp_ticks` along a ramp of `ramp_length_ticks`, on the
/// straight line from `start` to `end`:
/// `B = B_start + (B_end - B_start) × into_ramp_ticks / ramp_length_ticks`,
/// in mq/min (the ticks cancel), rounded to the nearest whole mq/min.
fn ramp_tempo_at(
    start: Tempo,
    end: Tempo,
    into_ramp_ticks: u64,
    ramp_length_ticks: u64,
) -> Result<Tempo, PlanError> {
    let start_mq_per_min: i128 = i128::from(start.milli_quarters_per_minute());
    let end_mq_per_min: i128 = i128::from(end.milli_quarters_per_minute());
    let ramp_length: i128 = i128::from(ramp_length_ticks);
    let numerator: i128 = (end_mq_per_min - start_mq_per_min) * i128::from(into_ramp_ticks);
    // Round half away from zero: add half the divisor with the numerator's sign.
    let half: i128 = ramp_length / 2;
    let step: i128 = if numerator >= 0 {
        (numerator + half) / ramp_length
    } else {
        (numerator - half) / ramp_length
    };
    let mq_per_min: u64 = u64::try_from(start_mq_per_min + step).map_err(|_| PlanError::TooLong)?;
    Tempo::from_milli_quarters_per_minute(mq_per_min).map_err(|_| PlanError::TooLong)
}

/// The sections in play order, each repeat written out, and the total length.
fn order_passes(song: &Song) -> Result<(Vec<Pass<'_>>, ScoreLength), PlanError> {
    let mut passes: Vec<Pass> = Vec::new();
    let mut position_ticks: u64 = 0;
    for section in song.sections() {
        // The ticks of the section's bars, each in its own meter.
        let section_ticks: u64 = section.length_ticks();
        for number in 1..=section.repeat_count() {
            passes.push(Pass {
                section,
                number,
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
        // Only the notes of a voice track are delayed; the clock and the cue
        // tracks send as written, which is said here and not left silent.
        if output.delay_trigger.ns() > 0 && !matches!(track.kind(), TrackKind::Voice { .. }) {
            notes.push(format!(
                "track \"{}\": delay_trigger {} ms is applied only to a voice track",
                track.name(),
                output.delay_trigger.ns() / 1_000_000
            ));
        }
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
    numeral_standard: Option<NumeralStandard>,
    passes: &[Pass],
) -> Result<NoteTrack, PlanError> {
    let TrackKind::Voice { clef, .. } = track.kind() else {
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
    // The clef sets the octave; `+8va` and `-8va` move a passage.
    let home_octave: i8 = clef.default_home_octave();
    let mut layer_notes: Vec<LayerNote> = Vec::new();
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
            // The track's dynamics line in this section, if it has one.
            let dynamics: DynamicsTimeline = pass
                .section
                .lines()
                .iter()
                .find(|other: &&Line| {
                    other.track_name() == track.name() && other.role() == LineRole::Dynamics
                })
                .map_or_else(
                    || DynamicsTimeline::flat(output.velocity.normal()),
                    |dynamics_line: &Line| {
                        DynamicsTimeline::from_line(dynamics_line, output.velocity.normal())
                    },
                );
            let around: LineSurroundings = LineSurroundings {
                source,
                dynamics: &dynamics,
                home_octave,
                key: pass.section.key(),
                numeral_standard,
            };
            layer_notes.extend(
                line_notes(track, line, &around, pass, output)?
                    .into_iter()
                    .map(|note: TimedNote| LayerNote::new(layer.value(), note)),
            );
        }
    }
    // One note for the same pitch in two voice layers.
    let notes: Vec<TimedNote> = merge_equal_pitches_across_layers(layer_notes);
    NoteTrack::new(
        track.name(),
        OutputPortName::new(&output.port_name).map_err(PlanError::Cue)?,
        channel,
        notes,
    )
    .map(|note_track: NoteTrack| {
        note_track
            .with_retrigger_gap(output.retrigger_gap)
            .with_delay_trigger(output.delay_trigger)
    })
    .map_err(PlanError::NoteTrack)
}

/// What a line is planned against besides the track and the pass: the
/// `sample_source` of its voice layer, the section's dynamics for the track,
/// the clef's home octave and the section's key.
#[derive(Clone, Copy)]
struct LineSurroundings<'a> {
    source: Option<&'a Line>,
    dynamics: &'a DynamicsTimeline,
    home_octave: i8,
    key: Option<Key>,
    /// What the song declared for the roots of numerals in a minor key.
    numeral_standard: Option<NumeralStandard>,
}

/// The notes of one line in one pass of a section: a note, or all of a
/// chord's notes, at each sounding element that no tie leads into. An
/// element with no contents of its own takes them from `source`, and follows
/// the source if it changes while the element sounds.
fn line_notes(
    track: &Track,
    line: &Line,
    around: &LineSurroundings,
    pass: &Pass,
    output: &TrackOutput,
) -> Result<Vec<TimedNote>, PlanError> {
    let LineSurroundings {
        source,
        dynamics,
        home_octave,
        key,
        numeral_standard,
    } = *around;
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
        // Ticks from the start of the line, then from the start of the song.
        let line_start_ticks: u64 = element.start().ticks();
        // The loudness the dynamics line has reached where the element
        // starts; the element's own mark is measured from it.
        let velocity: Velocity = output.velocity.for_level_from(
            dynamics.velocity_at(line_start_ticks),
            element.marks().accent(),
        );
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
            for note in note_numbers(
                contents,
                key,
                numeral_standard,
                home_octave,
                line_element.octave_shift_octaves(),
                output,
                track,
            )? {
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
                        // The `_` mark; NoteTrack::new settles it.
                        legato_to_next: element.marks().legato(),
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

/// The MIDI notes of a chord or of written notes, shifted by the song's
/// octave annotation (`+8va`) and the device's octave offset.
fn note_numbers(
    contents: &ElementContents,
    key: Option<Key>,
    numeral_standard: Option<NumeralStandard>,
    home_octave: i8,
    annotation_octaves: i8,
    output: &TrackOutput,
    track: &Track,
) -> Result<Vec<NoteNumber>, PlanError> {
    let unshifted: Vec<NoteNumber> = match contents {
        ElementContents::Chord(chord) => {
            let key: Key = key.ok_or_else(|| PlanError::NoKey {
                track_name: track.name().to_string(),
            })?;
            plain_voicing(*chord, key, numeral_standard, home_octave).map_err(PlanError::Voicing)?
        }
        ElementContents::InvertedChord { chord, inversion } => {
            let key: Key = key.ok_or_else(|| PlanError::NoKey {
                track_name: track.name().to_string(),
            })?;
            inverted_voicing(*chord, key, numeral_standard, home_octave, *inversion)
                .map_err(PlanError::Voicing)?
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
        // A cue sounds no note; a dynamics line is not planned as notes, so
        // a level mark never gets here.
        ElementContents::Cue(_) | ElementContents::Dynamic(_) => Vec::new(),
    };
    let shift: i16 = SEMITONES_PER_OCTAVE
        * (i16::from(output.octave_offset_octaves) + i16::from(annotation_octaves));
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
                 and the song's octave annotation is {shifted}, outside 0 to 127. Change the \
                 octave_offset, or the +8va or -8va"
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
