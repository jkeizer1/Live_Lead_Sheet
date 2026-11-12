//! Plays a hard-coded stretch of song through three output ports, to try
//! D-10 on the studio: a count-in carried by the beat pulse, then the
//! music's first bar, where a reset trigger and a Start cue fire.
//!
//! `cargo run --release -p alsa_output --example control_track_test -- [options]`
//!
//! What the test proves, its findings and how to run it are in
//! `explainer.md` beside this file.
//!
//! | Option | Default | Meaning |
//! |---|---|---|
//! | `--bpm N` | 120 | tempo, quarter notes per minute (4/4) |
//! | `--count-in-bars N` | 1 | count-in bars before the music, 0 to 2 |
//! | `--music-bars N` | 8 | bars after the count-in |
//! | `--pulse-channel C` | 15 | beat pulse channel, 1 to 16 |
//! | `--pulse-note N` | 60 | beat pulse note number |
//! | `--reset-channel C` | 16 | reset trigger channel, 1 to 16 |
//! | `--reset-note N` | 60 | reset trigger note number |
//! | `--transport-on-clock` | off | send the SQ-1's Start and Stop on `Clock` instead of `To SQ-1` |
//! | `--restart-every-sixteenths N` | none | from the music's first bar, every N sixteenths, Stop then Start the SQ-1, so it goes back to step 1 |
//!
//! The defaults are harmless: they only choose what this test sends.
//!
//! Ports (connect them yourself, with the MIDI router or `aconnect`, when
//! the program asks):
//!
//! - `Clock`: Timing Clock (F8) from the first count-in bar to the end.
//! - `Beat pulse`: a note on every beat (beat 1 at velocity 127, the
//!   others 64, each a 16th long), and a reset trigger note (a 32nd) on the
//!   music's first bar.
//! - `To SQ-1`: Start on the music's first bar, Stop at the end. With
//!   `--transport-on-clock` these go on `Clock` instead, for a MIDI router
//!   that passes clock and transport only from its clock master; its routes
//!   then decide which devices receive them.
//!
//! The client is named `Lead Sheet`, so the MIDI router suggests ids
//! such as `lead-sheet-clock`.
//!
//! Ctrl-C stops the program at once: no Stop is sent, so the SQ-1 keeps
//! running. Press Stop on the SQ-1.

use std::io::BufRead;
use std::process::ExitCode;
use std::thread;
use std::time::Duration as StdDuration;

use alsa_output::error::AlsaOutputError;
use alsa_output::sequencer_output::{FlushOutcome, OutputPort, SequencerOutput};
use alsa_output::warning::OutputWarning;
use player::anchor::Anchor;
use player::counter::{Counter, CounterSection};
use player::note_track::{NoteTrack, TimedNote};
use player::schedule::{EventSource, OutputAction, ScheduledEvent, schedule};
use vocabulary::control_track::ControlTrack;
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::midi_values::{MidiChannel, NoteNumber, Velocity};
use vocabulary::phrase_cue::{
    ChannelMessage, ControlCueMessage, OutputPortName, PhraseCue, TransportMessage,
};
use vocabulary::score_length::ScoreLength;
use vocabulary::score_position::ScorePosition;
use vocabulary::tempo::Tempo;
use vocabulary::time_signature::TimeSignature;

const CLOCK_PORT: &str = "Clock";
const PULSE_PORT: &str = "Beat pulse";
const SQ1_PORT: &str = "To SQ-1";

/// The player's lead window (D-8).
const LEAD: Duration = Duration::from_ms(50);
/// Time between Enter and the first beat, so the first events are not
/// already late when queued.
const START_DELAY: Duration = Duration::from_ms(200);
/// How long the sending loop sleeps between looks at the clock: well under
/// the 50 ms lead.
const SEND_LOOP_SLEEP: StdDuration = StdDuration::from_millis(5);

