//! Plays the most basic legato case, as a keyboard player plays it: a bass
//! note on at beat 1, a second note on at beat 3 while the first is still
//! held, then both notes off at the end of the bar. A mono voice should
//! keep one gate, from beat 1 to the end of the bar, and change pitch at
//! beat 3. A steady quarter-note beat plays on its own port.
//!
//! `cargo run --release -p alsa_output --example held_notes_test -- [options]`
//!
//! What the test proves, its findings and how to run it are in
//! `explainer.md` beside this file.
//!
//! | Option | Default | Meaning |
//! |---|---|---|
//! | `--bpm N` | 120 | tempo, quarter notes per minute |
//! | `--bars N` | 4 | bars to play, in 4/4 |
//! | `--channel C` | 2 | the bass's MIDI channel, 1 to 16 |
//! | `--first-note N` | 36 | the note on at beat 1, 0 to 127 |
//! | `--second-note N` | 43 | the note on at beat 3, 0 to 127 |
//! | `--velocity V` | 100 | the bass's note-on velocity, 1 to 127 |
//!
//! The defaults are harmless: they only choose what this test sends.
//!
//! Two output ports in the client `Lead Sheet`: `Bass`, and `Beat pulse`
//! (note 60, channel 15, velocity 80, as in `songs/Demo.studio`). Connect
//! them yourself, with the MIDI router or `aconnect`, when the program
//! asks. The end of each bar's notes is sent 5 ms early, as the player does
//! for a note that touches the next, so the next bar's first note on
//! retriggers.
//!
//! Ctrl-C stops the program at once and may leave a note sounding; send
//! All Notes Off from the router or play a note on the device.

use std::io::BufRead;
use std::process::ExitCode;
use std::thread;
use std::time::Duration as StdDuration;

use alsa_output::sequencer_output::{
    FlushOutcome, OutputPort, SequencerOutput, TOUCHING_NOTE_OFF_LEAD,
};
use alsa_output::warning::OutputWarning;
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::midi_values::{MidiChannel, NoteNumber, Velocity};

const BASS_PORT_NAME: &str = "Bass";
const BEAT_PORT_NAME: &str = "Beat pulse";
/// As in `songs/Demo.studio`: the Beat track's channel, note and velocity.
const BEAT_CHANNEL_NUMBER: u8 = 15;
const BEAT_NOTE_NUMBER: u8 = 60;
const BEAT_VELOCITY_VALUE: u8 = 80;

/// Time between Enter and the first note, so the first events are not
/// already late when queued.
const START_DELAY: Duration = Duration::from_ms(200);
/// Time left after the last note, so its note off is delivered before the
/// program ends.
const END_DELAY: Duration = Duration::from_ms(200);

const NANOSECONDS_PER_MINUTE: u64 = 60_000_000_000;
const BEATS_PER_BAR: u64 = 4;
/// The second note comes in at beat 3: two beats after the bar starts.
const SECOND_NOTE_BEATS_INTO_BAR: u64 = 2;

struct Settings {
    quarter_notes_per_minute: u64,
    bars: u64,
    bass_channel: MidiChannel,
    first_note: NoteNumber,
    second_note: NoteNumber,
    velocity: Velocity,
}

/// One message to queue, at an offset from the start of the music.
struct Message {
    offset_ns: u64,
    port_name: &'static str,
    channel: MidiChannel,
    note: NoteNumber,
    /// `Some` for a note on, `None` for a note off.
    velocity: Option<Velocity>,
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
    // 60e9 ns/min ÷ q/min = ns per beat (a beat is a quarter note in 4/4).
    let beat_ns: u64 = NANOSECONDS_PER_MINUTE / settings.quarter_notes_per_minute;
    let messages: Vec<Message> = build_messages(&settings, beat_ns)?;

