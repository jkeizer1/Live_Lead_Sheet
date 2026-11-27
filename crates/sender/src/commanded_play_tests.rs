//! The sending loop under command: played whole, from a later place,
//! stopped, looped, told to play while playing, played again and again, and
//! played to its natural end. Each test runs the loop on a recording
//! output with a simulated clock and compares what it handed over, and
//! when, with what the score says should be sent. The expected instants
//! are worked out here from the tempo (120 quarter notes/min), not by the
//! code under test.

use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc::{self, Receiver, Sender};

use player::schedule::{OutputAction, ScheduledEvent, schedule};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::duration::Duration;
use vocabulary::midi_values::MidiChannel;
use vocabulary::phrase_cue::TransportMessage;
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;

use crate::play_report::PlayReport;
use crate::play_request::{PlayRequest, Request};
use crate::recording_output::{RecordingOutput, ScriptedCommand, Sent, SentEvent, SentNow};
use crate::send_loop::{NO_POSITION, Shared, serve};

const SONG: &str = include_str!("../../../songs_canonical/sender/play_plain.song");
const STUDIO: &str = include_str!("../../../songs_canonical/sender/play_plain.studio");
const DELAYED_STUDIO: &str = include_str!("../../../songs_canonical/sender/play_delay.studio");

/// Where the simulated clock starts.
const START_NS: u64 = 1_000_000_000;
/// From taking a play request to its first event.
const START_DELAY_NS: u64 = 200_000_000;
/// The output's own lead before a note-on that a note-off touches.
const TOUCHING_LEAD_NS: u64 = 5_000_000;
/// 4 q × 5040 ticks/q.
const TICKS_PER_BAR: u64 = 4 * TICKS_PER_QUARTER_NOTE;
/// The tempo of the test song, in milli-quarters per minute.
const MILLI_QUARTERS_PER_MINUTE: u128 = 120_000;
const PORTS: [&str; 4] = ["Clock", "Harmony", "Bass", "Beat pulse"];

struct TestSong {
    events: Vec<ScheduledEvent>,
    plan: SongPlan,
}

fn load(studio_text: &str) -> TestSong {
    let song = parse_song("play_plain.song", SONG).unwrap();
    let studio = parse_studio("play_plain.studio", studio_text).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let events: Vec<ScheduledEvent> = schedule(
        &plan.clock_port,
        &plan.note_tracks,
        &plan.control_tracks,
        plan.stretch_length,
    )
    .unwrap();
    TestSong { events, plan }
}

fn request(
    song: &TestSong,
    start_ticks: u64,
    end_ticks: Option<u64>,
    looping: bool,
) -> PlayRequest {
    PlayRequest {
        events: song.events.clone(),
        tempo: song.plan.tempo,
        tempo_changes: song.plan.tempo_changes.clone(),
        start_ticks,
        end_ticks,
        looping,
        clock_port: song.plan.clock_port.clone(),
        note_outputs: song
            .plan
            .note_tracks
            .iter()
            .map(|track| (track.port().clone(), track.channel()))
            .collect(),
    }
}

/// How long `ticks` last at 120 quarter notes/min, to the nearest
/// nanosecond: ticks × 60e9 ns/min × 1000 mq/q ÷ (120000 mq/min × 5040
/// ticks/q).
fn duration_ns(ticks: u64) -> u64 {
    let denominator: u128 = MILLI_QUARTERS_PER_MINUTE * u128::from(TICKS_PER_QUARTER_NOTE);
    let numerator: u128 = u128::from(ticks) * 60_000_000_000 * 1000;
    u64::try_from((numerator + denominator / 2) / denominator).unwrap()
}

/// Plays `first` and then whatever `script` sends, until the loop has
/// ended `plays` plays. Returns the output, and the problems the loop met.
fn run(
    first: PlayRequest,
    plays: usize,
    script: impl FnOnce(&RecordingOutput),
) -> (RecordingOutput, Vec<String>) {
    let (output, problems, _): (RecordingOutput, Vec<String>, Vec<PlayReport>) =
        run_with_reports(first, plays, script);
    (output, problems)
}