/// A 16th: 5040 ticks/q ÷ 4.
const SIXTEENTH_NOTE: ScoreLength = ScoreLength::from_ticks(1260);
/// Each beat pulse is held for a 16th.
const BEAT_PULSE_LENGTH: ScoreLength = SIXTEENTH_NOTE;
/// A 32nd: 5040 ticks/q ÷ 8.
const RESET_TRIGGER_LENGTH: ScoreLength = ScoreLength::from_ticks(630);

const ACCENT_VELOCITY: u8 = 127;
const NORMAL_VELOCITY: u8 = 64;
const RESET_VELOCITY: u8 = 127;

struct Settings {
    quarter_notes_per_minute: u64,
    count_in_bars: u64,
    music_bars: u64,
    pulse_channel: MidiChannel,
    pulse_note: NoteNumber,
    reset_channel: MidiChannel,
    reset_note: NoteNumber,
    /// Start and Stop on the clock port instead of their own.
    transport_on_clock: bool,
    /// Restart the SQ-1 every this many sixteenths; `None`: never.
    restart_every_sixteenths: Option<u64>,
}

/// A note this program has sent a note on for and not yet a note off.
struct SoundingNote {
    port_name: OutputPortName,
    channel: MidiChannel,
    note: NoteNumber,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let settings: Settings = parse_settings()?;
    let time_signature: TimeSignature = TimeSignature::new(4, 4).map_err(|e| e.to_string())?;
    let bar_length: ScoreLength = time_signature.bar_length();
    let music_start: ScorePosition = ScorePosition::ZERO
        .checked_add(
            bar_length
                .checked_mul(settings.count_in_bars)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    let stretch_length: ScoreLength = bar_length
        .checked_mul(settings.count_in_bars + settings.music_bars)
        .map_err(|e| e.to_string())?;

    let track: ControlTrack = studio_track(&settings, music_start, stretch_length)?;
    let beat: NoteTrack = beat_track(&settings, time_signature, stretch_length)?;
    let clock_port: OutputPortName = port_name(CLOCK_PORT)?;
    let events: Vec<ScheduledEvent> =
        schedule(&clock_port, &[beat], &[track], stretch_length).map_err(|e| e.to_string())?;

    let (output, warnings): (SequencerOutput, Vec<OutputWarning>) =
        SequencerOutput::open("Lead Sheet", &[CLOCK_PORT, PULSE_PORT, SQ1_PORT])
            .map_err(|e| e.to_string())?;
    for warning in &warnings {
        eprintln!("WARNING: {warning}");
    }
    println!("Ports:");
    for port in output.ports() {
        println!(
            "  {:<11} {}:{}",
            port.name(),
            output.client_id(),
            port.port_id()
        );
    }
    print_settings(&settings);
    println!("Connect the ports now (MIDI router or aconnect), then press Enter to play.");
    wait_for_enter()?;

    let tempo: Tempo = Tempo::from_quarter_notes_per_minute(settings.quarter_notes_per_minute)
        .map_err(|e| e.to_string())?;
    let first_instant: Instant = output
        .now()
        .map_err(|e| e.to_string())?
        .checked_add(START_DELAY)
        .map_err(|e| e.to_string())?;
    let anchor: Anchor = Anchor::new(ScorePosition::ZERO, first_instant, tempo);
    let end: ScorePosition = ScorePosition::ZERO
        .checked_add(stretch_length)
        .map_err(|e| e.to_string())?;
    let last_instant: Instant = play(&output, &events, anchor, &settings, time_signature, end)?;

    // Let the last events be delivered before the queue closes with the
    // program.
    wait_until(
        &output,
        last_instant.checked_add(LEAD).map_err(|e| e.to_string())?,
    )?;
    // Final safety net: All Notes Off on the channels this test used.
    let pulse_port: &OutputPort = find_port(&output, &port_name(PULSE_PORT)?)?;
    for channel in [settings.pulse_channel, settings.reset_channel] {
        output
            .send_all_notes_off_now(pulse_port, channel.wire_value())
            .map_err(|e| e.to_string())?;
    }
    println!("Done.");
    Ok(())
}

/// The beat: a mono voice playing one fixed note on every beat, beat 1 of
/// each bar accented, each note held for `BEAT_PULSE_LENGTH`.
fn beat_track(
    settings: &Settings,
    time_signature: TimeSignature,
    stretch_length: ScoreLength,
) -> Result<NoteTrack, String> {
    let velocity = |value: u8| Velocity::new(value).map_err(|e| e.to_string());
    let beat_length_ticks: u64 = time_signature.beat_length().ticks();
    let bar_length_ticks: u64 = time_signature.bar_length().ticks();
    let mut notes: Vec<TimedNote> = Vec::new();
    let mut start_ticks: u64 = 0;
    while start_ticks < stretch_length.ticks() {
        notes.push(TimedNote {
            start_ticks,
            length_ticks: BEAT_PULSE_LENGTH.ticks(),
            note: settings.pulse_note,
            velocity: if start_ticks.is_multiple_of(bar_length_ticks) {
                velocity(ACCENT_VELOCITY)?
            } else {
                velocity(NORMAL_VELOCITY)?
            },
            legato_to_next: false,
        });
        start_ticks += beat_length_ticks;
    }
    NoteTrack::new(
        "Beat",
        port_name(PULSE_PORT)?,
        settings.pulse_channel,
        notes,
    )
    .map_err(|e| e.to_string())
}

/// The control track: a reset trigger and Start on the music's first bar,
/// Stop at the end.
fn studio_track(
    settings: &Settings,
    music_start: ScorePosition,
    stretch_length: ScoreLength,
) -> Result<ControlTrack, String> {
    let velocity = |value: u8| Velocity::new(value).map_err(|e| e.to_string());
    let reset: PhraseCue = PhraseCue::new(
        "Reset",
        music_start,
        port_name(PULSE_PORT)?,
        ControlCueMessage::Channel {
            channel: settings.reset_channel,
            message: ChannelMessage::Note {
                note: settings.reset_note,
                velocity: velocity(RESET_VELOCITY)?,
                length: RESET_TRIGGER_LENGTH,
            },
        },
    )
    .map_err(|e| e.to_string())?;
    let transport_port: OutputPortName = if settings.transport_on_clock {
        port_name(CLOCK_PORT)?
    } else {
        port_name(SQ1_PORT)?
    };
    let start: PhraseCue = PhraseCue::new(
        "Start SQ-1",
        music_start,
        transport_port.clone(),
        ControlCueMessage::Transport(TransportMessage::Start),
    )
    .map_err(|e| e.to_string())?;
    let end: ScorePosition = ScorePosition::ZERO
        .checked_add(stretch_length)
        .map_err(|e| e.to_string())?;
    let stop: PhraseCue = PhraseCue::new(
        "Stop SQ-1",
        end,
        transport_port.clone(),
        ControlCueMessage::Transport(TransportMessage::Stop),
    )
    .map_err(|e| e.to_string())?;
    let mut cues: Vec<PhraseCue> = vec![reset, start];
    if let Some(sixteenths) = settings.restart_every_sixteenths {
        cues.extend(restart_cues(&transport_port, music_start, end, sixteenths)?);
    }
    cues.push(stop);
    ControlTrack::new("Studio", cues).map_err(|e| e.to_string())
}

/// Stop then Start at every `sixteenths` sixteenths after `music_start`,
/// before `end`. The SQ-1 goes back to step 1 on Stop then Start (SQ-1
/// Observations, Linux MIDI Router). Both cues share a position; the
/// schedule keeps them in the written order, Stop first, and puts both
/// before that position's Timing Clock.
fn restart_cues(
    transport_port: &OutputPortName,
    music_start: ScorePosition,
    end: ScorePosition,
    sixteenths: u64,
) -> Result<Vec<PhraseCue>, String> {
    // n sixteenths × 1260 ticks/sixteenth → ticks.
    let interval: ScoreLength = SIXTEENTH_NOTE
        .checked_mul(sixteenths)
        .map_err(|e| e.to_string())?;
    let mut cues: Vec<PhraseCue> = Vec::new();
    let mut at: ScorePosition = music_start
        .checked_add(interval)
        .map_err(|e| e.to_string())?;
    let mut restart_number: u64 = 1;
    while at < end {
        for (name, message) in [
            ("Stop", TransportMessage::Stop),
            ("Start", TransportMessage::Start),
        ] {
            cues.push(
                PhraseCue::new(
                    &format!("Restart {restart_number}: {name} SQ-1"),
                    at,
                    transport_port.clone(),
                    ControlCueMessage::Transport(message),
                )
                .map_err(|e| e.to_string())?,
            );
        }
        at = at.checked_add(interval).map_err(|e| e.to_string())?;
        restart_number += 1;
    }
    Ok(cues)
}

/// Hands every event to the queue up to `LEAD` ahead of its instant, each
/// instant computed from `anchor`. Returns the last event's instant.
fn play(
    output: &SequencerOutput,
    events: &[ScheduledEvent],
    anchor: Anchor,
    settings: &Settings,
    time_signature: TimeSignature,
    end: ScorePosition,
) -> Result<Instant, String> {
    let mut sounding: Vec<SoundingNote> = Vec::new();
    let mut late_events: u64 = 0;
    let mut last_instant: Instant = anchor.instant();
    let mut next: usize = 0;
    // True while the kernel's output pool is full: queue nothing new until
    // a flush hands everything over (FlushOutcome::PoolFull).
    let mut pool_full: bool = false;
    let mut pool_full_passes: u64 = 0;
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
            send(output, event, at, anchor, &mut sounding)?;
            describe(event, settings, time_signature, end);
            last_instant = at;
            next += 1;
        }
        match output.flush().map_err(|e| e.to_string())? {
            FlushOutcome::AllHandedOver => pool_full = false,
            FlushOutcome::PoolFull => {
                pool_full = true;
                pool_full_passes += 1;
            }
        }
        thread::sleep(SEND_LOOP_SLEEP);
    }
    if pool_full_passes > 0 {
        eprintln!(
            "WARNING: MIDI output was full on {pool_full_passes} passes of {} ms; \
             events waited and may have been late.",
            SEND_LOOP_SLEEP.as_millis()
        );
    }
    if late_events > 0 {
        eprintln!("WARNING: {late_events} events were queued after their instant.");
    }
    if !sounding.is_empty() {
        // Every note on in the schedule has its note off, so this would be
        // a fault in the schedule.
        for note in &sounding {
            let port: &OutputPort = find_port(output, &note.port_name)?;
            output
                .send_note_off_now(port, note.channel.wire_value(), note.note.value())
                .map_err(|e| e.to_string())?;
        }
        eprintln!(
            "WARNING: {} notes were still sounding at the end; note offs sent.",
            sounding.len()
        );
    }
    Ok(last_instant)
}

