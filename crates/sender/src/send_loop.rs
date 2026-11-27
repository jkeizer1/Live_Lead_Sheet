//! The sending loop: waits for commands, and plays a schedule to an
//! output, handing events over up to 50 ms ahead (D-8), their instants
//! computed from anchors (Musical Time and Clock, section 8). It was the
//! text player's loop (`song_text/examples/support/event_player.rs`),
//! given Stop, a start place and looping.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration as StdDuration;

use alsa_output::sequencer_output::{FlushOutcome, TOUCHING_NOTE_OFF_LEAD};
use player::anchor::Anchor;
use player::schedule::{OutputAction, ScheduledEvent};
use player::song_plan::TempoChange;
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::phrase_cue::TransportMessage;
use vocabulary::score_position::ScorePosition;
use vocabulary::tempo::Tempo;

use crate::event_output::EventOutput;
use crate::play_report::PlayReport;
use crate::play_request::{PlayRequest, Request};
use crate::sounding_notes::{SoundingNote, SoundingNotes};

/// The player's lead window (D-8).
const LEAD: Duration = Duration::from_ms(50);
/// Time between the request and the first event, so the first events are
/// not already late when sent.
const START_DELAY: Duration = Duration::from_ms(200);
/// How long the thread waits for a request when idle.
pub(crate) const IDLE_WAIT: StdDuration = StdDuration::from_millis(50);
/// No position: the counter holds this while stopped.
pub(crate) const NO_POSITION: u64 = u64::MAX;

/// What the window reads.
pub(crate) struct Shared {
    pub(crate) position_ticks: AtomicU64,
    pub(crate) playing: AtomicBool,
    pub(crate) looping: AtomicBool,
}

/// Waits for requests and carries them out until Quit.
pub(crate) fn serve<O: EventOutput>(
    output: &O,
    requests: &Receiver<Request>,
    problems: &Sender<String>,
    reports: &Sender<PlayReport>,
    shared: &Shared,
) {
    let mut next: Option<Request> = None;
    loop {
        let request: Request = match next.take() {
            Some(request) => request,
            None => match requests.recv_timeout(IDLE_WAIT) {
                Ok(request) => request,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            },
        };
        match request {
            Request::Quit => return,
            Request::Stop => {}
            Request::Play(play) => {
                shared.playing.store(true, Ordering::Relaxed);
                shared.looping.store(play.looping, Ordering::Relaxed);
                let mut report: PlayReport = PlayReport::default();
                let mut sounding: SoundingNotes = SoundingNotes::default();
                let outcome: Result<Option<Request>, String> = play_until_interrupted(
                    output,
                    &play,
                    requests,
                    shared,
                    &mut report,
                    &mut sounding,
                );
                let _ = reports.send(report);
                shared.playing.store(false, Ordering::Relaxed);
                shared.looping.store(false, Ordering::Relaxed);
                shared.position_ticks.store(NO_POSITION, Ordering::Relaxed);
                if let Err(problem) = silence(output, &play, &mut sounding) {
                    let _ = problems.send(problem);
                }
                match outcome {
                    Ok(interrupted_by) => next = interrupted_by,
                    Err(problem) => {
                        let _ = problems.send(problem);
                    }
                }
                if matches!(next, Some(Request::Quit)) {
                    return;
                }
            }
        }
    }
}