/// As `run`, and the report of each play.
fn run_with_reports(
    first: PlayRequest,
    plays: usize,
    script: impl FnOnce(&RecordingOutput),
) -> (RecordingOutput, Vec<String>, Vec<PlayReport>) {
    let (commands, receiver): (Sender<Request>, Receiver<Request>) = mpsc::channel();
    let (problem_sender, problems): (Sender<String>, Receiver<String>) = mpsc::channel();
    let (report_sender, reports): (Sender<PlayReport>, Receiver<PlayReport>) = mpsc::channel();
    let output: RecordingOutput = RecordingOutput::new(&PORTS, START_NS, commands.clone(), plays);
    commands.send(Request::Play(Box::new(first))).unwrap();
    script(&output);
    let shared: Shared = Shared {
        position_ticks: AtomicU64::new(NO_POSITION),
        playing: AtomicBool::new(false),
        looping: AtomicBool::new(false),
    };
    serve(&output, &receiver, &problem_sender, &report_sender, &shared);
    (
        output,
        problems.try_iter().collect(),
        reports.try_iter().collect(),
    )
}

/// What the loop must hand over for one event of the score, and the
/// instant it is due: the event's position counted from `start_ticks` on
/// an anchor at `first_ns`, plus the track's delay.
fn expected_event(event: &ScheduledEvent, start_ticks: u64, first_ns: u64) -> (SentEvent, u64) {
    let port: String = event.port.as_str().to_string();
    let at_ns: u64 =
        first_ns + duration_ns(event.position.ticks() - start_ticks) + event.delay.ns();
    let sent: SentEvent = match event.action {
        OutputAction::ClockPulse => SentEvent::ClockPulse { port },
        OutputAction::NoteOn {
            channel,
            note,
            velocity,
        } => SentEvent::NoteOn {
            port,
            channel: channel.wire_value(),
            note: note.value(),
            velocity: velocity.value(),
        },
        OutputAction::NoteOff { channel, note } | OutputAction::LegatoNoteOff { channel, note } => {
            SentEvent::NoteOff {
                port,
                channel: channel.wire_value(),
                note: note.value(),
            }
        }
        OutputAction::TouchingNoteOff {
            channel,
            note,
            note_on_position,
            retrigger_gap,
        } => {
            if note_on_position.ticks() < start_ticks {
                // Its note-on was before the start: nothing to shorten.
                SentEvent::NoteOff {
                    port,
                    channel: channel.wire_value(),
                    note: note.value(),
                }
            } else {
                let lead_ns: u64 = retrigger_gap.ns().max(TOUCHING_LEAD_NS);
                return (
                    SentEvent::TouchingNoteOff {
                        port,
                        channel: channel.wire_value(),
                        note: note.value(),
                        note_on_instant_ns: first_ns
                            + duration_ns(note_on_position.ticks() - start_ticks)
                            + event.delay.ns(),
                        next_note_on_instant_ns: at_ns,
                        retrigger_gap_ns: retrigger_gap.ns(),
                    },
                    at_ns - lead_ns,
                );
            }
        }
        OutputAction::ControlChange {
            channel,
            controller,
            value,
        } => SentEvent::ControlChange {
            port,
            channel: channel.wire_value(),
            controller: controller.value(),
            value: value.value(),
        },
        OutputAction::ProgramChange { channel, program } => SentEvent::ProgramChange {
            port,
            channel: channel.wire_value(),
            program: program.value(),
        },
        OutputAction::Transport(TransportMessage::Start) => SentEvent::Start { port },
        OutputAction::Transport(TransportMessage::Stop) => SentEvent::Stop { port },
        OutputAction::Transport(TransportMessage::Continue) => SentEvent::Continue { port },
    };
    (sent, at_ns)
}

/// Whether the loop sends the event in a stretch ending at `end_ticks`:
/// a note-off at the end belongs to the stretch, a note-on at the end to
/// the next one.
fn in_stretch(event: &ScheduledEvent, end_ticks: Option<u64>) -> bool {
    let Some(end) = end_ticks else {
        return true;
    };
    let ends_note: bool = matches!(
        event.action,
        OutputAction::NoteOff { .. }
            | OutputAction::TouchingNoteOff { .. }
            | OutputAction::LegatoNoteOff { .. }
    );
    event.position.ticks() < end || (event.position.ticks() == end && ends_note)
}