fn send(
    output: &SequencerOutput,
    event: &ScheduledEvent,
    at: Instant,
    anchor: Anchor,
    sounding: &mut Vec<SoundingNote>,
) -> Result<(), String> {
    let port: &OutputPort = find_port(output, &event.port)?;
    let result: Result<(), AlsaOutputError> = match event.action {
        OutputAction::ClockPulse => output.queue_clock_pulse(port, at, 0),
        OutputAction::Transport(TransportMessage::Start) => output.queue_start(port, at),
        OutputAction::Transport(TransportMessage::Stop) => output.queue_stop(port, at),
        OutputAction::Transport(TransportMessage::Continue) => output.queue_continue(port, at),
        OutputAction::NoteOn {
            channel,
            note,
            velocity,
        } => {
            sounding.push(SoundingNote {
                port_name: event.port.clone(),
                channel,
                note,
            });
            output.queue_note_on(
                port,
                at,
                channel.wire_value(),
                note.value(),
                velocity.value(),
            )
        }
        OutputAction::NoteOff { channel, note } | OutputAction::LegatoNoteOff { channel, note } => {
            sounding.retain(|sounding_note: &SoundingNote| {
                !(sounding_note.port_name == event.port
                    && sounding_note.channel == channel
                    && sounding_note.note == note)
            });
            output.queue_note_off(port, at, channel.wire_value(), note.value())
        }
        OutputAction::TouchingNoteOff {
            channel,
            note,
            note_on_position,
            retrigger_gap,
        } => {
            sounding.retain(|sounding_note: &SoundingNote| {
                !(sounding_note.port_name == event.port
                    && sounding_note.channel == channel
                    && sounding_note.note == note)
            });
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
    };
    result.map_err(|e| e.to_string())
}

/// Prints each count of the count-in, each music bar's first count, and
/// each cue, as it is queued (up to 50 ms before it sounds). The song's
/// last position is shown as "End", not as a bar that never plays.
fn describe(
    event: &ScheduledEvent,
    settings: &Settings,
    time_signature: TimeSignature,
    end: ScorePosition,
) {
    let counter: Counter = Counter::at(event.position, time_signature, settings.count_in_bars);
    let place: String = if event.position == end {
        "End".to_string()
    } else {
        counter.to_string()
    };
    match (&event.source, event.action) {
        // The end of a trigger note is not worth a line.
        (EventSource::Cue { .. }, OutputAction::NoteOff { .. }) => {}
        (EventSource::Cue { cue_name, .. }, action) => {
            println!(
                "{place}   cue {cue_name:?} on {}: {}",
                event.port.as_str(),
                action_text(action)
            );
        }
        (EventSource::Notes { .. }, OutputAction::NoteOn { .. })
            if counter.section == CounterSection::CountIn || counter.beat == 1 =>
        {
            println!("{place}");
        }
        _ => {}
    }
}

/// The settings in use, defaults included, so the musician sees what the
/// run will do before pressing Enter.
fn print_settings(settings: &Settings) {
    let transport_port: &str = if settings.transport_on_clock {
        CLOCK_PORT
    } else {
        SQ1_PORT
    };
    println!(
        "Playing {} and {} at {} q/min, in 4/4.",
        bars_text(settings.count_in_bars, "count-in bar"),
        bars_text(settings.music_bars, "bar"),
        settings.quarter_notes_per_minute
    );
    println!(
        "Beat pulse: channel {}, note {}. Reset trigger: channel {}, note {}.",
        settings.pulse_channel.number(),
        settings.pulse_note.value(),
        settings.reset_channel.number(),
        settings.reset_note.value()
    );
    println!("SQ-1 Start and Stop on: {transport_port}.");
    match settings.restart_every_sixteenths {
        Some(sixteenths) => println!(
            "SQ-1 restart (Stop, Start) every {} from the music's first bar.",
            bars_text(sixteenths, "sixteenth")
        ),
        None => println!("SQ-1 restart: none."),
    }
    if !settings.transport_on_clock {
        println!(
            "  (Through a MIDI router that passes transport only from its clock master, add --transport-on-clock.)"
        );
    }
}

/// "1 bar", "2 bars": English plurals with the unit word.
fn bars_text(count: u64, unit: &str) -> String {
    if count == 1 {
        format!("1 {unit}")
    } else {
        format!("{count} {unit}s")
    }
}

fn action_text(action: OutputAction) -> String {
    match action {
        OutputAction::ClockPulse => "Timing Clock".to_string(),
        OutputAction::Transport(TransportMessage::Start) => "Start".to_string(),
        OutputAction::Transport(TransportMessage::Stop) => "Stop".to_string(),
        OutputAction::Transport(TransportMessage::Continue) => "Continue".to_string(),
        OutputAction::NoteOn {
            channel,
            note,
            velocity,
        } => format!(
            "note {} velocity {} on channel {}",
            note.value(),
            velocity.value(),
            channel.number()
        ),
        OutputAction::NoteOff { channel, note }
        | OutputAction::LegatoNoteOff { channel, note }
        | OutputAction::TouchingNoteOff { channel, note, .. } => {
            format!("note off {} on channel {}", note.value(), channel.number())
        }
        OutputAction::ControlChange {
            channel,
            controller,
            value,
        } => format!(
            "CC {} = {} on channel {}",
            controller.value(),
            value.value(),
            channel.number()
        ),
        OutputAction::ProgramChange { channel, program } => {
            format!(
                "program {} on channel {}",
                program.value(),
                channel.number()
            )
        }
    }
}

fn find_port<'a>(
    output: &'a SequencerOutput,
    name: &OutputPortName,
) -> Result<&'a OutputPort, String> {
    output
        .ports()
        .iter()
        .find(|port: &&OutputPort| port.name() == name.as_str())
        .ok_or_else(|| format!("No output port named {:?}.", name.as_str()))
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

