//! The sending thread (Architecture, Next, step 3): it owns the ALSA output,
//! opened once with the studio's ports so the musician connects them once,
//! and plays a schedule when asked, from a place in the song, once or in a
//! loop. Events are handed to the ALSA queue up to 50 ms ahead (D-8), their
//! instants computed from anchors (Musical Time and Clock, section 8). The
//! window reads where the song is from a shared counter of ticks, so
//! drawing never touches the output.
//!
//! The sending loop is `song_text/examples/support/event_player.rs`'s,
//! moved to its proper home and given Stop, a start place and looping.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration as StdDuration;

use alsa_output::error::AlsaOutputError;
use alsa_output::sequencer_output::{FlushOutcome, OutputPort, SequencerOutput};
use player::anchor::Anchor;
use player::schedule::{OutputAction, ScheduledEvent};
use player::song_plan::TempoChange;
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::midi_values::MidiChannel;
use vocabulary::phrase_cue::{OutputPortName, TransportMessage};
use vocabulary::score_position::ScorePosition;
use vocabulary::tempo::Tempo;

/// The player's lead window (D-8).
const LEAD: Duration = Duration::from_ms(50);
/// Time between the request and the first event, so the first events are
/// not already late when queued.
const START_DELAY: Duration = Duration::from_ms(200);
/// How long the loop sleeps between looks at the clock: well under the lead.
const SEND_LOOP_SLEEP: StdDuration = StdDuration::from_millis(5);
/// How long the thread waits for a request when idle.
const IDLE_WAIT: StdDuration = StdDuration::from_millis(50);
/// No position: the counter holds this while stopped.
const NO_POSITION: u64 = u64::MAX;

/// What to play.
pub struct PlayRequest {
    pub events: Vec<ScheduledEvent>,
    pub tempo: Tempo,
    pub tempo_changes: Vec<TempoChange>,
    /// Where to start, in ticks from the start of the song.
    pub start_ticks: u64,
    /// Where to stop (or loop back from); `None` plays to the end.
    pub end_ticks: Option<u64>,
    pub looping: bool,
    /// The clock and transport port, for Start and Stop.
    pub clock_port: OutputPortName,
    /// Each note track's port and channel, silenced when playing stops.
    pub note_outputs: Vec<(OutputPortName, MidiChannel)>,
}

enum Request {
    Play(Box<PlayRequest>),
    Stop,
    Quit,
}

/// The ports the thread opened, described for the musician, or why it
/// could not open them.
type PortsOpened = Result<Vec<String>, String>;

/// What the window reads.
struct Shared {
    position_ticks: AtomicU64,
    playing: AtomicBool,
    looping: AtomicBool,
}

pub struct Player {
    requests: Sender<Request>,
    problems: Receiver<String>,
    shared: Arc<Shared>,
    ports: Vec<String>,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    /// Opens the output ports in the sending thread.
    ///
    /// # Errors
    ///
    /// When the ALSA sequencer cannot be opened.
    pub fn start(port_names: Vec<String>) -> Result<Player, String> {
        let (requests, request_receiver): (Sender<Request>, Receiver<Request>) = mpsc::channel();
        let (problem_sender, problems): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (ready_sender, ready): (Sender<PortsOpened>, Receiver<PortsOpened>) = mpsc::channel();
        let shared: Arc<Shared> = Arc::new(Shared {
            position_ticks: AtomicU64::new(NO_POSITION),
            playing: AtomicBool::new(false),
            looping: AtomicBool::new(false),
        });
        let thread_shared: Arc<Shared> = Arc::clone(&shared);
        let thread: JoinHandle<()> = thread::Builder::new()
            .name(String::from("sending"))
            .spawn(move || {
                let names: Vec<&str> = port_names.iter().map(String::as_str).collect();
                let output: SequencerOutput = match SequencerOutput::open("Lead Sheet", &names) {
                    Ok((output, warnings)) => {
                        for warning in &warnings {
                            // The window shows these in its message line.
                            let _ = problem_sender.send(warning.to_string());
                        }
                        output
                    }
                    Err(error) => {
                        let _ =
                            ready_sender.send(Err(format!("Cannot open the MIDI output: {error}")));
                        return;
                    }
                };
                let described: Vec<String> = output
                    .ports()
                    .iter()
                    .map(|port: &OutputPort| {
                        format!("{} {}:{}", port.name(), output.client_id(), port.port_id())
                    })
                    .collect();
                let _ = ready_sender.send(Ok(described));
                serve(&output, &request_receiver, &problem_sender, &thread_shared);
            })
            .map_err(|error| format!("Cannot start the sending thread: {error}"))?;
        let ports: Vec<String> = ready.recv().map_err(|_| {
            String::from("The sending thread stopped while opening the MIDI output.")
        })??;
        Ok(Player {
            requests,
            problems,
            shared,
            ports,
            thread: Some(thread),
        })
    }

    /// The ports, as `Name client:port`, for the musician to connect.
    #[must_use]
    pub fn ports(&self) -> &[String] {
        &self.ports
    }

    /// Plays `request`, stopping whatever was playing.
    ///
    /// # Errors
    ///
    /// When the sending thread has stopped.
    pub fn play(&self, request: PlayRequest) -> Result<(), String> {
        self.requests
            .send(Request::Play(Box::new(request)))
            .map_err(|_| String::from("The sending thread has stopped; restart the program."))
    }

    /// Stops playing, silencing every note.
    ///
    /// # Errors
    ///
    /// When the sending thread has stopped.
    pub fn stop(&self) -> Result<(), String> {
        self.requests
            .send(Request::Stop)
            .map_err(|_| String::from("The sending thread has stopped; restart the program."))
    }