/// Stop, in the order of Musical Time and Clock, section 8: removes what is
/// queued, sends a Note Off at once for each note still sounding, then Stop,
/// then All Notes Off as the final safety net.
fn silence<O: EventOutput>(
    output: &O,
    play: &PlayRequest,
    sounding: &mut SoundingNotes,
) -> Result<(), String> {
    // The clock is read before the removal: a Note Off due between the two
    // is then counted as removed and sent again, which is harmless.
    let now: Instant = output.now()?;
    let still_sounding: Vec<SoundingNote> = sounding.take_sounding_at(now);
    output
        .remove_queued_events()
        .map_err(|error| format!("Stopping: {error}"))?;
    for note in &still_sounding {
        output
            .send_note_off_now(
                note.port.as_str(),
                note.channel.wire_value(),
                note.note.value(),
            )
            .map_err(|error| format!("Ending note {}: {error}", note.note.value()))?;
    }
    if output.has_port(play.clock_port.as_str()) {
        output
            .send_stop_now(play.clock_port.as_str())
            .map_err(|error| format!("Sending Stop: {error}"))?;
    }
    for (port_name, channel) in &play.note_outputs {
        output
            .send_all_notes_off_now(port_name.as_str(), channel.wire_value())
            .map_err(|error| format!("Silencing {}: {error}", port_name.as_str()))?;
    }
    match output
        .flush()
        .map_err(|error| format!("Stopping: {error}"))?
    {
        FlushOutcome::AllHandedOver => Ok(()),
        FlushOutcome::PoolFull => Err(String::from(
            "Stopping: the MIDI output was too busy to send All Notes Off; press Space twice to try again",
        )),
    }
}

/// The tempo in force at `position`: the last written change at or before
/// it, else the song's starting tempo.
fn tempo_at(play: &PlayRequest, position: ScorePosition) -> Tempo {
    play.tempo_changes
        .iter()
        .take_while(|change: &&TempoChange| change.position <= position)
        .last()
        .map_or(play.tempo, |change: &TempoChange| change.tempo)
}

/// Whether an event at the end of a stretch still belongs to it: a note
/// that ends there must be let go.
fn ends_a_note(action: OutputAction) -> bool {
    matches!(
        action,
        OutputAction::NoteOff { .. }
            | OutputAction::TouchingNoteOff { .. }
            | OutputAction::LegatoNoteOff { .. }
    )
}

/// Plays until the end (or forever, looping), or until a request comes;
/// returns that request, so a Play while playing starts the new song.
fn play_until_interrupted<O: EventOutput>(
    output: &O,
    play: &PlayRequest,
    requests: &Receiver<Request>,
    shared: &Shared,
    report: &mut PlayReport,
    sounding: &mut SoundingNotes,
) -> Result<Option<Request>, String> {
    let start: ScorePosition = ScorePosition::from_ticks(play.start_ticks);
    let end: Option<ScorePosition> = play.end_ticks.map(ScorePosition::from_ticks);
    let first_event: usize = play
        .events
        .partition_point(|event: &ScheduledEvent| event.position < start);
    let first_instant: Instant = output
        .now()?
        .checked_add(START_DELAY)
        .map_err(|error| error.to_string())?;
    let mut anchors: Vec<Anchor> = vec![Anchor::new(start, first_instant, tempo_at(play, start))];
    let mut next_change: usize = play
        .tempo_changes
        .partition_point(|change: &TempoChange| change.position <= start);
    let mut next: usize = first_event;
    let mut pool_full: bool = false;
    // The latest instant anything handed over is due, delays included: the
    // play is not over before it, or the last delayed events would be
    // taken back unsent.
    let mut last_due: Instant = first_instant;
    let mut previous_look: Option<Instant> = None;
    // Followers start with the song: Start before the first clock pulse
    // when playing from anywhere but the start (the schedule has its own
    // Start at the start).
    if play.start_ticks > 0 {
        output.start(play.clock_port.as_str(), first_instant)?;
    }
    loop {
        match requests.try_recv() {
            Ok(Request::Stop) => return Ok(None),
            Ok(request) => return Ok(Some(request)),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return Ok(Some(Request::Quit)),
        }
        let now: Instant = output.now()?;
        if let Some(previous) = previous_look
            && let Ok(pause) = now.checked_sub(previous)
        {
            report.note_pause(pause);
        }
        previous_look = Some(now);
        sounding.forget_ended(now);
        publish_position(shared, &anchors, now);
        let horizon: Instant = now.checked_add(LEAD).map_err(|error| error.to_string())?;
        while let Some(event) = play.events.get(next) {
            if pool_full {
                break;
            }
            let past_end: bool = end.is_some_and(|end| {
                event.position > end || (event.position == end && !ends_a_note(event.action))
            });
            if past_end {
                break;
            }
            while let Some(change) = play.tempo_changes.get(next_change) {
                if change.position > event.position {
                    break;
                }
                let before: Anchor = anchors[anchors.len() - 1];
                let instant: Instant = before
                    .instant_at(change.position)
                    .map_err(|error| error.to_string())?;
                anchors.push(Anchor::new(change.position, instant, change.tempo));
                next_change += 1;
            }
            let at: Instant = instant_in(&anchors, event.position)?
                .checked_add(event.delay)
                .map_err(|error| error.to_string())?;
            if at > horizon {
                break;
            }
            report.note_event(lateness(now, event, at, &anchors)?);
            report.note_first(first_note_text(event, at, first_instant));
            send(output, event, at, &anchors)?;
            track_sounding(sounding, event, at, &anchors);
            if at > last_due {
                last_due = at;
            }
            next += 1;
        }
        match output.flush()? {
            FlushOutcome::AllHandedOver => pool_full = false,
            FlushOutcome::PoolFull => pool_full = true,
        }
        let finished: bool = play.events.get(next).is_none_or(|event| {
            end.is_some_and(|end| {
                event.position > end || (event.position == end && !ends_a_note(event.action))
            })
        });
        if finished && !pool_full {
            let end_position: ScorePosition =
                end.unwrap_or_else(|| play.events.last().map_or(start, |event| event.position));
            let end_instant: Instant = instant_in(&anchors, end_position)?;
            if play.looping {
                // Round again: the start comes at the instant the end
                // would have, with the start's tempo.
                anchors = vec![Anchor::new(start, end_instant, tempo_at(play, start))];
                next_change = play
                    .tempo_changes
                    .partition_point(|change: &TempoChange| change.position <= start);
                next = first_event;
            } else if now >= end_instant && now >= last_due {
                return Ok(None);
            }
        }
        output.wait_a_little();
    }
}