/// Everything the loop must hand over for one stretch of the song, in
/// sending order, each with its due instant. `offset_ns` is how much later
/// than `first_ns` the stretch's anchor is (a later pass of a loop).
fn expected_stretch(
    song: &TestSong,
    start_ticks: u64,
    end_ticks: Option<u64>,
    first_ns: u64,
) -> Vec<(SentEvent, u64)> {
    song.events
        .iter()
        .filter(|event| event.position.ticks() >= start_ticks && in_stretch(event, end_ticks))
        .map(|event| expected_event(event, start_ticks, first_ns))
        .collect()
}

fn pairs(delivered: &[Sent]) -> Vec<(SentEvent, u64)> {
    delivered
        .iter()
        .map(|sent| (sent.event.clone(), sent.due_instant_ns))
        .collect()
}

/// The events of `expected` due by `limit_ns`: what has been delivered when
/// a play ends then.
fn due_by(expected: Vec<(SentEvent, u64)>, limit_ns: u64) -> Vec<(SentEvent, u64)> {
    expected
        .into_iter()
        .filter(|(_, due_ns)| *due_ns <= limit_ns)
        .collect()
}

/// No event was handed over after its instant.
fn assert_none_late(sent: &[Sent]) {
    for item in sent {
        assert!(item.handed_over_ns <= item.due_instant_ns, "late: {item:?}");
    }
}

#[test]
fn a_whole_song_played_from_the_start_is_handed_over_event_for_event() {
    let song: TestSong = load(STUDIO);
    let (output, problems) = run(request(&song, 0, None, false), 1, |_| {});
    assert_eq!(problems, Vec::<String>::new());
    let first_ns: u64 = START_NS + START_DELAY_NS;
    let expected = expected_stretch(&song, 0, None, first_ns);
    assert!(
        expected.len() > 100,
        "the song has events: {}",
        expected.len()
    );
    assert_eq!(pairs(&output.delivered()), expected);
    assert_none_late(&output.sent());
}

#[test]
fn a_touching_note_off_is_placed_before_the_next_note_on_by_the_studios_gap() {
    let song: TestSong = load(STUDIO);
    let (output, _) = run(request(&song, 0, None, false), 1, |_| {});
    // Only the bass has a retrigger gap in the studio.
    let touching: Vec<Sent> = output
        .delivered()
        .into_iter()
        .filter(
            |sent| matches!(&sent.event, SentEvent::TouchingNoteOff { port, .. } if port == "Bass"),
        )
        .collect();
    assert!(!touching.is_empty(), "the bass has touching notes");
    for sent in &touching {
        let SentEvent::TouchingNoteOff {
            next_note_on_instant_ns,
            retrigger_gap_ns,
            ..
        } = sent.event
        else {
            continue;
        };
        assert_eq!(retrigger_gap_ns, Duration::from_ms(18).ns());
        assert_eq!(
            sent.due_instant_ns,
            next_note_on_instant_ns - 18_000_000,
            "{sent:?}"
        );
    }
}

#[test]
fn played_from_a_later_bar_nothing_before_it_is_sent_and_start_goes_first() {
    let song: TestSong = load(STUDIO);
    // The start of the second bar of the verse's first pass.
    let start_ticks: u64 = 2 * TICKS_PER_BAR;
    let (output, problems) = run(request(&song, start_ticks, None, false), 1, |_| {});
    assert_eq!(problems, Vec::<String>::new());
    let first_ns: u64 = START_NS + START_DELAY_NS;
    let mut expected: Vec<(SentEvent, u64)> = vec![(
        SentEvent::Start {
            port: String::from("Clock"),
        },
        first_ns,
    )];
    expected.extend(expected_stretch(&song, start_ticks, None, first_ns));
    assert_eq!(pairs(&output.delivered()), expected);
    assert_none_late(&output.sent());
}

#[test]
fn a_note_held_across_the_start_gets_a_plain_note_off_not_a_shortened_one() {
    let song: TestSong = load(STUDIO);
    // Inside the second bar of the verse's first pass, where the bass holds
    // a note from the bar before.
    let start_ticks: u64 = 2 * TICKS_PER_BAR + TICKS_PER_QUARTER_NOTE;
    let (output, _) = run(request(&song, start_ticks, None, false), 1, |_| {});
    let first_ns: u64 = START_NS + START_DELAY_NS;
    let mut expected: Vec<(SentEvent, u64)> = vec![(
        SentEvent::Start {
            port: String::from("Clock"),
        },
        first_ns,
    )];
    expected.extend(expected_stretch(&song, start_ticks, None, first_ns));
    assert_eq!(pairs(&output.delivered()), expected);
}