    let (output, warnings): (SequencerOutput, Vec<OutputWarning>) =
        SequencerOutput::open("Lead Sheet", &[BASS_PORT_NAME, BEAT_PORT_NAME])
            .map_err(|e| e.to_string())?;
    for warning in &warnings {
        eprintln!("WARNING: {warning}");
    }
    for port in output.ports() {
        println!(
            "Port {:?} is {}:{}.",
            port.name(),
            output.client_id(),
            port.port_id()
        );
    }
    println!(
        "Playing {} bars at {} q/min. Bass, channel {}: note {} on at beat 1, note {} on at beat 3 \
         with the first still held, both notes off at the end of the bar (5 ms early, so the next \
         bar retriggers). Beat: steady quarter notes, channel {BEAT_CHANNEL_NUMBER}.",
        settings.bars,
        settings.quarter_notes_per_minute,
        settings.bass_channel.number(),
        settings.first_note.value(),
        settings.second_note.value()
    );
    println!("Connect the ports now (MIDI router or aconnect), then press Enter to play.");
    wait_for_enter()?;

    let start: Instant = output
        .now()
        .map_err(|e| e.to_string())?
        .checked_add(START_DELAY)
        .map_err(|e| e.to_string())?;
    for message in &messages {
        queue(&output, message, start)?;
    }
    match output.flush().map_err(|e| e.to_string())? {
        FlushOutcome::AllHandedOver => {}
        FlushOutcome::PoolFull => {
            return Err(format!(
                "MIDI output was full while queuing {} bars; use fewer with --bars.",
                settings.bars
            ));
        }
    }
    for bar in 0..settings.bars {
        wait_until(
            &output,
            offset_instant(start, bar * BEATS_PER_BAR * beat_ns)?,
        )?;
        println!("Bar {}.", bar + 1);
    }
    let end: Instant = offset_instant(start, settings.bars * BEATS_PER_BAR * beat_ns)?;
    wait_until(
        &output,
        end.checked_add(END_DELAY).map_err(|e| e.to_string())?,
    )?;
    // Final safety net: every note here has its note off, so this would
    // only matter after a fault.
    for port_name in [BASS_PORT_NAME, BEAT_PORT_NAME] {
        let port: &OutputPort = find_port(&output, port_name)?;
        let channel_wire_value: u8 = if port_name == BASS_PORT_NAME {
            settings.bass_channel.wire_value()
        } else {
            beat_channel()?.wire_value()
        };
        output
            .send_all_notes_off_now(port, channel_wire_value)
            .map_err(|e| e.to_string())?;
    }
    println!("Done.");
    Ok(())
}

/// Every message of the run, in the order to queue them: the bass note
/// ons at beats 1 and 3 of each bar, both its note offs 5 ms before the
/// bar ends, and the beat's quarter notes, each off 5 ms before the next.
fn build_messages(settings: &Settings, beat_ns: u64) -> Result<Vec<Message>, String> {
    let beat_channel: MidiChannel = beat_channel()?;
    let beat_note: NoteNumber =
        NoteNumber::new(BEAT_NOTE_NUMBER).map_err(|e| format!("beat note: {e}"))?;
    let beat_velocity: Velocity =
        Velocity::new(BEAT_VELOCITY_VALUE).map_err(|e| format!("beat velocity: {e}"))?;
    let lead_ns: u64 = TOUCHING_NOTE_OFF_LEAD.ns();
    let mut messages: Vec<Message> = Vec::new();
    for bar in 0..settings.bars {
        let bar_start_ns: u64 = bar * BEATS_PER_BAR * beat_ns;
        let bar_end_ns: u64 = (bar + 1) * BEATS_PER_BAR * beat_ns;
        messages.push(Message {
            offset_ns: bar_start_ns,
            port_name: BASS_PORT_NAME,
            channel: settings.bass_channel,
            note: settings.first_note,
            velocity: Some(settings.velocity),
        });
        messages.push(Message {
            offset_ns: bar_start_ns + SECOND_NOTE_BEATS_INTO_BAR * beat_ns,
            port_name: BASS_PORT_NAME,
            channel: settings.bass_channel,
            note: settings.second_note,
            velocity: Some(settings.velocity),
        });
        for note in [settings.first_note, settings.second_note] {
            messages.push(Message {
                offset_ns: bar_end_ns - lead_ns,
                port_name: BASS_PORT_NAME,
                channel: settings.bass_channel,
                note,
                velocity: None,
            });
        }
    }
    for beat in 0..settings.bars * BEATS_PER_BAR {
        messages.push(Message {
            offset_ns: beat * beat_ns,
            port_name: BEAT_PORT_NAME,
            channel: beat_channel,
            note: beat_note,
            velocity: Some(beat_velocity),
        });
        messages.push(Message {
            offset_ns: (beat + 1) * beat_ns - lead_ns,
            port_name: BEAT_PORT_NAME,
            channel: beat_channel,
            note: beat_note,
            velocity: None,
        });
    }
    // The queue sends by instant; the sort is stable, so messages at one
    // instant keep the order above.
    messages.sort_by_key(|message: &Message| message.offset_ns);
    Ok(messages)
}

