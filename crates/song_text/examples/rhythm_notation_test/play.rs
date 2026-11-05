//! The play half of the test: a chosen rhythm on the beat-pulse port, with
//! Timing Clock on its own port, for listening on the FH-2.
//!
//! This is the test's own copy of the sending loop of
//! `alsa_output/examples/control_track_test`, trimmed. Its proper home is
//! the app's sending thread (Architecture, Next, step 3).

use std::io::BufRead;
use std::thread;
use std::time::Duration as StdDuration;

use alsa_output::error::AlsaOutputError;
use alsa_output::sequencer_output::{FlushOutcome, OutputPort, SequencerOutput};
use alsa_output::warning::OutputWarning;
use player::anchor::Anchor;
use player::schedule::{EventSource, OutputAction, ScheduledEvent, schedule};
use song_text::length_words::describe_place;
use song_text::rhythm_library::RhythmLibraryEntry;
use vocabulary::control_track::{BeatPulse, ControlTrack};
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::midi_values::{MidiChannel, NoteNumber, Velocity};
use vocabulary::phrase_cue::{ControlCueMessage, OutputPortName, PhraseCue, TransportMessage};
use vocabulary::score_length::ScoreLength;
use vocabulary::score_position::ScorePosition;
use vocabulary::tempo::Tempo;
use vocabulary::time_signature::TimeSignature;

const CLOCK_PORT: &str = "Clock";
const PULSE_PORT: &str = "Beat pulse";

/// The player's lead window (D-8).
const LEAD: Duration = Duration::from_ms(50);
/// Time between Enter and the first note, so the first events are not
/// already late when queued.
const START_DELAY: Duration = Duration::from_ms(200);
/// How long the sending loop sleeps between looks at the clock: well under
/// the 50 ms lead.
const SEND_LOOP_SLEEP: StdDuration = StdDuration::from_millis(5);

/// A 16th: 5040 ticks/q ÷ 4. Each pulse is held this long, or for the
/// note's own length if that is shorter.
const PULSE_LENGTH: ScoreLength = ScoreLength::from_ticks(1260);

const ACCENT_VELOCITY: u8 = 127;
const NORMAL_VELOCITY: u8 = 64;

/// What to play, from the command line.
pub struct PlayRequest {
    rhythm_name: String,
    quarter_notes_per_minute: u64,
    bars: u64,
    channel: MidiChannel,
    note: NoteNumber,
    /// Start at the first note and Stop at the end, on the clock port.
    transport: bool,
}

/// The library file and, when `--play` is given, what to play.
///
/// # Errors
///
/// A message saying which option is wrong and what to type instead.
pub fn parse_arguments(
    arguments: &[String],
    default_library_file: &str,
) -> Result<(String, Option<PlayRequest>), String> {
    let mut library_file: String = default_library_file.to_string();
    let mut rhythm_name: Option<String> = None;
    let mut quarter_notes_per_minute: u64 = 120;
    let mut bars: u64 = 8;
    let mut channel: u8 = 15;
    let mut note: u8 = 60;
    let mut transport: bool = false;
    let mut index: usize = 0;
    while index < arguments.len() {
        let option: &str = arguments[index].as_str();
        if !option.starts_with("--") {
            library_file = option.to_string();
            index += 1;
            continue;
        }
        // The one option without a value.
        if option == "--transport" {
            transport = true;
            index += 1;
            continue;
        }
        let Some(value): Option<&String> = arguments.get(index + 1) else {
            return Err(format!("{option} needs a value, e.g. {option} 120."));
        };
        let number = || -> Result<u64, String> {
            value
                .parse::<u64>()
                .map_err(|_| format!("{option} {value}: not a whole number."))
        };
        let byte = || -> Result<u8, String> {
            value
                .parse::<u8>()
                .map_err(|_| format!("{option} {value}: use 0 to 127."))
        };
        match option {
            // Spaces at either end come from typing or tab completion.
            "--play" => rhythm_name = Some(value.trim().to_string()),
            "--bpm" => quarter_notes_per_minute = number()?,
            "--bars" => bars = number()?,
            "--channel" => channel = byte()?,
            "--note" => note = byte()?,
            _ => {
                return Err(format!(
                    "Unknown option {option}. Options: --play \"Name\" --bpm --bars \
                     --channel --note --transport"
                ));
            }
        }
        index += 2;
    }
    if bars == 0 {
        return Err(String::from("--bars 0: use 1 or more."));
    }
    let request: Option<PlayRequest> = match rhythm_name {
        None => None,
        Some(rhythm_name) => Some(PlayRequest {
            rhythm_name,
            quarter_notes_per_minute,
            bars,
            channel: MidiChannel::new(channel).map_err(|e| format!("--channel: {e}"))?,
            note: NoteNumber::new(note).map_err(|e| format!("--note: {e}"))?,
            transport,
        }),
    };
    Ok((library_file, request))
}

