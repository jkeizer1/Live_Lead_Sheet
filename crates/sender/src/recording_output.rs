//! A test output: it records what the sending loop hands it and when, on a
//! simulated clock that moves 5 ms each time the loop waits, and sends the
//! loop the commands a test scripts at chosen instants. No sleeping, no
//! ALSA, so a test is exact and runs at once.

use std::cell::{Cell, RefCell};
use std::sync::mpsc::Sender;

use alsa_output::sequencer_output::{FlushOutcome, TOUCHING_NOTE_OFF_LEAD};
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;

use crate::event_output::EventOutput;
use crate::play_request::{PlayRequest, Request};

/// How far the simulated clock moves each time the loop waits (the real
/// output sleeps 5 ms).
const WAIT_NS: u64 = 5_000_000;

/// One event the loop handed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SentEvent {
    NoteOn {
        port: String,
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        port: String,
        channel: u8,
        note: u8,
    },
    /// A note-off placed before the next note-on on the same track.
    TouchingNoteOff {
        port: String,
        channel: u8,
        note: u8,
        note_on_instant_ns: u64,
        next_note_on_instant_ns: u64,
        retrigger_gap_ns: u64,
    },
    ControlChange {
        port: String,
        channel: u8,
        controller: u8,
        value: u8,
    },
    ProgramChange {
        port: String,
        channel: u8,
        program: u8,
    },
    Start {
        port: String,
    },
    Stop {
        port: String,
    },
    Continue {
        port: String,
    },
    ClockPulse {
        port: String,
    },
}

/// An event with when it was handed over and when it is due.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sent {
    pub(crate) handed_over_ns: u64,
    /// The instant the output delivers it: for a touching note-off, earlier
    /// than the next note-on by the larger of the output's lead and the gap.
    pub(crate) due_instant_ns: u64,
    /// Taken back before its instant by `remove_queued_events`.
    pub(crate) cancelled: bool,
    pub(crate) event: SentEvent,
}

/// What was sent at once, bypassing the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SentNow {
    NoteOff { port: String, channel: u8, note: u8 },
    Stop { port: String },
    AllNotesOff { port: String, channel: u8 },
}

/// A command a test sends the loop at an instant.
pub(crate) enum ScriptedCommand {
    Stop,
    Play(Box<PlayRequest>),
}

pub(crate) struct RecordingOutput {
    ports: Vec<String>,
    clock_ns: Cell<u64>,
    sent: RefCell<Vec<Sent>>,
    sent_now: RefCell<Vec<(u64, SentNow)>>,
    script: RefCell<Vec<(u64, ScriptedCommand)>>,
    commands: Sender<Request>,
    silences: Cell<usize>,
    /// The clock jumps this much further at the first wait at or after the instant: the loop's thread not running.
    stall: RefCell<Option<(u64, u64)>>,
    /// The clock at each silencing: when each play ended.
    silenced_at_ns: RefCell<Vec<u64>>,
    quit_after_silences: usize,
}

impl RecordingOutput {
    /// `quit_after_silences`: the loop silences the output each time a play
    /// ends; after that many, the output sends Quit so the loop returns.
    pub(crate) fn new(
        ports: &[&str],
        start_ns: u64,
        commands: Sender<Request>,
        quit_after_silences: usize,
    ) -> RecordingOutput {
        RecordingOutput {
            ports: ports.iter().map(|name| (*name).to_string()).collect(),
            clock_ns: Cell::new(start_ns),
            sent: RefCell::new(Vec::new()),
            sent_now: RefCell::new(Vec::new()),
            script: RefCell::new(Vec::new()),
            commands,
            silences: Cell::new(0),
            stall: RefCell::new(None),
            silenced_at_ns: RefCell::new(Vec::new()),
            quit_after_silences,
        }
    }

    /// A command to send the loop when the clock reaches `instant_ns`.
    pub(crate) fn at(&self, instant_ns: u64, command: ScriptedCommand) {
        self.script.borrow_mut().push((instant_ns, command));
    }

    /// Holds the loop up for `for_ns` at its first wait at or after `at_ns`.
    pub(crate) fn stall_once(&self, at_ns: u64, for_ns: u64) {
        *self.stall.borrow_mut() = Some((at_ns, for_ns));
    }

    pub(crate) fn sent(&self) -> Vec<Sent> {
        self.sent.borrow().clone()
    }

    /// What was handed over and not taken back.
    pub(crate) fn delivered(&self) -> Vec<Sent> {
        self.sent
            .borrow()
            .iter()
            .filter(|sent| !sent.cancelled)
            .cloned()
            .collect()
    }

    pub(crate) fn sent_now(&self) -> Vec<(u64, SentNow)> {
        self.sent_now.borrow().clone()
    }

    /// The clock when each play ended, in order.
    pub(crate) fn silenced_at_ns(&self) -> Vec<u64> {
        self.silenced_at_ns.borrow().clone()
    }

    fn record(&self, due_instant_ns: u64, event: SentEvent) -> Result<(), String> {
        let port: &str = match &event {
            SentEvent::NoteOn { port, .. }
            | SentEvent::NoteOff { port, .. }
            | SentEvent::TouchingNoteOff { port, .. }
            | SentEvent::ControlChange { port, .. }
            | SentEvent::ProgramChange { port, .. }
            | SentEvent::Start { port }
            | SentEvent::Stop { port }
            | SentEvent::Continue { port }
            | SentEvent::ClockPulse { port } => port,
        };
        if !self.has_port(port) {
            return Err(format!("No output port named {port:?}."));
        }
        self.sent.borrow_mut().push(Sent {
            handed_over_ns: self.clock_ns.get(),
            due_instant_ns,
            cancelled: false,
            event,
        });
        Ok(())
    }
}

