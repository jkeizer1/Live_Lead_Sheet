//! Measures how late MIDI clock pulses arrive when this program waits for
//! each pulse's instant itself and then sends it at once: the fallback in
//! D-8 (Musical Time and Clock section 5). The queue's clock is still the
//! one read, so the result is comparable with `delivery_spread`. See
//! `measurement/mod.rs` for the method.
//!
//! `cargo run --release -p alsa_output --example deadline_spread -- [--bpm N] [--pulses N] [--load N] [--spin-us N]`
//!
//! Options as for `delivery_spread`, plus `--spin-us`: the last N µs before
//! each pulse are spent busy-waiting instead of sleeping, which trades CPU
//! for a more exact wake-up (default 0: sleep all the way).

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

fn main() -> ExitCode {
    let result: Result<(), String> = parse_settings(true)
        .and_then(|settings| run_measurement("Live Lead Sheet deadline", &settings, send_pulses));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn send_pulses(output: &SequencerOutput, settings: &Settings) -> Result<Vec<Instant>, String> {
    let first_pulse: Instant = output
        .now()
        .map_err(|e| e.to_string())?
        .checked_add(FIRST_PULSE_DELAY)
        .map_err(|e| e.to_string())?;
    let mut scheduled: Vec<Instant> = Vec::new();
    for pulse_number in 0..settings.pulses {
        // Each pulse is computed from the anchor, never from the previous one.
        let pulse_at: Instant =
            pulse_instant(first_pulse, pulse_number, settings.quarter_notes_per_minute)?;
        wait_until(output, pulse_at, settings.spin)?;
        output
            .send_clock_pulse_now(tag_for(pulse_number)?)
            .map_err(|e| e.to_string())?;
        scheduled.push(pulse_at);
    }
    Ok(scheduled)
}

/// Returns at `deadline` or later: sleeps until `spin` before it, then
/// reads the clock in a loop. The clock is read again after every sleep
/// because a sleep can end late (never early, but the clocks differ a little).
fn wait_until(output: &SequencerOutput, deadline: Instant, spin: Duration) -> Result<(), String> {
    loop {
        let now: Instant = output.now().map_err(|e| e.to_string())?;
        // A deadline already reached is harmless: the pulse goes out now and
        // the report shows how late it was.
        let Ok(remaining): Result<Duration, _> = deadline.checked_sub(now) else {
            return Ok(());
        };
        if remaining == Duration::ZERO {
            return Ok(());
        }
        if remaining > spin {
            thread::sleep(StdDuration::from_nanos(remaining.ns() - spin.ns()));
        }
    }
}