#[test]
fn stopped_in_the_middle_nothing_later_is_delivered_and_every_port_is_silenced() {
    let song: TestSong = load(STUDIO);
    let stop_ns: u64 = START_NS + START_DELAY_NS + 4_300_000_000;
    let (output, problems) = run(request(&song, 0, None, false), 1, |output| {
        output.at(stop_ns, ScriptedCommand::Stop);
    });
    assert_eq!(problems, Vec::<String>::new());
    let first_ns: u64 = START_NS + START_DELAY_NS;
    let stopped_at_ns: u64 = output.silenced_at_ns()[0];
    assert!(stopped_at_ns >= stop_ns && stopped_at_ns < stop_ns + 10_000_000);
    let expected = due_by(expected_stretch(&song, 0, None, first_ns), stopped_at_ns);
    assert_eq!(pairs(&output.delivered()), expected);
    let now: Vec<SentNow> = output
        .sent_now()
        .into_iter()
        .map(|(_, sent)| sent)
        .collect();
    assert!(now.contains(&SentNow::Stop {
        port: String::from("Clock"),
    }));
    for (port, channel) in [("Bass", 2_u8), ("Beat pulse", 15), ("Harmony", 1)] {
        assert!(
            now.contains(&SentNow::AllNotesOff {
                port: port.to_string(),
                channel: MidiChannel::new(channel).unwrap().wire_value(),
            }),
            "{port} is silenced: {now:?}"
        );
    }
}

/// The notes the receivers are left holding when a play ends, before Stop's
/// own Note Offs: each note whose Note On was handed over (delivered or not)
/// and whose Note Off after it was taken back or never handed over. Worked
/// out from what the output recorded, in the order it was handed over.
fn notes_left_sounding(sent: &[Sent]) -> Vec<(String, u8, u8)> {
    let mut sounding: Vec<(String, u8, u8)> = Vec::new();
    for item in sent {
        match &item.event {
            SentEvent::NoteOn {
                port,
                channel,
                note,
                ..
            } => {
                let key: (String, u8, u8) = (port.clone(), *channel, *note);
                if !sounding.contains(&key) {
                    sounding.push(key);
                }
            }
            SentEvent::NoteOff {
                port,
                channel,
                note,
            }
            | SentEvent::TouchingNoteOff {
                port,
                channel,
                note,
                ..
            } if !item.cancelled => {
                sounding.retain(|key| *key != (port.clone(), *channel, *note));
            }
            _ => {}
        }
    }
    sounding
}

#[test]
fn stopped_during_notes_each_sounding_note_gets_a_note_off_before_stop() {
    let song: TestSong = load(STUDIO);
    // 4.3 s at 120 q/min is 8.6 q: inside beat 1 of the song's third bar,
    // while notes are held.
    let stop_ns: u64 = START_NS + START_DELAY_NS + 4_300_000_000;
    let (output, problems) = run(request(&song, 0, None, false), 1, |output| {
        output.at(stop_ns, ScriptedCommand::Stop);
    });
    assert_eq!(problems, Vec::<String>::new());
    let mut expected: Vec<(String, u8, u8)> = notes_left_sounding(&output.sent());
    assert!(!expected.is_empty(), "the stop falls while notes sound");
    let now: Vec<SentNow> = output
        .sent_now()
        .into_iter()
        .map(|(_, sent)| sent)
        .collect();
    let mut note_offs: Vec<(String, u8, u8)> = now
        .iter()
        .filter_map(|sent| match sent {
            SentNow::NoteOff {
                port,
                channel,
                note,
            } => Some((port.clone(), *channel, *note)),
            _ => None,
        })
        .collect();
    expected.sort();
    note_offs.sort();
    assert_eq!(note_offs, expected);
    // Every Note Off comes before Stop and All Notes Off.
    let last_note_off: usize = now
        .iter()
        .rposition(|sent| matches!(sent, SentNow::NoteOff { .. }))
        .unwrap();
    let first_other: usize = now
        .iter()
        .position(|sent| !matches!(sent, SentNow::NoteOff { .. }))
        .unwrap();
    assert!(last_note_off < first_other, "{now:?}");
}

