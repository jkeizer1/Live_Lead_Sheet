//! The ALSA sequencer client: one output port, one queue, and the queue's
//! real time as the program's one clock (D-8).
//!
//! The queue has no tempo of its own; events are stamped with an instant
//! in real time and the kernel delivers them on time. See
//! docs/Architecture/Musical Time and Clock.md, section 5.

use std::ffi::CString;
use std::time::Duration as StdDuration;

use alsa::Direction;
use alsa::seq::{
    Addr, EvNote, EvQueueControl, Event, EventType, PortCap, PortSubscribe, PortType, QueueStatus,
    Seq,
};
use vocabulary::instant::Instant;

use crate::error::AlsaOutputError;
use crate::queue_timer::{self, QUEUE_TIMER_FILE, QueueTimerReport};
use crate::warning::OutputWarning;

/// MIDI channels are numbered 0 to 15 here (channel 1 is 0).
const LARGEST_CHANNEL: u8 = 15;
/// Note numbers and velocities are 7-bit.
const LARGEST_DATA_BYTE: u8 = 127;

/// `tv_sec` in ALSA's real-time stamp is 32 bits wide.
const LARGEST_STAMP_SECONDS: u64 = u32::MAX as u64;

/// A MIDI port somewhere else in the sequencer, as listed by `aconnect -l`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiDestination {
    pub client_id: i32,
    pub port_id: i32,
}

pub struct SequencerOutput {
    seq: Seq,
    own_client_id: i32,
    port_id: i32,
    queue_id: i32,
}

impl SequencerOutput {
    /// Opens the sequencer, creates the output port and the queue, starts
    /// the queue, and checks its timer.
    ///
    /// Returns the output and the warnings found while starting. The
    /// caller shows the warnings in the warnings region.
    ///
    /// # Errors
    ///
    /// When the sequencer cannot be opened, or the port or queue cannot be
    /// created or started.
    pub fn open(
        client_name: &str,
    ) -> Result<(SequencerOutput, Vec<OutputWarning>), AlsaOutputError> {
        let client_name_c: CString = c_string("client name", client_name)?;
        let port_name_c: CString = c_string("port name", "Out")?;
        let queue_name_c: CString = c_string("queue name", client_name)?;

        // Non-blocking: a full event pool is reported as an error instead of
        // stalling the player.
        let seq: Seq = Seq::open(None, Some(Direction::Playback), true)
            .map_err(|e| AlsaOutputError::alsa("opening the ALSA sequencer", e))?;
        seq.set_client_name(&client_name_c).map_err(|e| {
            AlsaOutputError::alsa(format!("naming the sequencer client {client_name:?}"), e)
        })?;
        let own_client_id: i32 = seq
            .client_id()
            .map_err(|e| AlsaOutputError::alsa("reading our sequencer client number", e))?;
        // READ + SUBS_READ lets other programs (and aconnect) subscribe to
        // this port to receive our events.
        let port_id: i32 = seq
            .create_simple_port(
                &port_name_c,
                PortCap::READ | PortCap::SUBS_READ,
                PortType::MIDI_GENERIC | PortType::APPLICATION,
            )
            .map_err(|e| AlsaOutputError::alsa("creating the MIDI output port", e))?;
        let queue_id: i32 = seq
            .alloc_named_queue(&queue_name_c)
            .map_err(|e| AlsaOutputError::alsa("creating the event queue", e))?;
        seq.control_queue(queue_id, EventType::Start, 0, None)
            .map_err(|e| AlsaOutputError::alsa(format!("starting queue {queue_id}"), e))?;
        seq.drain_output()
            .map_err(|e| AlsaOutputError::alsa(format!("starting queue {queue_id}"), e))?;

        let output: SequencerOutput = SequencerOutput {
            seq,
            own_client_id,
            port_id,
            queue_id,
        };
        let warnings: Vec<OutputWarning> = output.check_queue_timer();
        Ok((output, warnings))
    }