impl EventOutput for RecordingOutput {
    fn now(&self) -> Result<Instant, String> {
        Ok(Instant::from_ns(self.clock_ns.get()))
    }

    fn wait_a_little(&self) {
        self.clock_ns.set(self.clock_ns.get() + WAIT_NS);
        let stalled: Option<(u64, u64)> = *self.stall.borrow();
        if let Some((at_ns, for_ns)) = stalled
            && self.clock_ns.get() >= at_ns
        {
            self.clock_ns.set(self.clock_ns.get() + for_ns);
            *self.stall.borrow_mut() = None;
        }
        let now: u64 = self.clock_ns.get();
        let mut script = self.script.borrow_mut();
        let mut index: usize = 0;
        while index < script.len() {
            if script[index].0 <= now {
                let (_, command): (u64, ScriptedCommand) = script.remove(index);
                let request: Request = match command {
                    ScriptedCommand::Stop => Request::Stop,
                    ScriptedCommand::Play(play) => Request::Play(play),
                };
                // The loop's own receiver is alive while it waits.
                let _ = self.commands.send(request);
            } else {
                index += 1;
            }
        }
    }

    fn has_port(&self, port_name: &str) -> bool {
        self.ports.iter().any(|name| name == port_name)
    }

    fn note_on(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        note: u8,
        velocity: u8,
    ) -> Result<(), String> {
        self.record(
            at.ns(),
            SentEvent::NoteOn {
                port: port_name.to_string(),
                channel,
                note,
                velocity,
            },
        )
    }

    fn note_off(&self, port_name: &str, at: Instant, channel: u8, note: u8) -> Result<(), String> {
        self.record(
            at.ns(),
            SentEvent::NoteOff {
                port: port_name.to_string(),
                channel,
                note,
            },
        )
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
        let lead_ns: u64 = retrigger_gap.ns().max(TOUCHING_NOTE_OFF_LEAD.ns());
        self.record(
            next_note_on_at.ns().saturating_sub(lead_ns),
            SentEvent::TouchingNoteOff {
                port: port_name.to_string(),
                channel,
                note,
                note_on_instant_ns: note_on_at.ns(),
                next_note_on_instant_ns: next_note_on_at.ns(),
                retrigger_gap_ns: retrigger_gap.ns(),
            },
        )
    }

    fn control_change(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        controller: u8,
        value: u8,
    ) -> Result<(), String> {
        self.record(
            at.ns(),
            SentEvent::ControlChange {
                port: port_name.to_string(),
                channel,
                controller,
                value,
            },
        )
    }

    fn program_change(
        &self,
        port_name: &str,
        at: Instant,
        channel: u8,
        program: u8,
    ) -> Result<(), String> {
        self.record(
            at.ns(),
            SentEvent::ProgramChange {
                port: port_name.to_string(),
                channel,
                program,
            },
        )
    }

    fn start(&self, port_name: &str, at: Instant) -> Result<(), String> {
        self.record(
            at.ns(),
            SentEvent::Start {
                port: port_name.to_string(),
            },
        )
    }

    fn stop(&self, port_name: &str, at: Instant) -> Result<(), String> {
        self.record(
            at.ns(),
            SentEvent::Stop {
                port: port_name.to_string(),
            },
        )
    }

    fn continue_(&self, port_name: &str, at: Instant) -> Result<(), String> {
        self.record(
            at.ns(),
            SentEvent::Continue {
                port: port_name.to_string(),
            },
        )
    }

    fn clock_pulse(&self, port_name: &str, at: Instant) -> Result<(), String> {
        self.record(
            at.ns(),
            SentEvent::ClockPulse {
                port: port_name.to_string(),
            },
        )
    }

    fn flush(&self) -> Result<FlushOutcome, String> {
        Ok(FlushOutcome::AllHandedOver)
    }

    fn remove_queued_events(&self) -> Result<(), String> {
        let now: u64 = self.clock_ns.get();
        for sent in self.sent.borrow_mut().iter_mut() {
            if sent.due_instant_ns > now {
                sent.cancelled = true;
            }
        }
        self.silenced_at_ns.borrow_mut().push(now);
        self.silences.set(self.silences.get() + 1);
        if self.silences.get() >= self.quit_after_silences {
            let _ = self.commands.send(Request::Quit);
        }
        Ok(())
    }

    fn send_note_off_now(&self, port_name: &str, channel: u8, note: u8) -> Result<(), String> {
        self.sent_now.borrow_mut().push((
            self.clock_ns.get(),
            SentNow::NoteOff {
                port: port_name.to_string(),
                channel,
                note,
            },
        ));
        Ok(())
    }

    fn send_stop_now(&self, port_name: &str) -> Result<(), String> {
        self.sent_now.borrow_mut().push((
            self.clock_ns.get(),
            SentNow::Stop {
                port: port_name.to_string(),
            },
        ));
        Ok(())
    }

    fn send_all_notes_off_now(&self, port_name: &str, channel: u8) -> Result<(), String> {
        self.sent_now.borrow_mut().push((
            self.clock_ns.get(),
            SentNow::AllNotesOff {
                port: port_name.to_string(),
                channel,
            },
        ));
        Ok(())
    }
}