fn beat_channel() -> Result<MidiChannel, String> {
    MidiChannel::new(BEAT_CHANNEL_NUMBER).map_err(|e| format!("beat channel: {e}"))
}

fn queue(output: &SequencerOutput, message: &Message, start: Instant) -> Result<(), String> {
    let port: &OutputPort = find_port(output, message.port_name)?;
    let at: Instant = offset_instant(start, message.offset_ns)?;
    let result = match message.velocity {
        Some(velocity) => output.queue_note_on(
            port,
            at,
            message.channel.wire_value(),
            message.note.value(),
            velocity.value(),
        ),
        None => output.queue_note_off(port, at, message.channel.wire_value(), message.note.value()),
    };
    result.map_err(|e| e.to_string())
}

fn offset_instant(start: Instant, offset_ns: u64) -> Result<Instant, String> {
    start
        .checked_add(Duration::from_ns(offset_ns))
        .map_err(|e| e.to_string())
}

fn find_port<'a>(output: &'a SequencerOutput, name: &str) -> Result<&'a OutputPort, String> {
    output
        .ports()
        .iter()
        .find(|port: &&OutputPort| port.name() == name)
        .ok_or_else(|| format!("No output port named {name:?}."))
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
    let mut bars: u64 = 4;
    let mut channel: u8 = 2;
    let mut first_note: u8 = 36;
    let mut second_note: u8 = 43;
    let mut velocity: u8 = 100;
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut index: usize = 0;
    while index < arguments.len() {
        let option: &str = arguments[index].as_str();
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
            "--bars" => bars = number()?,
            "--channel" => channel = byte()?,
            "--first-note" => first_note = byte()?,
            "--second-note" => second_note = byte()?,
            "--velocity" => velocity = byte()?,
            _ => {
                return Err(format!(
                    "Unknown option {option}. Options: --bpm --bars --channel --first-note \
                     --second-note --velocity"
                ));
            }
        }
        index += 2;
    }
    if quarter_notes_per_minute == 0 {
        return Err("--bpm 0: use 1 or more.".to_string());
    }
    if bars == 0 {
        return Err("--bars 0: use 1 or more.".to_string());
    }
    if velocity == 0 {
        return Err("--velocity 0: use 1 to 127 (0 would be a note off).".to_string());
    }
    Ok(Settings {
        quarter_notes_per_minute,
        bars,
        bass_channel: MidiChannel::new(channel).map_err(|e| format!("--channel: {e}"))?,
        first_note: NoteNumber::new(first_note).map_err(|e| format!("--first-note: {e}"))?,
        second_note: NoteNumber::new(second_note).map_err(|e| format!("--second-note: {e}"))?,
        velocity: Velocity::new(velocity).map_err(|e| format!("--velocity: {e}"))?,
    })
}