#[test]
fn played_to_its_natural_end_no_note_off_is_sent_at_once() {
    let song: TestSong = load(STUDIO);
    let (output, problems) = run(request(&song, 0, None, false), 1, |_| {});
    assert_eq!(problems, Vec::<String>::new());
    let note_offs: Vec<SentNow> = output
        .sent_now()
        .into_iter()
        .map(|(_, sent)| sent)
        .filter(|sent| matches!(sent, SentNow::NoteOff { .. }))
        .collect();
    assert_eq!(note_offs, Vec::<SentNow>::new());
}

#[test]
fn a_looped_stretch_comes_round_on_the_same_beat_each_time() {
    let song: TestSong = load(STUDIO);
    // The verse's two bars, played in a loop.
    let start_ticks: u64 = TICKS_PER_BAR;
    let end_ticks: u64 = 3 * TICKS_PER_BAR;
    let loop_ns: u64 = duration_ns(end_ticks - start_ticks);
    let first_ns: u64 = START_NS + START_DELAY_NS;
    let stop_ns: u64 = first_ns + loop_ns * 5 / 2;
    let (output, problems) = run(
        request(&song, start_ticks, Some(end_ticks), true),
        1,
        |output| output.at(stop_ns, ScriptedCommand::Stop),
    );
    assert_eq!(problems, Vec::<String>::new());
    let stopped_at_ns: u64 = output.silenced_at_ns()[0];
    let mut expected: Vec<(SentEvent, u64)> = vec![(
        SentEvent::Start {
            port: String::from("Clock"),
        },
        first_ns,
    )];
    for pass in 0..3_u64 {
        expected.extend(expected_stretch(
            &song,
            start_ticks,
            Some(end_ticks),
            first_ns + pass * loop_ns,
        ));
    }
    assert_eq!(pairs(&output.delivered()), due_by(expected, stopped_at_ns));
}

#[test]
fn a_play_command_while_playing_takes_back_the_first_plays_events_and_starts_the_second_clean() {
    let song: TestSong = load(STUDIO);
    let second_ticks: u64 = 3 * TICKS_PER_BAR;
    let command_ns: u64 = START_NS + START_DELAY_NS + 3_100_000_000;
    let (output, problems) = run(request(&song, 0, None, false), 2, |output| {
        output.at(
            command_ns,
            ScriptedCommand::Play(Box::new(request(&song, second_ticks, None, false))),
        );
    });
    assert_eq!(problems, Vec::<String>::new());
    let silenced: Vec<u64> = output.silenced_at_ns();
    let first_ns: u64 = START_NS + START_DELAY_NS;
    let second_first_ns: u64 = silenced[0] + START_DELAY_NS;
    let mut expected: Vec<(SentEvent, u64)> =
        due_by(expected_stretch(&song, 0, None, first_ns), silenced[0]);
    expected.push((
        SentEvent::Start {
            port: String::from("Clock"),
        },
        second_first_ns,
    ));
    expected.extend(expected_stretch(&song, second_ticks, None, second_first_ns));
    assert_eq!(pairs(&output.delivered()), expected);
}

#[test]
fn play_stop_play_stop_play_each_starts_clean_and_the_last_runs_to_the_end() {
    let song: TestSong = load(STUDIO);
    let first_ns: u64 = START_NS + START_DELAY_NS;
    let first_stop_ns: u64 = first_ns + 1_500_000_000;
    let (output, problems) = run(request(&song, 0, None, false), 3, |output| {
        // The clock only moves while a play is running, so each Play is sent
        // together with the Stop before it: the loop takes the Stop, ends
        // the play, and takes the Play next.
        output.at(first_stop_ns, ScriptedCommand::Stop);
        output.at(
            first_stop_ns,
            ScriptedCommand::Play(Box::new(request(&song, TICKS_PER_BAR, None, false))),
        );
        output.at(first_stop_ns + 2_000_000_000, ScriptedCommand::Stop);
        output.at(
            first_stop_ns + 2_000_000_000,
            ScriptedCommand::Play(Box::new(request(&song, 0, None, false))),
        );
    });
    assert_eq!(problems, Vec::<String>::new());
    let silenced: Vec<u64> = output.silenced_at_ns();
    assert_eq!(silenced.len(), 3);
    let second_first_ns: u64 = silenced[0] + START_DELAY_NS;
    let third_first_ns: u64 = silenced[1] + START_DELAY_NS;
    let mut expected: Vec<(SentEvent, u64)> =
        due_by(expected_stretch(&song, 0, None, first_ns), silenced[0]);
    expected.push((
        SentEvent::Start {
            port: String::from("Clock"),
        },
        second_first_ns,
    ));
    expected.extend(due_by(
        expected_stretch(&song, TICKS_PER_BAR, None, second_first_ns),
        silenced[1],
    ));
    // The last play runs to its natural end: every event of the song.
    expected.extend(expected_stretch(&song, 0, None, third_first_ns));
    assert_eq!(pairs(&output.delivered()), expected);
}