/// Where the song is at `now`, for the window.
fn publish_position(shared: &Shared, anchors: &[Anchor], now: Instant) {
    let Some(anchor) = anchors
        .iter()
        .rev()
        .find(|anchor: &&Anchor| anchor.instant() <= now)
    else {
        // Before the first event: the start.
        if let Some(first) = anchors.first() {
            shared
                .position_ticks
                .store(first.position().ticks(), Ordering::Relaxed);
        }
        return;
    };
    if let Ok(position) = anchor.position_at(now) {
        shared
            .position_ticks
            .store(position.ticks(), Ordering::Relaxed);
    }
}

/// A note-on described for the report: the port, the note number, and how
/// long after the play's first instant it is due.
fn first_note_text(event: &ScheduledEvent, at: Instant, first_instant: Instant) -> Option<String> {
    let OutputAction::NoteOn { note, .. } = event.action else {
        return None;
    };
    let after_ms: u64 = at
        .checked_sub(first_instant)
        .map_or(0, |after| after.ns() / 1_000_000);
    Some(format!(
        "{} note {} at +{after_ms} ms",
        event.port.as_str(),
        note.value()
    ))
}

/// How far ahead of its instant the output must be given an event: a
/// note-off placed before the next note-on is sent earlier than that
/// note-on by the larger of the output's lead and the studio's gap.
fn send_lead(event: &ScheduledEvent, anchors: &[Anchor]) -> Duration {
    match event.action {
        OutputAction::TouchingNoteOff {
            note_on_position,
            retrigger_gap,
            ..
        } if anchors
            .first()
            .is_none_or(|start: &Anchor| note_on_position >= start.position()) =>
        {
            if retrigger_gap > TOUCHING_NOTE_OFF_LEAD {
                retrigger_gap
            } else {
                TOUCHING_NOTE_OFF_LEAD
            }
        }
        _ => Duration::ZERO,
    }
}

