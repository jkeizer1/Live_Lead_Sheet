//! The schedule: every MIDI event of a stretch of song, at its score
//! position and on its output port, in the order it must be sent.
//!
//! Made once when playing starts, from the song as written: Timing Clock
//! on the clock output (EFS-2.5) and each control track's beat pulse and
//! cues (D-10). The player then converts positions to instants from the
//! anchor and hands events to the ALSA output up to 50 ms ahead (D-8).
//!
//! This first version takes one time signature for the whole stretch;
//! sections, repeats and voice tracks come with the song.

use std::fmt;

use vocabulary::control_track::{BeatPulse, ControlTrack};
use vocabulary::midi_values::{
    ControllerNumber, ControllerValue, MidiChannel, NoteNumber, ProgramNumber, Velocity,
};
use vocabulary::phrase_cue::{
    ChannelMessage, ControlCueMessage, OutputPortName, PhraseCue, TransportMessage,
};
use vocabulary::score_length::ScoreLength;
use vocabulary::score_position::ScorePosition;
use vocabulary::time_signature::TimeSignature;

/// 5040 ticks/q ÷ 24 pulses/q.
pub const TICKS_PER_CLOCK_PULSE: u64 = 210;

/// One MIDI message to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputAction {
    NoteOff {
        channel: MidiChannel,
        note: NoteNumber,
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
    ///   which MIDI counts as the first pulse after Start;
    /// - note ons last.
    fn order_at_same_position(self) -> u8 {
        match self {
            OutputAction::NoteOff { .. } => 0,
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
    BeatPulse {
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
/// When a beat pulse is longer than a count, a cue lies past the end of
/// the stretch, or a position cannot be counted.
pub fn schedule(
    clock_port: &OutputPortName,
    control_tracks: &[ControlTrack],
    time_signature: TimeSignature,
    stretch_length: ScoreLength,
) -> Result<Vec<ScheduledEvent>, ScheduleError> {
    let mut events: Vec<ScheduledEvent> = timing_clock(clock_port, stretch_length)?;
    for track in control_tracks {
        if let Some(beat_pulse) = track.beat_pulse() {
            events.extend(beat_pulse_events(
                track.name(),
                beat_pulse,
                time_signature,
                stretch_length,
            )?);
        }
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

/// A note on each count, count 1 of each bar accented, each held for the
/// pulse's length (EFS-2.6).
fn beat_pulse_events(
    track_name: &str,
    beat_pulse: &BeatPulse,
    time_signature: TimeSignature,
    stretch_length: ScoreLength,
) -> Result<Vec<ScheduledEvent>, ScheduleError> {
    let count_length_ticks: u64 = time_signature.count_length().ticks();
    let bar_length_ticks: u64 = time_signature.bar_length().ticks();
    if beat_pulse.length().ticks() > count_length_ticks {
        return Err(ScheduleError::BeatPulseLongerThanCount {
            track_name: track_name.to_string(),
            pulse_length_ticks: beat_pulse.length().ticks(),
            count_length_ticks,
        });
    }
    let source: EventSource = EventSource::BeatPulse {
        track_name: track_name.to_string(),
    };
    let mut events: Vec<ScheduledEvent> = Vec::new();
    let mut count_start_ticks: u64 = 0;
    while count_start_ticks < stretch_length.ticks() {
        // Count 1 starts each bar: its offset into the bar is 0 ticks.
        let offset_into_bar_ticks: u64 = count_start_ticks % bar_length_ticks;
        let velocity: Velocity = if offset_into_bar_ticks == 0 {
            beat_pulse.accent_velocity()
        } else {
            beat_pulse.normal_velocity()
        };
        // Within a count, so no overflow: count start + at most one count
        // length, and count start is below the stretch length.
        let note_off_ticks: u64 = count_start_ticks + beat_pulse.length().ticks();
        events.push(ScheduledEvent {
            position: ScorePosition::from_ticks(count_start_ticks),
            port: beat_pulse.port().clone(),
            action: OutputAction::NoteOn {
                channel: beat_pulse.channel(),
                note: beat_pulse.note(),
                velocity,
            },
            source: source.clone(),
        });
        events.push(ScheduledEvent {
            position: ScorePosition::from_ticks(note_off_ticks),
            port: beat_pulse.port().clone(),
            action: OutputAction::NoteOff {
                channel: beat_pulse.channel(),
                note: beat_pulse.note(),
            },
            source: source.clone(),
        });
        count_start_ticks += count_length_ticks;
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
    BeatPulseLongerThanCount {
        track_name: String,
        pulse_length_ticks: u64,
        count_length_ticks: u64,
    },
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
            ScheduleError::BeatPulseLongerThanCount {
                track_name,
                pulse_length_ticks,
                count_length_ticks,
            } => write!(
                f,
                "track {track_name:?}: the beat pulse lasts {pulse_length_ticks} ticks, longer \
                 than a count ({count_length_ticks} ticks). Shorten it, e.g. to a 16th"
            ),
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
    use vocabulary::control_track::BeatPulse;

    fn port(name: &str) -> OutputPortName {
        OutputPortName::new(name).unwrap()
    }

    fn four_four() -> TimeSignature {
        TimeSignature::new(4, 4).unwrap()
    }

    fn beat_pulse_track(cues: Vec<PhraseCue>) -> ControlTrack {
        let pulse: BeatPulse = BeatPulse::new(
            port("Beat pulse"),
            MidiChannel::new(15).unwrap(),
            NoteNumber::new(60).unwrap(),
            Velocity::new(127).unwrap(),
            Velocity::new(64).unwrap(),
            ScoreLength::from_ticks(1260),
        )
        .unwrap();
        ControlTrack::new("Studio", Some(pulse), cues).unwrap()
    }

    #[test]
    fn one_bar_has_96_clock_pulses_and_4_beat_pulses() {
        let events: Vec<ScheduledEvent> = schedule(
            &port("Clock"),
            &[beat_pulse_track(Vec::new())],
            four_four(),
            ScoreLength::from_ticks(20_160),
        )
        .unwrap();
        let clock_pulses: usize = events
            .iter()
            .filter(|event: &&ScheduledEvent| event.action == OutputAction::ClockPulse)
            .count();
        // 20160 ticks ÷ 210 ticks/pulse = 96 pulses.
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
        let events: Vec<ScheduledEvent> = schedule(
            &port("Clock"),
            &[beat_pulse_track(vec![start])],
            four_four(),
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
    fn cue_past_the_end_is_refused() {
        let late: PhraseCue = PhraseCue::new(
            "Too late",
            ScorePosition::from_ticks(40_321),
            port("SQ-1"),
            ControlCueMessage::Transport(TransportMessage::Stop),
        )
        .unwrap();
        let track: ControlTrack = ControlTrack::new("Studio", None, vec![late]).unwrap();
        assert!(
            schedule(
                &port("Clock"),
                &[track],
                four_four(),
                ScoreLength::from_ticks(40_320)
            )
            .is_err()
        );
    }

    #[test]
    fn beat_pulse_longer_than_a_count_is_refused() {
        let pulse: BeatPulse = BeatPulse::new(
            port("Beat pulse"),
            MidiChannel::new(15).unwrap(),
            NoteNumber::new(60).unwrap(),
            Velocity::new(127).unwrap(),
            Velocity::new(64).unwrap(),
            ScoreLength::from_ticks(5041),
        )
        .unwrap();
        let track: ControlTrack = ControlTrack::new("Studio", Some(pulse), Vec::new()).unwrap();
        assert!(
            schedule(
                &port("Clock"),
                &[track],
                four_four(),
                ScoreLength::from_ticks(20_160)
            )
            .is_err()
        );
    }
}