#[test]
fn a_song_played_to_its_natural_end_delivers_every_event_even_with_a_delay_trigger() {
    let song: TestSong = load(DELAYED_STUDIO);
    let (output, problems) = run(request(&song, 0, None, false), 1, |_| {});
    assert_eq!(problems, Vec::<String>::new());
    let first_ns: u64 = START_NS + START_DELAY_NS;
    let expected = expected_stretch(&song, 0, None, first_ns);
    let delivered = pairs(&output.delivered());
    let missing: Vec<&(SentEvent, u64)> = expected
        .iter()
        .filter(|event| !delivered.contains(event))
        .collect();
    assert_eq!(missing, Vec::<&(SentEvent, u64)>::new(), "never delivered");
    assert_eq!(delivered, expected);
}

#[test]
fn a_play_with_the_loop_looking_every_5_ms_reports_no_late_event() {
    let song: TestSong = load(STUDIO);
    let (output, _, reports) = run_with_reports(request(&song, 0, None, false), 1, |_| {});
    assert_eq!(reports.len(), 1);
    assert_eq!(
        reports[0].events_sent,
        u64::try_from(output.sent().len()).unwrap()
    );
    assert_eq!(reports[0].late_events, 0);
    assert_eq!(reports[0].worst_lateness, Duration::ZERO);
    assert_eq!(reports[0].longest_pause, Duration::from_ms(5));
}

#[test]
fn the_report_lists_the_first_notes_handed_over_with_their_offsets() {
    let song: TestSong = load(STUDIO);
    let (_, _, reports) = run_with_reports(request(&song, 0, None, false), 1, |_| {});
    // The count-in's four beats, a quarter note (500 ms at 120 q/min) apart.
    assert_eq!(
        reports[0].first_notes[..4],
        [
            "Beat pulse note 60 at +0 ms",
            "Beat pulse note 60 at +500 ms",
            "Beat pulse note 60 at +1000 ms",
            "Beat pulse note 60 at +1500 ms",
        ]
    );
    assert_eq!(reports[0].first_notes.len(), 6);
}

#[test]
fn a_loop_held_up_for_80_ms_reports_the_pause_and_the_events_it_handed_over_late() {
    let song: TestSong = load(STUDIO);
    let held_at_ns: u64 = START_NS + START_DELAY_NS + 3_000_000_000;
    let (output, _, reports) = run_with_reports(request(&song, 0, None, false), 1, |output| {
        output.stall_once(held_at_ns, 80_000_000);
    });
    assert_eq!(reports.len(), 1);
    let report: PlayReport = reports[0].clone();
    assert_eq!(report.longest_pause, Duration::from_ms(85));
    assert!(report.late_events > 0, "{report:?}");
    // The look before the pause was 5 ms before it began; events due
    // within the 50 ms lead of that look were handed over in time, so none
    // is later than the pause less the lead.
    assert!(report.worst_lateness <= Duration::from_ms(35), "{report:?}");
    assert!(report.worst_lateness > Duration::ZERO);
    // The count in the report is the count the output saw.
    let late_by_output: u64 = u64::try_from(
        output
            .sent()
            .iter()
            .filter(|sent| sent.handed_over_ns > sent.due_instant_ns)
            .count(),
    )
    .unwrap();
    assert_eq!(report.late_events, late_by_output);
}