    #[must_use]
    pub fn playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn looping(&self) -> bool {
        self.shared.looping.load(Ordering::Relaxed)
    }

    /// Where the song is now, in ticks from its start.
    #[must_use]
    pub fn position_ticks(&self) -> Option<u64> {
        let ticks: u64 = self.shared.position_ticks.load(Ordering::Relaxed);
        (ticks != NO_POSITION).then_some(ticks)
    }

    /// A problem the thread met, if any is waiting.
    #[must_use]
    pub fn next_problem(&self) -> Option<String> {
        self.problems.try_recv().ok()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        // The thread silences the notes as it ends; wait for that, so no
        // note is left sounding when the program closes.
        let _ = self.requests.send(Request::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Waits for requests and carries them out until Quit.
fn serve(
    output: &SequencerOutput,
    requests: &Receiver<Request>,
    problems: &Sender<String>,
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
                let outcome: Result<Option<Request>, String> =
                    play_until_interrupted(output, &play, requests, shared);
                shared.playing.store(false, Ordering::Relaxed);
                shared.looping.store(false, Ordering::Relaxed);
                shared.position_ticks.store(NO_POSITION, Ordering::Relaxed);
                if let Err(problem) = silence(output, &play) {
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

/// Removes what is queued, and sends Stop and All Notes Off at once.
fn silence(output: &SequencerOutput, play: &PlayRequest) -> Result<(), String> {
    output
        .remove_queued_events()
        .map_err(|error| format!("Stopping: {error}"))?;
    if let Ok(port) = find_port(output, play.clock_port.as_str()) {
        output
            .send_stop_now(port)
            .map_err(|error| format!("Sending Stop: {error}"))?;
    }
    for (port_name, channel) in &play.note_outputs {
        let port: &OutputPort = find_port(output, port_name.as_str())?;
        output
            .send_all_notes_off_now(port, channel.wire_value())
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
fn play_until_interrupted(
    output: &SequencerOutput,
    play: &PlayRequest,
    requests: &Receiver<Request>,
    shared: &Shared,
) -> Result<Option<Request>, String> {
    let start: ScorePosition = ScorePosition::from_ticks(play.start_ticks);
    let end: Option<ScorePosition> = play.end_ticks.map(ScorePosition::from_ticks);
    let first_event: usize = play
        .events
        .partition_point(|event: &ScheduledEvent| event.position < start);
    let first_instant: Instant = output
        .now()
        .map_err(|error| error.to_string())?
        .checked_add(START_DELAY)
        .map_err(|error| error.to_string())?;
    let mut anchors: Vec<Anchor> = vec![Anchor::new(start, first_instant, tempo_at(play, start))];
    let mut next_change: usize = play
        .tempo_changes
        .partition_point(|change: &TempoChange| change.position <= start);
    let mut next: usize = first_event;
    let mut pool_full: bool = false;
    // Followers start with the song: Start before the first clock pulse
    // when playing from anywhere but the start (the schedule has its own
    // Start at the start).
    if play.start_ticks > 0 {
        let port: &OutputPort = find_port(output, play.clock_port.as_str())?;
        output
            .queue_start(port, first_instant)
            .map_err(|error| error.to_string())?;
    }
    loop {
        match requests.try_recv() {
            Ok(Request::Stop) => return Ok(None),
            Ok(request) => return Ok(Some(request)),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => return Ok(Some(Request::Quit)),
        }
        let now: Instant = output.now().map_err(|error| error.to_string())?;
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
            send(output, event, at, &anchors)?;
            next += 1;
        }
        match output.flush().map_err(|error| error.to_string())? {
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
            } else if now >= end_instant {
                return Ok(None);
            }
        }
        thread::sleep(SEND_LOOP_SLEEP);
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

fn send(
    output: &SequencerOutput,
    event: &ScheduledEvent,
    at: Instant,
    anchors: &[Anchor],
) -> Result<(), String> {
    let port: &OutputPort = find_port(output, event.port.as_str())?;
    let result: Result<(), AlsaOutputError> = match event.action {
        OutputAction::ClockPulse => output.queue_clock_pulse(port, at, 0),
        OutputAction::NoteOn {
            channel,
            note,
            velocity,
        } => output.queue_note_on(
            port,
            at,
            channel.wire_value(),
            note.value(),
            velocity.value(),
        ),
        OutputAction::NoteOff { channel, note } | OutputAction::LegatoNoteOff { channel, note } => {
            output.queue_note_off(port, at, channel.wire_value(), note.value())
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
            output.queue_note_off(port, at, channel.wire_value(), note.value())
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
            output.queue_note_off_before_note_on(
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
        } => output.queue_control_change(
            port,
            at,
            channel.wire_value(),
            controller.value(),
            value.value(),
        ),
        OutputAction::ProgramChange { channel, program } => {
            output.queue_program_change(port, at, channel.wire_value(), program.value())
        }
        OutputAction::Transport(TransportMessage::Start) => output.queue_start(port, at),
        OutputAction::Transport(TransportMessage::Stop) => output.queue_stop(port, at),
        OutputAction::Transport(TransportMessage::Continue) => output.queue_continue(port, at),
    };
    result.map_err(|error| error.to_string())
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

fn find_port<'a>(output: &'a SequencerOutput, name: &str) -> Result<&'a OutputPort, String> {
    output
        .ports()
        .iter()
        .find(|port: &&OutputPort| port.name() == name)
        .ok_or_else(|| format!("No output port named {name:?}."))
}
