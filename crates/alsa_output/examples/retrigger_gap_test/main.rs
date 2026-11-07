//! Plays one repeated note with a stepped gap between the notes, to find
//! what a driven device needs to retrigger its gate (Song Notation, Q11).
//!
//! `cargo run --release -p alsa_output --example retrigger_gap_test -- [options]`
//!
//! What the test proves, its findings and how to run it are in
//! `explainer.md` beside this file.
//!
//! | Option | Default | Meaning |
//! |---|---|---|
//! | `--bpm N` | 120 | tempo, quarter notes per minute |
//! | `--channel C` | 2 | MIDI channel, 1 to 16 |
//! | `--note N` | 24 | note number, 0 to 127 |
//! | `--velocity V` | 100 | note-on velocity, 1 to 127 |
//! | `--notes N` | 8 | notes in each group |
//! | `--gaps-us A,B,C` | 0,960,1100,1250,1400,1600 | the gap before each next note on, one group for each, in µs |
//! | `--on-before-off` | off | at a gap of 0 µs, queue each note on before the note off it touches |
//!
//! The defaults are harmless: they only choose what this test sends.
//!
//! One output port, `Bass`, in the client `Lead Sheet` (the MIDI router
//! suggests `lead-sheet-bass`). Connect it yourself, with the MIDI router
//! or `aconnect`, when the program asks.
//!
//! Ctrl-C stops the program at once and may leave a note sounding; send
//! All Notes Off from the router or play a note on the device.

use std::io::BufRead;
use std::process::ExitCode;
use std::thread;
use std::time::Duration as StdDuration;

use alsa_output::sequencer_output::{FlushOutcome, OutputPort, SequencerOutput};
use alsa_output::warning::OutputWarning;
use vocabulary::duration::{Duration, NANOSECONDS_PER_MILLISECOND};
use vocabulary::instant::Instant;
use vocabulary::midi_values::{MidiChannel, NoteNumber, Velocity};

const PORT_NAME: &str = "Bass";

/// Time between Enter and the first note, so the first events are not
/// already late when queued.
const START_DELAY: Duration = Duration::from_ms(200);
/// Time left after the last note, so its note off is delivered before the
/// program ends.
const END_DELAY: Duration = Duration::from_ms(200);

const NANOSECONDS_PER_MINUTE: u64 = 60_000_000_000;
const NANOSECONDS_PER_MICROSECOND: u64 = 1_000;
const EIGHTH_NOTES_PER_QUARTER_NOTE: u64 = 2;
/// The silence between two groups, so each group is heard as its own.
const EIGHTH_NOTES_BETWEEN_GROUPS: u64 = 8;

struct Settings {
    quarter_notes_per_minute: u64,
    channel: MidiChannel,
    note: NoteNumber,
    velocity: Velocity,
    notes_per_group: u64,
    gaps_us: Vec<u64>,
    on_before_off: bool,
}

/// One end of a note.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    On,
    Off,
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
    // 60e9 ns/min ÷ (q/min × 2 eighths/q) = ns per eighth.
    let eighth_note_ns: u64 = NANOSECONDS_PER_MINUTE
        / (settings.quarter_notes_per_minute * EIGHTH_NOTES_PER_QUARTER_NOTE);
    for gap_us in &settings.gaps_us {
        let gap: Duration = gap_from_microseconds(*gap_us)?;
        if gap.ns() >= eighth_note_ns {
            return Err(format!(
                "--gaps-us: {gap_us} µs is not shorter than an eighth note \
                 ({} µs at {} q/min).",
                eighth_note_ns / NANOSECONDS_PER_MICROSECOND,
                settings.quarter_notes_per_minute
            ));
        }
    }

    let (output, warnings): (SequencerOutput, Vec<OutputWarning>) =
        SequencerOutput::open("Lead Sheet", &[PORT_NAME]).map_err(|e| e.to_string())?;
    for warning in &warnings {
        eprintln!("WARNING: {warning}");
    }
    let port: &OutputPort = output
        .ports()
        .first()
        .ok_or_else(|| format!("No output port named {PORT_NAME:?}."))?;
    println!(
        "Port {PORT_NAME:?} is {}:{}.",
        output.client_id(),
        port.port_id()
    );
    print_settings(&settings, eighth_note_ns);
    println!("Connect the port now (MIDI router or aconnect), then press Enter to play.");
    wait_for_enter()?;

    let pause: Duration = Duration::from_ns(EIGHTH_NOTES_BETWEEN_GROUPS * eighth_note_ns);
    let mut group_start: Instant = output
        .now()
        .map_err(|e| e.to_string())?
        .checked_add(START_DELAY)
        .map_err(|e| e.to_string())?;
    for gap_us in &settings.gaps_us {
        let gap: Duration = gap_from_microseconds(*gap_us)?;
        let group_end: Instant =
            queue_group(&output, port, &settings, eighth_note_ns, gap, group_start)?;
        wait_until(&output, group_start)?;
        println!(
            "Gap {gap_us} µs: {} notes of {} µs, the next one {gap_us} µs after each ends.",
            settings.notes_per_group,
            (eighth_note_ns - gap.ns()) / NANOSECONDS_PER_MICROSECOND
        );
        wait_until(&output, group_end)?;
        group_start = group_end.checked_add(pause).map_err(|e| e.to_string())?;
    }
    let last_end: Instant = group_start;
    wait_until(
        &output,
        last_end.checked_add(END_DELAY).map_err(|e| e.to_string())?,
    )?;
    // Final safety net: every note in this test has its note off, so this
    // would only matter after a fault.
    output
        .send_all_notes_off_now(port, settings.channel.wire_value())
        .map_err(|e| e.to_string())?;
    println!("Done.");
    Ok(())
}