/// Plays the rhythm, repeated to fill `request.bars` bars.
///
/// # Errors
///
/// When the rhythm is not in the library, or the output or the schedule
/// fails; the message says what was being done.
pub fn play_rhythm(entries: &[RhythmLibraryEntry], request: &PlayRequest) -> Result<(), String> {
    let Some(entry) = entries
        .iter()
        .find(|entry: &&RhythmLibraryEntry| entry.pattern().name() == request.rhythm_name)
    else {
        let names: Vec<&str> = entries
            .iter()
            .map(|entry: &RhythmLibraryEntry| entry.pattern().name())
            .collect();
        return Err(format!(
            "No rhythm named {:?} in the library. Choose one of: {}.",
            request.rhythm_name,
            names.join(", ")
        ));
    };
    let pattern = entry.pattern();
    let time_signature: TimeSignature = pattern.time_signature();
    let stretch_length: ScoreLength = time_signature
        .bar_length()
        .checked_mul(request.bars)
        .map_err(|e| e.to_string())?;

    let velocity = |value: u8| Velocity::new(value).map_err(|e| e.to_string());
    let pulse: BeatPulse = BeatPulse::new(
        port_name(PULSE_PORT)?,
        request.channel,
        request.note,
        velocity(ACCENT_VELOCITY)?,
        velocity(NORMAL_VELOCITY)?,
        PULSE_LENGTH,
    )
    .map_err(|e| e.to_string())?
    .with_rhythm(pattern.clone());
    let cues: Vec<PhraseCue> = if request.transport {
        transport_cues(stretch_length)?
    } else {
        Vec::new()
    };
    let track: ControlTrack =
        ControlTrack::new("Rhythm test", Some(pulse), cues).map_err(|e| e.to_string())?;
    let events: Vec<ScheduledEvent> = schedule(
        &port_name(CLOCK_PORT)?,
        &[track],
        time_signature,
        stretch_length,
    )
    .map_err(|e| e.to_string())?;

    let (output, warnings): (SequencerOutput, Vec<OutputWarning>) =
        SequencerOutput::open("Lead Sheet", &[CLOCK_PORT, PULSE_PORT])
            .map_err(|e| e.to_string())?;
    for warning in &warnings {
        eprintln!("WARNING: {warning}");
    }
    print_plan(&output, request, pattern.name(), time_signature);
    println!("Connect the ports now (MIDI router or aconnect), then press Enter to play.");
    wait_for_enter()?;

    let tempo: Tempo = Tempo::from_quarter_notes_per_minute(request.quarter_notes_per_minute)
        .map_err(|e| e.to_string())?;
    let first_instant: Instant = output
        .now()
        .map_err(|e| e.to_string())?
        .checked_add(START_DELAY)
        .map_err(|e| e.to_string())?;
    let anchor: Anchor = Anchor::new(ScorePosition::ZERO, first_instant, tempo);
    let last_instant: Instant = send_all(&output, &events, anchor, time_signature)?;

    // Let the last events be delivered before the queue closes with the
    // program, then silence the channel as a safety net.
    wait_until(
        &output,
        last_instant.checked_add(LEAD).map_err(|e| e.to_string())?,
    )?;
    let pulse_port: &OutputPort = find_port(&output, PULSE_PORT)?;
    output
        .send_all_notes_off_now(pulse_port, request.channel.wire_value())
        .map_err(|e| e.to_string())?;
    println!("Done.");
    Ok(())
}

