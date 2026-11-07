//! The schedule: every MIDI event of a stretch of song, at its score
//! position and on its output port, in the order it must be sent.
//!
//! Made once when playing starts, from the song as written: Timing Clock
//! on the clock output (EFS-2.5), each note track's notes and each control
//! track's cues (D-10). The player then converts positions to instants from the
//! anchor and hands events to the ALSA output up to 50 ms ahead (D-8).
//!
//! This first version takes one stretch with a clock and note tracks that
//! are already settled; sections, repeats and the voices' chords come with
//! the song.

use std::collections::HashSet;
use std::fmt;

use vocabulary::control_track::ControlTrack;
use vocabulary::duration::Duration;
use vocabulary::midi_values::{
    ControllerNumber, ControllerValue, MidiChannel, NoteNumber, ProgramNumber, Velocity,
};
use vocabulary::phrase_cue::{
    ChannelMessage, ControlCueMessage, OutputPortName, PhraseCue, TransportMessage,
};
use vocabulary::score_length::ScoreLength;
use vocabulary::score_position::ScorePosition;

use crate::note_track::{NoteTrack, TimedNote};

/// 5040 ticks/q ÷ 24 clock pulses/q.
pub const TICKS_PER_CLOCK_PULSE: u64 = 210;

/// One MIDI message to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputAction {
    NoteOff {
        channel: MidiChannel,
        note: NoteNumber,
    },
    /// A note-off at a position where the same track starts a note. The
    /// output sends it earlier, so the receiver sees a note-off and then a
    /// note-on, not both at once (the ALSA output's lead floor; Song
    /// Notation, Q11). `note_on_position` is where the note started, to
    /// check the note is longer than the lead, and `retrigger_gap` is the
    /// studio's raise of that lead.
    TouchingNoteOff {
        channel: MidiChannel,
        note: NoteNumber,
        note_on_position: ScorePosition,
        retrigger_gap: Duration,
    },
    ControlChange {
        channel: MidiChannel,
        controller: ControllerNumber,
        value: ControllerValue,
    },
    ProgramChange {
        channel: MidiChannel,
        program: ProgramNumber,
    },
    Transport(TransportMessage),
    ClockPulse,
    NoteOn {
        channel: MidiChannel,
        note: NoteNumber,
        velocity: Velocity,
    },
}

impl OutputAction {
    /// The order of actions at the same position:
    /// - note offs first, so a note ending where the same note starts
    ///   again is retriggered, not cut;
    /// - control and program changes before the notes they set up;
    /// - Start, Stop and Continue before that position's Timing Clock,
    ///   which MIDI counts as the first clock pulse after Start;
    /// - note ons last.
    fn order_at_same_position(self) -> u8 {
        match self {
            OutputAction::NoteOff { .. } | OutputAction::TouchingNoteOff { .. } => 0,
            OutputAction::ControlChange { .. } => 1,
            OutputAction::ProgramChange { .. } => 2,
            OutputAction::Transport(_) => 3,
            OutputAction::ClockPulse => 4,
            OutputAction::NoteOn { .. } => 5,
        }
    }
}

