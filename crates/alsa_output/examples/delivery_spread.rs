//! Measures how late the ALSA queue delivers MIDI clock pulses stamped
//! through the output (evidence for D-8, Musical Time and Clock section 5).
//! The pulses are queued about 50 ms ahead and the kernel's timer releases
//! them. See `measurement/mod.rs` for the method.
//!
//! `cargo run --release -p alsa_output --example delivery_spread -- [--bpm N] [--pulses N] [--load N]`
//!
//! `--bpm` whole quarter notes per minute (default 120), `--pulses` how many
//! pulses to measure (default 3000), `--load` busy threads to compete for the
//! CPU, standing in for drawing (default 0).

#[path = "measurement/mod.rs"]
mod measurement;

use std::process::ExitCode;
use std::thread;
use std::time::Duration as StdDuration;

use alsa_output::sequencer_output::SequencerOutput;
use measurement::{
    FIRST_PULSE_DELAY, Settings, parse_settings, pulse_instant, run_measurement, tag_for,
};
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;

/// The player's lead window (D-8).
const LEAD: Duration = Duration::from_ms(50);

fn main() -> ExitCode {
    let result: Result<(), String> = parse_settings(false)
        .and_then(|settings| run_measurement("Live Lead Sheet queue", &settings, send_pulses));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Sends every pulse about `LEAD` ahead of its instant, as the player will.
fn send_pulses(output: &SequencerOutput, settings: &Settings) -> Result<Vec<Instant>, String> {
    let first_pulse: Instant = output
        .now()
        .map_err(|e| e.to_string())?
        .checked_add(FIRST_PULSE_DELAY)
        .map_err(|e| e.to_string())?;
    let mut scheduled: Vec<Instant> = Vec::new();
    let mut late_sends: u64 = 0;
    for pulse_number in 0..settings.pulses {
        // Each pulse is computed from the anchor, never from the previous one.
        let pulse_at: Instant =
            pulse_instant(first_pulse, pulse_number, settings.quarter_notes_per_minute)?;
        let send_at: Instant = Instant::from_ns(pulse_at.ns().saturating_sub(LEAD.ns()));
        let now: Instant = output.now().map_err(|e| e.to_string())?;
        // Already past send time: send now; counted below if the pulse itself is late.
        let wait: Duration = send_at.checked_sub(now).unwrap_or(Duration::ZERO);
        thread::sleep(StdDuration::from_nanos(wait.ns()));

        if output.now().map_err(|e| e.to_string())? > pulse_at {
            late_sends += 1;
        }
        output
            .queue_clock_pulse(pulse_at, tag_for(pulse_number)?)
            .map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
        scheduled.push(pulse_at);
    }
    if late_sends > 0 {
        eprintln!("WARNING: {late_sends} pulses were queued after their instant.");
    }
    Ok(scheduled)
}
