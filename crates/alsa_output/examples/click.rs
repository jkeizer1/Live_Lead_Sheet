//! Plays a steady click through the ALSA output and prints, for each click,
//! how far ahead of the queue's clock it was stamped (the lead, which the
//! player keeps within 50 ms, D-8).
//!
//! `cargo run -p alsa_output --example click [CLIENT:PORT]`
//!
//! With no destination the events go nowhere (no subscriber); the lead is
//! still printed. Find a destination's CLIENT:PORT with `aconnect -l`.

use std::process::ExitCode;
use std::thread;
use std::time::Duration as StdDuration;

use alsa_output::error::AlsaOutputError;
use alsa_output::sequencer_output::{MidiDestination, SequencerOutput};
use alsa_output::warning::OutputWarning;
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;

const CLICKS: u64 = 8;
const CLICK_INTERVAL: Duration = Duration::from_ms(500);
const CLICK_LENGTH: Duration = Duration::from_ms(50);
const FIRST_CLICK_DELAY: Duration = Duration::from_ms(1000);
/// The player's lead window (D-8).
const LEAD: Duration = Duration::from_ms(50);
const CHANNEL: u8 = 9; // MIDI channel 10, percussion on General MIDI
const NOTE: u8 = 76; // high wood block
const VELOCITY: u8 = 100;

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
    let destination: Option<MidiDestination> = parse_destination()?;
    let (output, warnings): (SequencerOutput, Vec<OutputWarning>) =
        SequencerOutput::open("Live Lead Sheet click").map_err(|e| e.to_string())?;
    for warning in &warnings {
        eprintln!("WARNING: {warning}");
    }
    match destination {
        Some(destination) => output.connect_to(destination).map_err(|e| e.to_string())?,
        None => eprintln!("No destination given: the clicks go nowhere."),
    }
    play(&output).map_err(|e| e.to_string())
}

fn play(output: &SequencerOutput) -> Result<(), AlsaOutputError> {
    // Every click is computed from this anchor, never from the previous
    // click, so no error accumulates.
    let first_click: Instant = add(output.now()?, FIRST_CLICK_DELAY)?;
    for click_number in 0..CLICKS {
        let click_at: Instant = add(
            first_click,
            Duration::from_ns(CLICK_INTERVAL.ns() * click_number),
        )?;
        // Sleep until the click is LEAD ahead, then stamp it.
        let send_at: Instant = Instant::from_ns(click_at.ns().saturating_sub(LEAD.ns()));
        // Already past send time is harmless: the click is simply sent now.
        let wait: Duration = send_at.checked_sub(output.now()?).unwrap_or(Duration::ZERO);
        thread::sleep(StdDuration::from_nanos(wait.ns()));

        output.queue_note_on(click_at, CHANNEL, NOTE, VELOCITY)?;
        output.queue_note_off(add(click_at, CLICK_LENGTH)?, CHANNEL, NOTE)?;
        output.flush()?;
        // Negative lead (stamped in the past) would show as 0 µs here.
        let lead: Duration = click_at
            .checked_sub(output.now()?)
            .unwrap_or(Duration::ZERO);
        println!(
            "click {click_number}: stamped {} µs ahead of the clock",
            lead.ns() / 1000
        );
    }
    // Let the last click sound before the program closes the queue.
    thread::sleep(StdDuration::from_nanos(CLICK_LENGTH.ns() + LEAD.ns()));
    Ok(())
}

fn add(instant: Instant, duration: Duration) -> Result<Instant, AlsaOutputError> {
    instant
        .checked_add(duration)
        .map_err(|_| AlsaOutputError::InstantOutOfRange {
            instant_ns: instant.ns(),
        })
}

fn parse_destination() -> Result<Option<MidiDestination>, String> {
    let Some(argument): Option<String> = std::env::args().nth(1) else {
        return Ok(None);
    };
    let bad = || {
        format!("Destination {argument:?} is not CLIENT:PORT. Example: 128:0 (see aconnect -l).")
    };
    let (client, port): (&str, &str) = argument.split_once(':').ok_or_else(bad)?;
    Ok(Some(MidiDestination {
        client_id: client.parse().map_err(|_| bad())?,
        port_id: port.parse().map_err(|_| bad())?,
    }))
}