/// The ports to connect, and what is about to play.
fn print_plan(
    output: &SequencerOutput,
    request: &PlayRequest,
    rhythm_name: &str,
    time_signature: TimeSignature,
) {
    println!("\nPorts:");
    for port in output.ports() {
        println!(
            "  {:<11} {}:{}",
            port.name(),
            output.client_id(),
            port.port_id()
        );
    }
    println!(
        "Playing {:?} ({}/{}) for {} bar{} at {} q/min on {PULSE_PORT:?}, channel {}, note {}. \
         Timing Clock{} on {CLOCK_PORT:?}.",
        rhythm_name,
        time_signature.counts_per_bar(),
        time_signature.counts_per_whole_note(),
        request.bars,
        if request.bars == 1 { "" } else { "s" },
        request.quarter_notes_per_minute,
        request.channel.number(),
        request.note.value(),
        if request.transport {
            " and Start/Stop"
        } else {
            ""
        }
    );
}

/// Start at the first note and Stop at the end, on the clock port. The FH-2's
/// clock output pulses only between a Start and a Stop, so it can be watched
/// only with these.
fn transport_cues(stretch_length: ScoreLength) -> Result<Vec<PhraseCue>, String> {
    let end: ScorePosition = ScorePosition::ZERO
        .checked_add(stretch_length)
        .map_err(|e| e.to_string())?;
    let mut cues: Vec<PhraseCue> = Vec::new();
    for (name, position, message) in [
        ("Start", ScorePosition::ZERO, TransportMessage::Start),
        ("Stop", end, TransportMessage::Stop),
    ] {
        cues.push(
            PhraseCue::new(
                name,
                position,
                port_name(CLOCK_PORT)?,
                ControlCueMessage::Transport(message),
            )
            .map_err(|e| e.to_string())?,
        );
    }
    Ok(cues)
}

/// Hands every event to the queue up to `LEAD` ahead of its instant, each
/// instant computed from `anchor`. Returns the last event's instant.
fn send_all(
    output: &SequencerOutput,
    events: &[ScheduledEvent],
    anchor: Anchor,
    time_signature: TimeSignature,
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
            send(output, event, at)?;
            if let (EventSource::BeatPulse { .. }, OutputAction::NoteOn { velocity, .. }) =
                (&event.source, event.action)
            {
                let accent: &str = if velocity.value() == ACCENT_VELOCITY {
                    "  (accent)"
                } else {
                    ""
                };
                println!(
                    "{}{accent}",
                    describe_place(event.position.ticks(), time_signature)
                );
            }
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

fn send(output: &SequencerOutput, event: &ScheduledEvent, at: Instant) -> Result<(), String> {
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
        OutputAction::NoteOff { channel, note } => {
            output.queue_note_off(port, at, channel.wire_value(), note.value())
        }
        OutputAction::Transport(TransportMessage::Start) => output.queue_start(port, at),
        OutputAction::Transport(TransportMessage::Stop) => output.queue_stop(port, at),
        // This test's schedule has only Timing Clock, notes, Start and Stop.
        other => {
            return Err(format!(
                "The rhythm test's schedule holds {other:?}, which it does not send."
            ));
        }
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

fn port_name(name: &str) -> Result<OutputPortName, String> {
    OutputPortName::new(name).map_err(|e| e.to_string())
}

fn wait_for_enter() -> Result<(), String> {
    let mut line: String = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| format!("Reading Enter from the terminal: {e}"))?;
    Ok(())
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
