//! Sends a schedule to the ALSA sequencer in real time, for the listening
//! tests (`rhythm_notation_test`, `song_play_test`).
//!
//! This is the sending loop of `alsa_output/examples/control_track_test`,
//! trimmed, shared by the examples so there is one copy to retire. Its
//! proper home is the app's sending thread (Architecture, Next, step 3).
//! Examples include it with `#[path = "../support/event_player.rs"]`.

use std::io::BufRead;
use std::thread;
use std::time::Duration as StdDuration;

use alsa_output::error::AlsaOutputError;
use alsa_output::sequencer_output::{FlushOutcome, OutputPort, SequencerOutput};
use alsa_output::warning::OutputWarning;
use player::anchor::Anchor;
use player::schedule::{OutputAction, ScheduledEvent};
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::midi_values::MidiChannel;
use vocabulary::phrase_cue::TransportMessage;
use vocabulary::score_position::ScorePosition;
use vocabulary::tempo::Tempo;

/// The player's lead window (D-8).
const LEAD: Duration = Duration::from_ms(50);
/// Time between Enter and the first event, so the first events are not
/// already late when queued.
const START_DELAY: Duration = Duration::from_ms(200);
/// How long the sending loop sleeps between looks at the clock: well under
/// the 50 ms lead.
const SEND_LOOP_SLEEP: StdDuration = StdDuration::from_millis(5);

/// Opens the sequencer with the named output ports and shows any warnings.
///
/// # Errors
///
/// When the sequencer cannot be opened.
pub fn open_output(port_names: &[&str]) -> Result<SequencerOutput, String> {
    let (output, warnings): (SequencerOutput, Vec<OutputWarning>) =
        SequencerOutput::open("Lead Sheet", port_names).map_err(|e| e.to_string())?;
    for warning in &warnings {
        eprintln!("WARNING: {warning}");
    }
    Ok(output)
}

/// The ports to connect.
pub fn print_ports(output: &SequencerOutput) {
    println!("\nPorts:");
    for port in output.ports() {
        println!(
            "  {:<11} {}:{}",
            port.name(),
            output.client_id(),
            port.port_id()
        );
    }
}

/// Waits for the musician to connect the ports and press Enter.
///
/// # Errors
///
/// When the terminal cannot be read.
pub fn wait_for_enter() -> Result<(), String> {
    println!("Connect the ports now (MIDI router or aconnect), then press Enter to play.");
    let mut line: String = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("Reading Enter from the terminal: {e}"))?;
    Ok(())
}

/// Plays `events` at `tempo`, starting `START_DELAY` from now, calling
/// `announce` for each event as it is queued, and returns once the last
/// event has been delivered.
///
/// # Errors
///
/// When the output fails; the message says what was being done.
pub fn play(
    output: &SequencerOutput,
    events: &[ScheduledEvent],
    tempo: Tempo,
    announce: &mut dyn FnMut(&ScheduledEvent),
) -> Result<(), String> {
    let first_instant: Instant = output
        .now()
        .map_err(|e| e.to_string())?
        .checked_add(START_DELAY)
        .map_err(|e| e.to_string())?;
    let anchor: Anchor = Anchor::new(ScorePosition::ZERO, first_instant, tempo);
    let last_instant: Instant = send_all(output, events, anchor, announce)?;
    // Let the last events be delivered before the queue closes with the
    // program.
    wait_until(
        output,
        last_instant.checked_add(LEAD).map_err(|e| e.to_string())?,
    )
}

/// All Notes Off on one channel of one port: the safety net after playing.
///
/// # Errors
///
/// When the port is unknown or the output refuses the message.
pub fn silence(
    output: &SequencerOutput,
    port_name: &str,
    channel: MidiChannel,
) -> Result<(), String> {
    let port: &OutputPort = find_port(output, port_name)?;
    output
        .send_all_notes_off_now(port, channel.wire_value())
        .map_err(|e| e.to_string())
}

/// Hands every event to the queue up to `LEAD` ahead of its instant, each
/// instant computed from `anchor`. Returns the last event's instant.
fn send_all(
    output: &SequencerOutput,
    events: &[ScheduledEvent],
    anchor: Anchor,
    announce: &mut dyn FnMut(&ScheduledEvent),
) -> Result<Instant, String> {
    let mut late_events: u64 = 0;
    let mut last_instant: Instant = anchor.instant();
    let mut next: usize = 0;
    // True while the kernel's output pool is full: queue nothing new until
    // a flush hands everything over.
    let mut pool_full: bool = false;
    loop {
        if next >= events.len() && !pool_full {
            break;
        }
        let now: Instant = output.now().map_err(|e| e.to_string())?;
        let horizon: Instant = now.checked_add(LEAD).map_err(|e| e.to_string())?;
        while let Some(event) = events.get(next) {
            if pool_full {
                break;
            }
            let at: Instant = anchor
                .instant_at(event.position)
                .map_err(|e| e.to_string())?;
            if at > horizon {
                break;
            }
            if at < now {
                late_events += 1;
            }
            send(output, event, at, anchor)?;
            announce(event);
            last_instant = at;
            next += 1;
        }
        match output.flush().map_err(|e| e.to_string())? {
            FlushOutcome::AllHandedOver => pool_full = false,
            FlushOutcome::PoolFull => pool_full = true,
        }
        thread::sleep(SEND_LOOP_SLEEP);
    }
    if late_events > 0 {
        eprintln!("WARNING: {late_events} events were queued after their instant.");
    }
    Ok(last_instant)
}

fn send(
    output: &SequencerOutput,
    event: &ScheduledEvent,
    at: Instant,
    anchor: Anchor,
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
            retrigger_gap,
        } => {
            let note_on_at: Instant = anchor
                .instant_at(note_on_position)
                .map_err(|e| e.to_string())?;
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
    result.map_err(|e| e.to_string())
}

fn find_port<'a>(output: &'a SequencerOutput, name: &str) -> Result<&'a OutputPort, String> {
    output
        .ports()
        .iter()
        .find(|port: &&OutputPort| port.name() == name)
        .ok_or_else(|| format!("No output port named {name:?}."))
}

fn wait_until(output: &SequencerOutput, deadline: Instant) -> Result<(), String> {
    loop {
        let now: Instant = output.now().map_err(|e| e.to_string())?;
        // Reached or passed: done. Passing is harmless here.
        let Ok(remaining): Result<Duration, _> = deadline.checked_sub(now) else {
            return Ok(());
        };
        if remaining == Duration::ZERO {
            return Ok(());
        }
        thread::sleep(StdDuration::from_nanos(remaining.ns()));
    }
}