/// Queues one group: `notes_per_group` notes, each starting one eighth
/// note after the last started, each sounding for an eighth note less
/// `gap`. At a gap of 0 µs a note off and the next note on fall at the same
/// instant, the case the player sends today. Returns the instant the group
/// ends.
fn queue_group(
    output: &SequencerOutput,
    port: &OutputPort,
    settings: &Settings,
    eighth_note_ns: u64,
    gap: Duration,
    group_start: Instant,
) -> Result<Instant, String> {
    let mut edges: Vec<(u64, Edge)> = Vec::new();
    for note_index in 0..settings.notes_per_group {
        let start_offset_ns: u64 = note_index * eighth_note_ns;
        let end_offset_ns: u64 = (note_index + 1) * eighth_note_ns - gap.ns();
        edges.push((start_offset_ns, Edge::On));
        edges.push((end_offset_ns, Edge::Off));
    }
    // The player's rule: at one instant a note off comes before a note on,
    // so a repeated note is retriggered, not cut; `--on-before-off`
    // reverses it.
    let rank = |edge: Edge| -> u8 {
        match (edge, settings.on_before_off) {
            (Edge::Off, false) | (Edge::On, true) => 0,
            (Edge::On, false) | (Edge::Off, true) => 1,
        }
    };
    edges.sort_by_key(|(offset_ns, edge): &(u64, Edge)| (*offset_ns, rank(*edge)));
    for (offset_ns, edge) in &edges {
        let at: Instant = group_start
            .checked_add(Duration::from_ns(*offset_ns))
            .map_err(|e| e.to_string())?;
        let result = match edge {
            Edge::On => output.queue_note_on(
                port,
                at,
                settings.channel.wire_value(),
                settings.note.value(),
                settings.velocity.value(),
            ),
            Edge::Off => output.queue_note_off(
                port,
                at,
                settings.channel.wire_value(),
                settings.note.value(),
            ),
        };
        result.map_err(|e| e.to_string())?;
    }
    match output.flush().map_err(|e| e.to_string())? {
        FlushOutcome::AllHandedOver => {}
        FlushOutcome::PoolFull => {
            return Err(format!(
                "MIDI output was full while queuing a group of {} notes; \
                 use fewer notes with --notes.",
                settings.notes_per_group
            ));
        }
    }
    group_start
        .checked_add(Duration::from_ns(settings.notes_per_group * eighth_note_ns))
        .map_err(|e| e.to_string())
}

/// The settings in use, defaults included, so the musician sees what the
/// run will do before pressing Enter.
fn print_settings(settings: &Settings, eighth_note_ns: u64) {
    println!(
        "Playing {} groups of {} eighth notes at {} q/min ({} ms each): channel {}, note {}, velocity {}.",
        settings.gaps_us.len(),
        settings.notes_per_group,
        settings.quarter_notes_per_minute,
        eighth_note_ns / NANOSECONDS_PER_MILLISECOND,
        settings.channel.number(),
        settings.note.value(),
        settings.velocity.value()
    );
    println!(
        "Gaps, in order: {} µs. A gap of 0 µs is the touching case: note off and the next note on at the same instant.",
        settings
            .gaps_us
            .iter()
            .map(u64::to_string)
            .collect::<Vec<String>>()
            .join(", ")
    );
    if settings.on_before_off {
        println!("At the same instant each note on is queued BEFORE the note off it touches.");
    } else {
        println!(
            "At the same instant each note off is queued before the note on (as the player does)."
        );
    }
}

fn gap_from_microseconds(gap_us: u64) -> Result<Duration, String> {
    gap_us
        .checked_mul(NANOSECONDS_PER_MICROSECOND)
        .map(Duration::from_ns)
        .ok_or_else(|| format!("--gaps-us: {gap_us} µs is too large."))
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
    let mut channel: u8 = 2;
    let mut note: u8 = 24;
    let mut velocity: u8 = 100;
    let mut notes_per_group: u64 = 8;
    let mut gaps_us: Vec<u64> = vec![0, 960, 1100, 1250, 1400, 1600];
    let mut on_before_off: bool = false;
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut index: usize = 0;
    while index < arguments.len() {
        let option: &str = arguments[index].as_str();
        // The one option without a value.
        if option == "--on-before-off" {
            on_before_off = true;
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
            "--channel" => channel = byte()?,
            "--note" => note = byte()?,
            "--velocity" => velocity = byte()?,
            "--notes" => notes_per_group = number()?,
            "--gaps-us" => {
                gaps_us = value
                    .split(',')
                    .map(|part: &str| {
                        part.trim().parse::<u64>().map_err(|_| {
                            format!("{option} {value}: write whole numbers of µs, e.g. 0,500,960.")
                        })
                    })
                    .collect::<Result<Vec<u64>, String>>()?;
            }
            _ => {
                return Err(format!(
                    "Unknown option {option}. Options: --bpm --channel --note --velocity \
                     --notes --gaps-us --on-before-off"
                ));
            }
        }
        index += 2;
    }
    if quarter_notes_per_minute == 0 {
        return Err("--bpm 0: use 1 or more.".to_string());
    }
    if notes_per_group == 0 {
        return Err("--notes 0: use 1 or more.".to_string());
    }
    if velocity == 0 {
        return Err("--velocity 0: use 1 to 127 (0 would be a note off).".to_string());
    }
    Ok(Settings {
        quarter_notes_per_minute,
        channel: MidiChannel::new(channel).map_err(|e| format!("--channel: {e}"))?,
        note: NoteNumber::new(note).map_err(|e| format!("--note: {e}"))?,
        velocity: Velocity::new(velocity).map_err(|e| format!("--velocity: {e}"))?,
        notes_per_group,
        gaps_us,
        on_before_off,
    })
}