/// Where an event came from, for the musician's display and messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventSource {
    TimingClock,
    Notes {
        track_name: String,
    },
    Cue {
        track_name: String,
        cue_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledEvent {
    pub position: ScorePosition,
    pub port: OutputPortName,
    pub action: OutputAction,
    pub source: EventSource,
}

/// Every event from 0 ticks up to `stretch_length`, in sending order:
/// by position, then by [`OutputAction`]'s order at the same position,
/// then in the order the tracks and cues are written.
///
/// # Errors
///
/// When a cue lies past the end of the stretch, or a position cannot be
/// counted.
pub fn schedule(
    clock_port: &OutputPortName,
    note_tracks: &[NoteTrack],
    control_tracks: &[ControlTrack],
    stretch_length: ScoreLength,
) -> Result<Vec<ScheduledEvent>, ScheduleError> {
    let mut events: Vec<ScheduledEvent> = timing_clock(clock_port, stretch_length)?;
    for track in note_tracks {
        events.extend(note_events(track, stretch_length)?);
    }
    for track in control_tracks {
        for cue in track.cues() {
            events.extend(cue_events(track.name(), cue, stretch_length)?);
        }
    }
    // A stable sort keeps the written order among equals.
    events.sort_by_key(|event: &ScheduledEvent| {
        (event.position, event.action.order_at_same_position())
    });
    Ok(events)
}

/// Timing Clock at every multiple of 210 ticks before the end (EFS-2.5).
fn timing_clock(
    clock_port: &OutputPortName,
    stretch_length: ScoreLength,
) -> Result<Vec<ScheduledEvent>, ScheduleError> {
    let mut events: Vec<ScheduledEvent> = Vec::new();
    let mut position_ticks: u64 = 0;
    while position_ticks < stretch_length.ticks() {
        events.push(ScheduledEvent {
            position: ScorePosition::from_ticks(position_ticks),
            port: clock_port.clone(),
            action: OutputAction::ClockPulse,
            source: EventSource::TimingClock,
        });
        position_ticks = position_ticks
            .checked_add(TICKS_PER_CLOCK_PULSE)
            .ok_or(ScheduleError::PositionTooLate { position_ticks })?;
    }
    Ok(events)
}

/// A note-on at each note's start and a note-off at its end. A note that
/// starts at or after the end of the stretch is left out, and one that
/// reaches past it is cut there. A note-off at a position where the track
/// starts a note is a touching note-off (legato, a connected gate, is not
/// built yet).
fn note_events(
    track: &NoteTrack,
    stretch_length: ScoreLength,
) -> Result<Vec<ScheduledEvent>, ScheduleError> {
    let source: EventSource = EventSource::Notes {
        track_name: track.track_name().to_string(),
    };
    let mut events: Vec<ScheduledEvent> = Vec::new();
    // Only notes that are played: one left out past the end touches nothing.
    let start_ticks_of_notes: HashSet<u64> = track
        .notes()
        .iter()
        .map(|timed: &TimedNote| timed.start_ticks)
        .filter(|start_ticks: &u64| *start_ticks < stretch_length.ticks())
        .collect();
    for timed in track.notes() {
        if timed.start_ticks >= stretch_length.ticks() {
            continue;
        }
        let end_ticks: u64 = timed
            .start_ticks
            .checked_add(timed.length_ticks)
            .ok_or(ScheduleError::PositionTooLate {
                position_ticks: timed.start_ticks,
            })?
            .min(stretch_length.ticks());
        events.push(ScheduledEvent {
            position: ScorePosition::from_ticks(timed.start_ticks),
            port: track.port().clone(),
            action: OutputAction::NoteOn {
                channel: track.channel(),
                note: timed.note,
                velocity: timed.velocity,
            },
            source: source.clone(),
        });
        events.push(ScheduledEvent {
            position: ScorePosition::from_ticks(end_ticks),
            port: track.port().clone(),
            action: if start_ticks_of_notes.contains(&end_ticks) {
                OutputAction::TouchingNoteOff {
                    channel: track.channel(),
                    note: timed.note,
                    note_on_position: ScorePosition::from_ticks(timed.start_ticks),
                    retrigger_gap: track.retrigger_gap(),
                }
            } else {
                OutputAction::NoteOff {
                    channel: track.channel(),
                    note: timed.note,
                }
            },
            source: source.clone(),
        });
    }
    Ok(events)
}

/// A cue's message at its position; a note also gets its note off.
fn cue_events(
    track_name: &str,
    cue: &PhraseCue,
    stretch_length: ScoreLength,
) -> Result<Vec<ScheduledEvent>, ScheduleError> {
    let past_end = || ScheduleError::CuePastEnd {
        track_name: track_name.to_string(),
        cue_name: cue.name().to_string(),
        cue_position_ticks: cue.position().ticks(),
        stretch_length_ticks: stretch_length.ticks(),
    };
    // A cue exactly at the end is allowed: e.g. Stop on the last barline.
    if cue.position().ticks() > stretch_length.ticks() {
        return Err(past_end());
    }
    let source: EventSource = EventSource::Cue {
        track_name: track_name.to_string(),
        cue_name: cue.name().to_string(),
    };
    let at = |action: OutputAction, position: ScorePosition| ScheduledEvent {
        position,
        port: cue.port().clone(),
        action,
        source: source.clone(),
    };
    let events: Vec<ScheduledEvent> = match cue.message() {
        ControlCueMessage::Transport(transport) => {
            vec![at(OutputAction::Transport(transport), cue.position())]
        }
        ControlCueMessage::Channel { channel, message } => match message {
            ChannelMessage::Note {
                note,
                velocity,
                length,
            } => {
                let note_off: ScorePosition = cue.position().checked_add(length).map_err(|_| {
                    ScheduleError::PositionTooLate {
                        position_ticks: cue.position().ticks(),
                    }
                })?;
                if note_off.ticks() > stretch_length.ticks() {
                    return Err(past_end());
                }
                vec![
                    at(
                        OutputAction::NoteOn {
                            channel,
                            note,
                            velocity,
                        },
                        cue.position(),
                    ),
                    at(OutputAction::NoteOff { channel, note }, note_off),
                ]
            }
            ChannelMessage::ControlChange { controller, value } => vec![at(
                OutputAction::ControlChange {
                    channel,
                    controller,
                    value,
                },
                cue.position(),
            )],
            ChannelMessage::ProgramChange { program } => vec![at(
                OutputAction::ProgramChange { channel, program },
                cue.position(),
            )],
        },
    };
    Ok(events)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    CuePastEnd {
        track_name: String,
        cue_name: String,
        cue_position_ticks: u64,
        stretch_length_ticks: u64,
    },
    PositionTooLate {
        position_ticks: u64,
    },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScheduleError::CuePastEnd {
                track_name,
                cue_name,
                cue_position_ticks,
                stretch_length_ticks,
            } => write!(
                f,
                "track {track_name:?}, cue {cue_name:?}: it reaches past the end of the song \
                 ({cue_position_ticks} ticks; the song ends at {stretch_length_ticks} ticks). \
                 Move it earlier"
            ),
            ScheduleError::PositionTooLate { position_ticks } => write!(
                f,
                "scheduling past score position {position_ticks} ticks: \
                 past the last position that can be counted"
            ),
        }
    }
}