    /// The clock: the queue's real time, in nanoseconds since the queue
    /// was created.
    ///
    /// # Errors
    ///
    /// When the queue's status cannot be read.
    pub fn now(&self) -> Result<Instant, AlsaOutputError> {
        let status: QueueStatus = self
            .seq
            .get_queue_status(self.queue_id)
            .map_err(|e| AlsaOutputError::alsa("reading the queue's real time", e))?;
        let real_time: StdDuration = status.get_real_time();
        // as_nanos is u128 ns; it must fit the u64 ns of an Instant.
        match u64::try_from(real_time.as_nanos()) {
            Ok(instant_ns) => Ok(Instant::from_ns(instant_ns)),
            Err(_) => Err(AlsaOutputError::ClockOutOfRange {
                seconds: real_time.as_secs(),
                nanoseconds: real_time.subsec_nanos(),
            }),
        }
    }

    /// Subscribes `destination` to our port, so our events reach it.
    ///
    /// # Errors
    ///
    /// When the subscription is refused, for example because the
    /// destination does not exist.
    pub fn connect_to(&self, destination: MidiDestination) -> Result<(), AlsaOutputError> {
        let doing = || {
            format!(
                "connecting our port {}:{} to {}:{}",
                self.own_client_id, self.port_id, destination.client_id, destination.port_id
            )
        };
        let subscription: PortSubscribe =
            PortSubscribe::empty().map_err(|e| AlsaOutputError::alsa(doing(), e))?;
        subscription.set_sender(Addr {
            client: self.own_client_id,
            port: self.port_id,
        });
        subscription.set_dest(Addr {
            client: destination.client_id,
            port: destination.port_id,
        });
        self.seq
            .subscribe_port(&subscription)
            .map_err(|e| AlsaOutputError::alsa(doing(), e))
    }

    /// Queues a note-on to every subscriber, to sound at `at`. Call
    /// [`SequencerOutput::flush`] to hand queued events to the kernel.
    ///
    /// # Errors
    ///
    /// When a value is outside the MIDI range, or the queue refuses the event.
    pub fn queue_note_on(
        &self,
        at: Instant,
        channel: u8,
        note: u8,
        velocity: u8,
    ) -> Result<(), AlsaOutputError> {
        self.queue_note(EventType::Noteon, at, channel, note, velocity)
    }

    /// Queues a note-off to every subscriber, to sound at `at`.
    ///
    /// # Errors
    ///
    /// As for [`SequencerOutput::queue_note_on`].
    pub fn queue_note_off(
        &self,
        at: Instant,
        channel: u8,
        note: u8,
    ) -> Result<(), AlsaOutputError> {
        self.queue_note(EventType::Noteoff, at, channel, note, 0)
    }

    /// Hands every queued event to the kernel.
    ///
    /// # Errors
    ///
    /// When the kernel does not accept them.
    pub fn flush(&self) -> Result<(), AlsaOutputError> {
        self.seq
            .drain_output()
            .map(|_| ())
            .map_err(|e| AlsaOutputError::alsa("handing queued events to the kernel", e))
    }