/// Notes which notes `event`, just handed over for `at`, starts or ends.
fn track_sounding(
    sounding: &mut SoundingNotes,
    event: &ScheduledEvent,
    at: Instant,
    anchors: &[Anchor],
) {
    match event.action {
        OutputAction::NoteOn { channel, note, .. } => sounding.started(&event.port, channel, note),
        OutputAction::NoteOff { channel, note } | OutputAction::LegatoNoteOff { channel, note } => {
            sounding.ending(&event.port, channel, note, at);
        }
        OutputAction::TouchingNoteOff { channel, note, .. } => {
            // The output sends it earlier than `at` by the send lead. If
            // that would come before instant 0, `at` is kept: a later
            // instant only risks one extra, harmless Note Off at Stop.
            let lead: Duration = send_lead(event, anchors);
            let due: Instant = at.ns().checked_sub(lead.ns()).map_or(at, Instant::from_ns);
            sounding.ending(&event.port, channel, note, due);
        }
        OutputAction::ClockPulse
        | OutputAction::ControlChange { .. }
        | OutputAction::ProgramChange { .. }
        | OutputAction::Transport(_) => {}
    }
}

/// How late handing `event` over at `now` is, when it is: after the instant
/// the output must send it.
fn lateness(
    now: Instant,
    event: &ScheduledEvent,
    at: Instant,
    anchors: &[Anchor],
) -> Result<Option<Duration>, String> {
    let needed_by: Instant = now
        .checked_add(send_lead(event, anchors))
        .map_err(|error| error.to_string())?;
    Ok(needed_by
        .checked_sub(at)
        .ok()
        .filter(|late| *late > Duration::ZERO))
}

fn send<O: EventOutput>(
    output: &O,
    event: &ScheduledEvent,
    at: Instant,
    anchors: &[Anchor],
) -> Result<(), String> {
    let port: &str = event.port.as_str();
    match event.action {
        OutputAction::ClockPulse => output.clock_pulse(port, at),
        OutputAction::NoteOn {
            channel,
            note,
            velocity,
        } => output.note_on(
            port,
            at,
            channel.wire_value(),
            note.value(),
            velocity.value(),
        ),
        OutputAction::NoteOff { channel, note } | OutputAction::LegatoNoteOff { channel, note } => {
            output.note_off(port, at, channel.wire_value(), note.value())
        }
        OutputAction::TouchingNoteOff {
            channel,
            note,
            note_on_position,
            ..
        } if anchors
            .first()
            .is_some_and(|start: &Anchor| note_on_position < start.position()) =>
        {
            // The note started before the place playing started from, so
            // its note-on was never sent and there is nothing to shorten.
            output.note_off(port, at, channel.wire_value(), note.value())
        }
        OutputAction::TouchingNoteOff {
            channel,
            note,
            note_on_position,
            retrigger_gap,
        } => {
            let note_on_at: Instant = instant_in(anchors, note_on_position)?
                .checked_add(event.delay)
                .map_err(|error| error.to_string())?;
            output.note_off_before_note_on(
                port,
                note_on_at,
                at,
                retrigger_gap,
                channel.wire_value(),
                note.value(),
            )
        }
        OutputAction::ControlChange {
            channel,
            controller,
            value,
        } => output.control_change(
            port,
            at,
            channel.wire_value(),
            controller.value(),
            value.value(),
        ),
        OutputAction::ProgramChange { channel, program } => {
            output.program_change(port, at, channel.wire_value(), program.value())
        }
        OutputAction::Transport(TransportMessage::Start) => output.start(port, at),
        OutputAction::Transport(TransportMessage::Stop) => output.stop(port, at),
        OutputAction::Transport(TransportMessage::Continue) => output.continue_(port, at),
    }
}

/// The instant of `position`, from the last anchor at or before it.
fn instant_in(anchors: &[Anchor], position: ScorePosition) -> Result<Instant, String> {
    let anchor: &Anchor = anchors
        .iter()
        .rev()
        .find(|anchor: &&Anchor| anchor.position() <= position)
        .or_else(|| anchors.first())
        .ok_or_else(|| String::from("no anchor to count a position from"))?;
    anchor
        .instant_at(position)
        .map_err(|error| error.to_string())
}
