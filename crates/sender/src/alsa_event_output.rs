//! The ALSA sequencer as the sending loop's output: each call finds the
//! port by name and passes on to `SequencerOutput`.

use std::thread;
use std::time::Duration as StdDuration;

use alsa_output::sequencer_output::{FlushOutcome, OutputPort, SequencerOutput};
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;

use crate::event_output::EventOutput;

/// How long the loop sleeps between looks at the clock: well under the
/// 50 ms lead.
const SEND_LOOP_SLEEP: StdDuration = StdDuration::from_millis(5);

fn find_port<'a>(output: &'a SequencerOutput, name: &str) -> Result<&'a OutputPort, String> {
    output
        .ports()
        .iter()
        .find(|port: &&OutputPort| port.name() == name)
        .ok_or_else(|| format!("No output port named {name:?}."))
}

impl EventOutput for SequencerOutput {
    fn now(&self) -> Result<Instant, String> {
        SequencerOutput::now(self).map_err(|error| error.to_string())
    }

    fn wait_a_little(&self) {
        thread::sleep(SEND_LOOP_SLEEP);
    }

    fn has_port(&self, port_name: &str) -> bool {
        find_port(self, port_name).is_ok()
    }

    fn note_on(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        note: u8,
        velocity: u8,
    ) -> Result<(), String> {
        self.queue_note_on(find_port(self, port_name)?, at, channel, note, velocity)
            .map_err(|error| error.to_string())
    }

    fn note_off(&self, port_name: &str, at: Instant, channel: u8, note: u8) -> Result<(), String> {
        self.queue_note_off(find_port(self, port_name)?, at, channel, note)
            .map_err(|error| error.to_string())
    }

    fn note_off_before_note_on(
        &self,
        port_name: &str,
        note_on_at: Instant,
        next_note_on_at: Instant,
        retrigger_gap: Duration,
        channel: u8,
        note: u8,
    ) -> Result<(), String> {
        self.queue_note_off_before_note_on(
            find_port(self, port_name)?,
            note_on_at,
            next_note_on_at,
            retrigger_gap,
            channel,
            note,
        )
        .map_err(|error| error.to_string())
    }

    fn control_change(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        controller: u8,
        value: u8,
    ) -> Result<(), String> {
        self.queue_control_change(find_port(self, port_name)?, at, channel, controller, value)
            .map_err(|error| error.to_string())
    }

    fn program_change(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        program: u8,
    ) -> Result<(), String> {
        self.queue_program_change(find_port(self, port_name)?, at, channel, program)
            .map_err(|error| error.to_string())
    }

    fn start(&self, port_name: &str, at: Instant) -> Result<(), String> {
        self.queue_start(find_port(self, port_name)?, at)
            .map_err(|error| error.to_string())
    }

    fn stop(&self, port_name: &str, at: Instant) -> Result<(), String> {
        self.queue_stop(find_port(self, port_name)?, at)
            .map_err(|error| error.to_string())
    }

    fn continue_(&self, port_name: &str, at: Instant) -> Result<(), String> {
        self.queue_continue(find_port(self, port_name)?, at)
            .map_err(|error| error.to_string())
    }

    fn clock_pulse(&self, port_name: &str, at: Instant) -> Result<(), String> {
        self.queue_clock_pulse(find_port(self, port_name)?, at, 0)
            .map_err(|error| error.to_string())
    }

    fn flush(&self) -> Result<FlushOutcome, String> {
        SequencerOutput::flush(self).map_err(|error| error.to_string())
    }

    fn remove_queued_events(&self) -> Result<(), String> {
        SequencerOutput::remove_queued_events(self).map_err(|error| error.to_string())
    }

    fn send_note_off_now(&self, port_name: &str, channel: u8, note: u8) -> Result<(), String> {
        SequencerOutput::send_note_off_now(self, find_port(self, port_name)?, channel, note)
            .map_err(|error| error.to_string())
    }

    fn send_stop_now(&self, port_name: &str) -> Result<(), String> {
        SequencerOutput::send_stop_now(self, find_port(self, port_name)?)
            .map_err(|error| error.to_string())
    }

    fn send_all_notes_off_now(&self, port_name: &str, channel: u8) -> Result<(), String> {
        SequencerOutput::send_all_notes_off_now(self, find_port(self, port_name)?, channel)
            .map_err(|error| error.to_string())
    }
}