    fn queue_note(
        &self,
        event_type: EventType,
        at: Instant,
        channel: u8,
        note: u8,
        velocity: u8,
    ) -> Result<(), AlsaOutputError> {
        check_midi_value("channel", channel, LARGEST_CHANNEL)?;
        check_midi_value("note number", note, LARGEST_DATA_BYTE)?;
        check_midi_value("velocity", velocity, LARGEST_DATA_BYTE)?;
        let event: Event<'static> = Event::new(
            event_type,
            &EvNote {
                channel,
                note,
                velocity,
                off_velocity: 0,
                duration: 0,
            },
        );
        self.stamp_and_queue(
            event,
            at,
            &format!("{event_type:?} (channel {channel}, note {note}, velocity {velocity})"),
        )
    }

    /// Queues a MIDI clock pulse to every subscriber, to be sent at `at`.
    /// `tag` rides along unchanged to the receiver, so a measuring program
    /// can tell which pulse arrived (it wraps every 256 pulses).
    ///
    /// # Errors
    ///
    /// When the queue refuses the event.
    pub fn queue_clock_pulse(&self, at: Instant, tag: u8) -> Result<(), AlsaOutputError> {
        self.stamp_and_queue(
            self.clock_pulse_event(tag),
            at,
            &format!("clock pulse (tag {tag})"),
        )
    }

    /// Sends a MIDI clock pulse to every subscriber immediately, bypassing
    /// the queue's timer. For a sender that does its own waiting (the
    /// deadline fallback in D-8); the caller says when by calling this then.
    ///
    /// # Errors
    ///
    /// When the kernel refuses the event.
    pub fn send_clock_pulse_now(&self, tag: u8) -> Result<(), AlsaOutputError> {
        let mut event: Event<'static> = self.clock_pulse_event(tag);
        event.set_source(self.port_id);
        event.set_subs();
        event.set_direct();
        self.seq
            .event_output_direct(&mut event)
            .map(|_| ())
            .map_err(|e| AlsaOutputError::alsa(format!("sending clock pulse (tag {tag}) now"), e))
    }

    fn clock_pulse_event(&self, tag: u8) -> Event<'static> {
        // ALSA's clock event is carried with a queue-control payload; the
        // queue number in it is not used by a MIDI receiver.
        let mut event: Event<'static> = Event::new(
            EventType::Clock,
            &EvQueueControl {
                queue: self.queue_id,
                value: (),
            },
        );
        event.set_tag(tag);
        event
    }

    /// The queue number, so another client can stamp arrivals with this
    /// queue's real time (measurement only).
    #[must_use]
    pub fn queue_id(&self) -> i32 {
        self.queue_id
    }

    /// This client's number, as listed by `aconnect -l`.
    #[must_use]
    pub fn client_id(&self) -> i32 {
        self.own_client_id
    }

    fn stamp_and_queue(
        &self,
        mut event: Event<'static>,
        at: Instant,
        what: &str,
    ) -> Result<(), AlsaOutputError> {
        let stamp: StdDuration = StdDuration::from_nanos(at.ns());
        if stamp.as_secs() > LARGEST_STAMP_SECONDS {
            return Err(AlsaOutputError::InstantOutOfRange {
                instant_ns: at.ns(),
            });
        }
        event.set_source(self.port_id);
        event.set_subs();
        event.schedule_real(self.queue_id, false, stamp);
        self.seq
            .event_output(&mut event)
            .map(|_| ())
            .map_err(|e| AlsaOutputError::alsa(format!("queueing {what} at {} ns", at.ns()), e))
    }

    fn check_queue_timer(&self) -> Vec<OutputWarning> {
        let timer_file_text: String = match std::fs::read_to_string(QUEUE_TIMER_FILE) {
            Ok(text) => text,
            Err(e) => {
                return vec![OutputWarning::QueueTimerFileUnreadable {
                    file: QUEUE_TIMER_FILE.to_string(),
                    reason: e.to_string(),
                }];
            }
        };
        match queue_timer::report_for_queue(&timer_file_text, self.queue_id) {
            QueueTimerReport::HrTimer => Vec::new(),
            QueueTimerReport::OtherTimer { timer_name } => vec![OutputWarning::QueueTimerNotHr {
                queue_id: self.queue_id,
                timer_name,
            }],
            QueueTimerReport::QueueNotListed => vec![OutputWarning::QueueTimerNotListed {
                queue_id: self.queue_id,
            }],
        }
    }
}

fn c_string(what: &'static str, name: &str) -> Result<CString, AlsaOutputError> {
    CString::new(name).map_err(|_| AlsaOutputError::NameHasNul {
        what,
        name: name.to_string(),
    })
}

fn check_midi_value(what: &'static str, value: u8, largest: u8) -> Result<(), AlsaOutputError> {
    if value > largest {
        return Err(AlsaOutputError::MidiValueOutOfRange {
            what,
            value,
            largest,
        });
    }
    Ok(())
}