fn parse_settings() -> Result<Settings, String> {
    let mut quarter_notes_per_minute: u64 = 120;
    let mut count_in_bars: u64 = 1;
    let mut music_bars: u64 = 8;
    let mut pulse_channel: u8 = 15;
    let mut pulse_note: u8 = 60;
    let mut reset_channel: u8 = 16;
    let mut reset_note: u8 = 60;
    let mut transport_on_clock: bool = false;
    let mut restart_every_sixteenths: Option<u64> = None;
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut index: usize = 0;
    while index < arguments.len() {
        let option: &str = arguments[index].as_str();
        // The one option without a value.
        if option == "--transport-on-clock" {
            transport_on_clock = true;
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
            "--bpm" => quarter_notes_per_minute = number()?,
            "--count-in-bars" => count_in_bars = number()?,
            "--music-bars" => music_bars = number()?,
            "--pulse-channel" => pulse_channel = byte()?,
            "--pulse-note" => pulse_note = byte()?,
            "--reset-channel" => reset_channel = byte()?,
            "--reset-note" => reset_note = byte()?,
            "--restart-every-sixteenths" => restart_every_sixteenths = Some(number()?),
            _ => {
                return Err(format!(
                    "Unknown option {option}. Options: --bpm --count-in-bars --music-bars \
                     --pulse-channel --pulse-note --reset-channel --reset-note \
                     --transport-on-clock --restart-every-sixteenths"
                ));
            }
        }
        index += 2;
    }
    if count_in_bars > 2 {
        return Err(format!("--count-in-bars {count_in_bars}: use 0, 1 or 2."));
    }
    if restart_every_sixteenths == Some(0) {
        return Err("--restart-every-sixteenths 0: use 1 or more.".to_string());
    }
    if music_bars == 0 {
        return Err("--music-bars 0: use 1 or more.".to_string());
    }
    let channel =
        |what: &str, value: u8| MidiChannel::new(value).map_err(|e| format!("{what}: {e}"));
    let note = |what: &str, value: u8| NoteNumber::new(value).map_err(|e| format!("{what}: {e}"));
    Ok(Settings {
        quarter_notes_per_minute,
        count_in_bars,
        music_bars,
        pulse_channel: channel("--pulse-channel", pulse_channel)?,
        pulse_note: note("--pulse-note", pulse_note)?,
        reset_channel: channel("--reset-channel", reset_channel)?,
        reset_note: note("--reset-note", reset_note)?,
        transport_on_clock,
        restart_every_sixteenths,
    })
}