impl std::error::Error for ScheduleError {}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::time_signature::TimeSignature;

    fn port(name: &str) -> OutputPortName {
        OutputPortName::new(name).unwrap()
    }

    fn four_four() -> TimeSignature {
        TimeSignature::new(4, 4).unwrap()
    }

    /// A note on each beat of a 4/4 bar, beat 1 at velocity 127.
    fn four_notes() -> NoteTrack {
        let notes: Vec<TimedNote> = (0..4)
            .map(|beat_index: u64| TimedNote {
                start_ticks: beat_index * four_four().beat_length().ticks(),
                length_ticks: 1260,
                note: NoteNumber::new(60).unwrap(),
                velocity: Velocity::new(if beat_index == 0 { 127 } else { 64 }).unwrap(),
            })
            .collect();
        NoteTrack::new(
            "Beat",
            port("Beat pulse"),
            MidiChannel::new(15).unwrap(),
            notes,
        )
        .unwrap()
    }

    #[test]
    fn one_bar_has_96_clock_pulses_and_4_notes() {
        let events: Vec<ScheduledEvent> = schedule(
            &port("Clock"),
            &[four_notes()],
            &[],
            ScoreLength::from_ticks(20_160),
        )
        .unwrap();
        let clock_pulses: usize = events
            .iter()
            .filter(|event: &&ScheduledEvent| event.action == OutputAction::ClockPulse)
            .count();
        // 20160 ticks ÷ 210 ticks/clock pulse = 96 clock pulses.
        assert_eq!(clock_pulses, 96);
        let velocities: Vec<u8> = events
            .iter()
            .filter_map(|event: &ScheduledEvent| match event.action {
                OutputAction::NoteOn { velocity, .. } => Some(velocity.value()),
                _ => None,
            })
            .collect();
        assert_eq!(velocities, vec![127, 64, 64, 64]);
    }

    #[test]
    fn start_comes_before_the_clock_pulse_at_the_same_position() {
        let start: PhraseCue = PhraseCue::new(
            "Start SQ-1",
            ScorePosition::from_ticks(20_160),
            port("SQ-1"),
            ControlCueMessage::Transport(TransportMessage::Start),
        )
        .unwrap();
        let cues: ControlTrack = ControlTrack::new("Studio", vec![start]).unwrap();
        let next_bar: NoteTrack = NoteTrack::new(
            "Beat",
            port("Beat pulse"),
            MidiChannel::new(15).unwrap(),
            vec![TimedNote {
                start_ticks: 20_160,
                length_ticks: 1260,
                note: NoteNumber::new(60).unwrap(),
                velocity: Velocity::new(127).unwrap(),
            }],
        )
        .unwrap();
        let events: Vec<ScheduledEvent> = schedule(
            &port("Clock"),
            &[next_bar],
            &[cues],
            ScoreLength::from_ticks(40_320),
        )
        .unwrap();
        let at_bar_2: Vec<OutputAction> = events
            .iter()
            .filter(|event: &&ScheduledEvent| event.position.ticks() == 20_160)
            .map(|event: &ScheduledEvent| event.action)
            .collect();
        assert_eq!(
            at_bar_2,
            vec![
                OutputAction::Transport(TransportMessage::Start),
                OutputAction::ClockPulse,
                OutputAction::NoteOn {
                    channel: MidiChannel::new(15).unwrap(),
                    note: NoteNumber::new(60).unwrap(),
                    velocity: Velocity::new(127).unwrap()
                },
            ]
        );
    }

    #[test]
    fn a_note_reaching_past_the_end_is_cut_and_one_after_it_is_left_out() {
        let note = |start_ticks: u64| TimedNote {
            start_ticks,
            length_ticks: 10_000,
            note: NoteNumber::new(60).unwrap(),
            velocity: Velocity::new(64).unwrap(),
        };
        let track: NoteTrack = NoteTrack::new(
            "Beat",
            port("Beat pulse"),
            MidiChannel::new(15).unwrap(),
            vec![note(15_000), note(20_160)],
        )
        .unwrap();
        let events: Vec<ScheduledEvent> = schedule(
            &port("Clock"),
            &[track],
            &[],
            ScoreLength::from_ticks(20_160),
        )
        .unwrap();
        let offs: Vec<u64> = events
            .iter()
            .filter(|event: &&ScheduledEvent| matches!(event.action, OutputAction::NoteOff { .. }))
            .map(|event: &ScheduledEvent| event.position.ticks())
            .collect();
        assert_eq!(offs, vec![20_160]);
    }

    #[test]
    fn a_note_off_comes_before_the_same_note_on_at_one_position() {
        let note = |start_ticks: u64| TimedNote {
            start_ticks,
            length_ticks: 5040,
            note: NoteNumber::new(60).unwrap(),
            velocity: Velocity::new(64).unwrap(),
        };
        let track: NoteTrack = NoteTrack::new(
            "Bass",
            port("Bass"),
            MidiChannel::new(2).unwrap(),
            vec![note(0), note(5040)],
        )
        .unwrap();
        let events: Vec<ScheduledEvent> = schedule(
            &port("Clock"),
            &[track],
            &[],
            ScoreLength::from_ticks(10_080),
        )
        .unwrap();
        let at_5040: Vec<&OutputAction> = events
            .iter()
            .filter(|event: &&ScheduledEvent| {
                event.position.ticks() == 5040 && event.action != OutputAction::ClockPulse
            })
            .map(|event: &ScheduledEvent| &event.action)
            .collect();
        assert!(matches!(at_5040[0], OutputAction::TouchingNoteOff { .. }));
        assert!(matches!(at_5040[1], OutputAction::NoteOn { .. }));
    }

    #[test]
    fn a_note_off_where_no_note_starts_is_a_plain_note_off() {
        let note = |start_ticks: u64| TimedNote {
            start_ticks,
            length_ticks: 2520,
            note: NoteNumber::new(60).unwrap(),
            velocity: Velocity::new(64).unwrap(),
        };
        let track: NoteTrack = NoteTrack::new(
            "Bass",
            port("Bass"),
            MidiChannel::new(2).unwrap(),
            vec![note(0), note(5040)],
        )
        .unwrap();
        let events: Vec<ScheduledEvent> = schedule(
            &port("Clock"),
            &[track],
            &[],
            ScoreLength::from_ticks(10_080),
        )
        .unwrap();
        let offs: Vec<(u64, bool)> = events
            .iter()
            .filter_map(|event: &ScheduledEvent| match event.action {
                OutputAction::NoteOff { .. } => Some((event.position.ticks(), false)),
                OutputAction::TouchingNoteOff { .. } => Some((event.position.ticks(), true)),
                _ => None,
            })
            .collect();
        assert_eq!(offs, vec![(2520, false), (7560, false)]);
    }

    #[test]
    fn a_touching_note_off_carries_the_note_start_and_the_studio_gap() {
        let note = |start_ticks: u64| TimedNote {
            start_ticks,
            length_ticks: 5040,
            note: NoteNumber::new(60).unwrap(),
            velocity: Velocity::new(64).unwrap(),
        };
        let track: NoteTrack = NoteTrack::new(
            "Bass",
            port("Bass"),
            MidiChannel::new(2).unwrap(),
            vec![note(0), note(5040)],
        )
        .unwrap()
        .with_retrigger_gap(Duration::from_ms(8));
        let events: Vec<ScheduledEvent> = schedule(
            &port("Clock"),
            &[track],
            &[],
            ScoreLength::from_ticks(10_080),
        )
        .unwrap();
        let touching: Vec<OutputAction> = events
            .iter()
            .map(|event: &ScheduledEvent| event.action)
            .filter(|action: &OutputAction| matches!(action, OutputAction::TouchingNoteOff { .. }))
            .collect();
        let expected: OutputAction = OutputAction::TouchingNoteOff {
            channel: MidiChannel::new(2).unwrap(),
            note: NoteNumber::new(60).unwrap(),
            note_on_position: ScorePosition::from_ticks(0),
            retrigger_gap: Duration::from_ms(8),
        };
        // The note ending at 10080 ticks, the end of the stretch, touches
        // no note and stays a plain note-off.
        assert_eq!(touching, vec![expected]);
    }

    #[test]
    fn cue_past_the_end_is_refused() {
        let late: PhraseCue = PhraseCue::new(
            "Too late",
            ScorePosition::from_ticks(40_321),
            port("SQ-1"),
            ControlCueMessage::Transport(TransportMessage::Stop),
        )
        .unwrap();
        let track: ControlTrack = ControlTrack::new("Studio", vec![late]).unwrap();
        assert!(
            schedule(
                &port("Clock"),
                &[],
                &[track],
                ScoreLength::from_ticks(40_320)
            )
            .is_err()
        );
    }
}
