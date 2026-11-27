//! What the sending loop needs of a MIDI output. The ALSA sequencer
//! implements it (`alsa_event_output`); the tests implement it with a
//! recording output and a simulated clock.
//!
//! Ports are named, as in the schedule; an output that has no port of a
//! name refuses events for it with a sentence saying so.

use alsa_output::sequencer_output::FlushOutcome;
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;

pub trait EventOutput {
    /// The output's own clock: the instants of every event are on it.
    ///
    /// # Errors
    ///
    /// When the clock cannot be read.
    fn now(&self) -> Result<Instant, String>;

    /// Lets a little time pass between two looks at the clock. The real
    /// output sleeps; a simulated one moves its clock on.
    fn wait_a_little(&self);

    /// Whether the output has a port of this name.
    fn has_port(&self, port_name: &str) -> bool;

    /// # Errors
    ///
    /// When the port is unknown, a value is out of range, or the output
    /// refuses the event. The same for every method below that sends.
    fn note_on(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        note: u8,
        velocity: u8,
    ) -> Result<(), String>;

    #[allow(clippy::missing_errors_doc)]
    fn note_off(&self, port_name: &str, at: Instant, channel: u8, note: u8) -> Result<(), String>;

    /// The note-off of a note that started at `note_on_at` and ends where
    /// the next note on the track starts, `next_note_on_at`: sent earlier
    /// than that by the larger of the output's own lead and `retrigger_gap`.
    #[allow(clippy::missing_errors_doc)]
    fn note_off_before_note_on(
        &self,
        port_name: &str,
        note_on_at: Instant,
        next_note_on_at: Instant,
        retrigger_gap: Duration,
        channel: u8,
        note: u8,
    ) -> Result<(), String>;

    #[allow(clippy::missing_errors_doc)]
    fn control_change(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        controller: u8,
        value: u8,
    ) -> Result<(), String>;

    #[allow(clippy::missing_errors_doc)]
    fn program_change(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        program: u8,
    ) -> Result<(), String>;

    #[allow(clippy::missing_errors_doc)]
    fn start(&self, port_name: &str, at: Instant) -> Result<(), String>;

    #[allow(clippy::missing_errors_doc)]
    fn stop(&self, port_name: &str, at: Instant) -> Result<(), String>;

    #[allow(clippy::missing_errors_doc)]
    fn continue_(&self, port_name: &str, at: Instant) -> Result<(), String>;

    #[allow(clippy::missing_errors_doc)]
    fn clock_pulse(&self, port_name: &str, at: Instant) -> Result<(), String>;

    /// Hands what has been sent so far to the kernel (or whatever carries
    /// it). A full output pool is not an error.
    ///
    /// # Errors
    ///
    /// When the output refuses for any other reason.
    fn flush(&self) -> Result<FlushOutcome, String>;

    /// Takes back every event sent and not yet delivered.
    ///
    /// # Errors
    ///
    /// When the output refuses.
    fn remove_queued_events(&self) -> Result<(), String>;

    /// A Note Off on a port and channel, at once.
    #[allow(clippy::missing_errors_doc)]
    fn send_note_off_now(&self, port_name: &str, channel: u8, note: u8) -> Result<(), String>;

    /// MIDI Stop on a port, at once.
    #[allow(clippy::missing_errors_doc)]
    fn send_stop_now(&self, port_name: &str) -> Result<(), String>;

    /// All Notes Off on a port and channel, at once.
    #[allow(clippy::missing_errors_doc)]
    fn send_all_notes_off_now(&self, port_name: &str, channel: u8) -> Result<(), String>;
}
