//! The ALSA sequencer client: named output ports, one queue, and the
//! queue's real time as the program's one clock (D-8).
//!
//! The queue has no tempo of its own; events are stamped with an instant
//! in real time and the kernel delivers them on time. See
//! docs/Architecture/Musical Time and Clock.md, section 5.
//!
//! Each output port reaches only what the studio has connected to it (the
//! MIDI router, `aconnect`, or another program), so a Start sent on one
//! port reaches only the devices behind that port (D-10).

use std::ffi::CString;
use std::time::Duration as StdDuration;

use alsa::Direction;
use alsa::seq::{
    Addr, EvCtrl, EvNote, EvQueueControl, Event, EventType, PortCap, PortSubscribe, PortType,
    Remove, RemoveEvents, Seq,
};
use vocabulary::instant::Instant;

use crate::error::AlsaOutputError;
use crate::queue_clock::read_queue_instant;
use crate::queue_timer::{self, QUEUE_TIMER_FILE, QueueTimerReport};
use crate::warning::OutputWarning;

/// MIDI channels are numbered 0 to 15 here (channel 1 is 0).
const LARGEST_CHANNEL: u8 = 15;
/// Note numbers, velocities, controller numbers and values, and program
/// numbers are 7-bit.
const LARGEST_DATA_BYTE: u8 = 127;

/// All Notes Off is controller 123 (MIDI 1.0, Channel Mode Messages).
const ALL_NOTES_OFF_CONTROLLER: u8 = 123;

/// The client's output pool: how many events the kernel holds for us
/// between being handed over and being delivered. Needed:
/// peak events/s × (50 ms lead + 5 ms send loop + 1 ms delivery). ALSA's
/// default (about 500 events) ran out near 9500 events/s, about
/// 22000 q/min of Timing Clock (`control_track_test`, 4 Nov 2026); 2000
/// events covers about 2000 events ÷ 0.056 s ≈ 35000 events/s. A
/// fast-sequencer burst (a sequencer clocked as an oscillator) is a real
/// use, so the headroom is kept.
const OUTPUT_POOL_EVENTS: u32 = 2000;

/// `tv_sec` in ALSA's real-time stamp is 32 bits wide.
const LARGEST_STAMP_SECONDS: u64 = u32::MAX as u64;

/// A MIDI port somewhere else in the sequencer, as listed by `aconnect -l`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiDestination {
    pub client_id: i32,
    pub port_id: i32,
}

/// What [`SequencerOutput::flush`] managed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum FlushOutcome {
    /// Every queued event is now with the kernel.
    AllHandedOver,
    /// The kernel's output pool is full: some events are still in our own
    /// buffer. They are not lost; flush again shortly, and queue nothing
    /// new until a flush hands everything over.
    PoolFull,
}

/// One of our own output ports, as created by [`SequencerOutput::open`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputPort {
    name: String,
    port_id: i32,
}

impl OutputPort {
    /// The name shown by `aconnect -l` and the MIDI router.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The port number, as listed by `aconnect -l` after our client number.
    #[must_use]
    pub fn port_id(&self) -> i32 {
        self.port_id
    }
}

pub struct SequencerOutput {
    seq: Seq,
    own_client_id: i32,
    ports: Vec<OutputPort>,
    queue_id: i32,
}

impl SequencerOutput {
    /// Opens the sequencer, creates one output port for each name in
    /// `port_names` (in that order), creates and starts the queue, and
    /// checks its timer.
    ///
    /// Returns the output and the warnings found while starting. The
    /// caller shows the warnings in the warnings region.
    ///
    /// # Errors
    ///
    /// When the sequencer cannot be opened, a name contains a NUL
    /// character, or a port or the queue cannot be created or started.
    pub fn open(
        client_name: &str,
        port_names: &[&str],
    ) -> Result<(SequencerOutput, Vec<OutputWarning>), AlsaOutputError> {
        let client_name_c: CString = c_string("client name", client_name)?;
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

        let mut warnings: Vec<OutputWarning> = Vec::new();
        if let Err(e) = seq.set_client_pool_output(OUTPUT_POOL_EVENTS) {
            // The program still works with the default pool; only very fast
            // event rates are affected, so this is a warning, not a failure.
            warnings.push(OutputWarning::OutputPoolNotEnlarged {
                requested_events: OUTPUT_POOL_EVENTS,
                reason: e.to_string(),
            });
        }

        let mut ports: Vec<OutputPort> = Vec::with_capacity(port_names.len());
        for port_name in port_names {
            let port_name_c: CString = c_string("port name", port_name)?;
            // READ + SUBS_READ lets other programs (and aconnect) subscribe
            // to this port to receive our events.
            let port_id: i32 = seq
                .create_simple_port(
                    &port_name_c,
                    PortCap::READ | PortCap::SUBS_READ,
                    PortType::MIDI_GENERIC | PortType::APPLICATION,
                )
                .map_err(|e| {
                    AlsaOutputError::alsa(format!("creating the output port {port_name:?}"), e)
                })?;
            ports.push(OutputPort {
                name: (*port_name).to_string(),
                port_id,
            });
        }

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
            ports,
            queue_id,
        };
        warnings.extend(output.check_queue_timer());
        Ok((output, warnings))
    }

    /// The output ports, in the order their names were given to
    /// [`SequencerOutput::open`].
    #[must_use]
    pub fn ports(&self) -> &[OutputPort] {
        &self.ports
    }

    /// The clock: the queue's real time, in nanoseconds since the queue
    /// was created.
    ///
    /// # Errors
    ///
    /// When the queue's status cannot be read.
    pub fn now(&self) -> Result<Instant, AlsaOutputError> {
        read_queue_instant(&self.seq, self.queue_id)
    }

    /// Subscribes `destination` to `port`, so the events sent on `port`
    /// reach it. The program itself rarely needs this: connecting ports is
    /// the studio's job (D-10). Measurement examples use it.
    ///
    /// # Errors
    ///
    /// When the subscription is refused, for example because the
    /// destination does not exist.
    pub fn connect_to(
        &self,
        port: &OutputPort,
        destination: MidiDestination,
    ) -> Result<(), AlsaOutputError> {
        let doing = || {
            format!(
                "connecting our port {:?} ({}:{}) to {}:{}",
                port.name,
                self.own_client_id,
                port.port_id,
                destination.client_id,
                destination.port_id
            )
        };
        let subscription: PortSubscribe =
            PortSubscribe::empty().map_err(|e| AlsaOutputError::alsa(doing(), e))?;
        subscription.set_sender(Addr {
            client: self.own_client_id,
            port: port.port_id,
        });
        subscription.set_dest(Addr {
            client: destination.client_id,
            port: destination.port_id,
        });
        self.seq
            .subscribe_port(&subscription)
            .map_err(|e| AlsaOutputError::alsa(doing(), e))
    }

    /// Queues a note-on on `port`, to sound at `at`. Call
    /// [`SequencerOutput::flush`] to hand queued events to the kernel.
    ///
    /// # Errors
    ///
    /// When a value is outside the MIDI range, or the queue refuses the event.
    pub fn queue_note_on(
        &self,
        port: &OutputPort,
        at: Instant,
        channel: u8,
        note: u8,
        velocity: u8,
    ) -> Result<(), AlsaOutputError> {
        check_midi_value("velocity", velocity, LARGEST_DATA_BYTE)?;
        let event: Event<'static> = note_event(EventType::Noteon, channel, note, velocity)?;
        self.stamp_and_queue(
            port,
            event,
            at,
            &format!("note on (channel {channel}, note {note}, velocity {velocity})"),
        )
    }

    /// Queues a note-off on `port`, to sound at `at`.
    ///
    /// # Errors
    ///
    /// As for [`SequencerOutput::queue_note_on`].
    pub fn queue_note_off(
        &self,
        port: &OutputPort,
        at: Instant,
        channel: u8,
        note: u8,
    ) -> Result<(), AlsaOutputError> {
        let event: Event<'static> = note_event(EventType::Noteoff, channel, note, 0)?;
        self.stamp_and_queue(
            port,
            event,
            at,
            &format!("note off (channel {channel}, note {note})"),
        )
    }

    /// Queues a control change on `port`, to be sent at `at`.
    ///
    /// # Errors
    ///
    /// When a value is outside the MIDI range, or the queue refuses the event.
    pub fn queue_control_change(
        &self,
        port: &OutputPort,
        at: Instant,
        channel: u8,
        controller: u8,
        value: u8,
    ) -> Result<(), AlsaOutputError> {
        let event: Event<'static> = control_change_event(channel, controller, value)?;
        self.stamp_and_queue(
            port,
            event,
            at,
            &format!("control change (channel {channel}, controller {controller}, value {value})"),
        )
    }

    /// Queues a program change on `port`, to be sent at `at`. `program` is
    /// the 7-bit value on the wire (0 to 127; many devices show it as 1 to
    /// 128).
    ///
    /// # Errors
    ///
    /// When a value is outside the MIDI range, or the queue refuses the event.
    pub fn queue_program_change(
        &self,
        port: &OutputPort,
        at: Instant,
        channel: u8,
        program: u8,
    ) -> Result<(), AlsaOutputError> {
        check_midi_value("channel", channel, LARGEST_CHANNEL)?;
        check_midi_value("program number", program, LARGEST_DATA_BYTE)?;
        let event: Event<'static> = Event::new(
            EventType::Pgmchange,
            &EvCtrl {
                channel,
                param: 0,
                value: i32::from(program),
            },
        );
        self.stamp_and_queue(
            port,
            event,
            at,
            &format!("program change (channel {channel}, program {program})"),
        )
    }

    /// Queues MIDI Start (FA) on `port`, to be sent at `at`.
    ///
    /// # Errors
    ///
    /// When the queue refuses the event.
    pub fn queue_start(&self, port: &OutputPort, at: Instant) -> Result<(), AlsaOutputError> {
        self.stamp_and_queue(port, self.transport_event(EventType::Start), at, "Start")
    }

    /// Queues MIDI Stop (FC) on `port`, to be sent at `at`.
    ///
    /// # Errors
    ///
    /// When the queue refuses the event.
    pub fn queue_stop(&self, port: &OutputPort, at: Instant) -> Result<(), AlsaOutputError> {
        self.stamp_and_queue(port, self.transport_event(EventType::Stop), at, "Stop")
    }

    /// Queues MIDI Continue (FB) on `port`, to be sent at `at`.
    ///
    /// # Errors
    ///
    /// When the queue refuses the event.
    pub fn queue_continue(&self, port: &OutputPort, at: Instant) -> Result<(), AlsaOutputError> {
        self.stamp_and_queue(
            port,
            self.transport_event(EventType::Continue),
            at,
            "Continue",
        )
    }

    /// Queues a Timing Clock (F8) on `port`, to be sent at `at`. `tag`
    /// rides along unchanged to the receiver, so a measuring program can
    /// tell which pulse arrived (it wraps every 256 pulses).
    ///
    /// # Errors
    ///
    /// When the queue refuses the event.
    pub fn queue_clock_pulse(
        &self,
        port: &OutputPort,
        at: Instant,
        tag: u8,
    ) -> Result<(), AlsaOutputError> {
        let mut event: Event<'static> = self.transport_event(EventType::Clock);
        event.set_tag(tag);
        self.stamp_and_queue(port, event, at, &format!("clock pulse (tag {tag})"))
    }

    /// Hands queued events to the kernel. A full output pool is not an
    /// error: alsa-lib's non-blocking pattern is to try again shortly
    /// (`snd_seq_drain_output` returns -EAGAIN, or the bytes still waiting);
    /// the events wait in our own buffer meanwhile.
    ///
    /// # Errors
    ///
    /// When the kernel refuses the events for any other reason.
    pub fn flush(&self) -> Result<FlushOutcome, AlsaOutputError> {
        match self.seq.drain_output() {
            Ok(0) => Ok(FlushOutcome::AllHandedOver),
            // Bytes still waiting in our buffer.
            Ok(_) => Ok(FlushOutcome::PoolFull),
            Err(e) if is_pool_full(&e) => Ok(FlushOutcome::PoolFull),
            Err(e) => Err(AlsaOutputError::alsa(
                "handing queued events to the kernel",
                e,
            )),
        }
    }

    /// Removes every event this client has queued and not yet delivered:
    /// those still in our own output buffer, and those already handed to
    /// the kernel's queue. Step 1 of Stop (Musical Time and Clock,
    /// section 8).
    ///
    /// # Errors
    ///
    /// When the kernel refuses the removal.
    pub fn remove_queued_events(&self) -> Result<(), AlsaOutputError> {
        let doing: String = format!("removing the events queued on queue {}", self.queue_id);
        // Events not yet handed to the kernel.
        self.seq
            .drop_output()
            .map_err(|e| AlsaOutputError::alsa(doing.clone(), e))?;
        // Events in the kernel's queue: OUTPUT selects the events this
        // client sent, on our queue.
        let removal: RemoveEvents =
            RemoveEvents::new().map_err(|e| AlsaOutputError::alsa(doing.clone(), e))?;
        removal.set_condition(Remove::OUTPUT);
        removal.set_queue(self.queue_id);
        self.seq
            .remove_events(removal)
            .map_err(|e| AlsaOutputError::alsa(doing, e))
    }

    /// Sends a note-off on `port` immediately, bypassing the queue. For
    /// the notes the player knows are sounding when it stops.
    ///
    /// # Errors
    ///
    /// When a value is outside the MIDI range, or the kernel refuses the
    /// event.
    pub fn send_note_off_now(
        &self,
        port: &OutputPort,
        channel: u8,
        note: u8,
    ) -> Result<(), AlsaOutputError> {
        let event: Event<'static> = note_event(EventType::Noteoff, channel, note, 0)?;
        self.send_now(
            port,
            event,
            &format!("note off (channel {channel}, note {note})"),
        )
    }

    /// Sends MIDI Stop (FC) on `port` immediately, bypassing the queue.
    ///
    /// # Errors
    ///
    /// When the kernel refuses the event.
    pub fn send_stop_now(&self, port: &OutputPort) -> Result<(), AlsaOutputError> {
        self.send_now(port, self.transport_event(EventType::Stop), "Stop")
    }

    /// Sends All Notes Off (controller 123) on `port` and `channel`
    /// immediately, bypassing the queue. The final safety net of Stop.
    ///
    /// # Errors
    ///
    /// When the channel is outside the MIDI range, or the kernel refuses
    /// the event.
    pub fn send_all_notes_off_now(
        &self,
        port: &OutputPort,
        channel: u8,
    ) -> Result<(), AlsaOutputError> {
        let event: Event<'static> = control_change_event(channel, ALL_NOTES_OFF_CONTROLLER, 0)?;
        self.send_now(port, event, &format!("All Notes Off (channel {channel})"))
    }

    /// Sends a Timing Clock (F8) on `port` immediately, bypassing the
    /// queue's timer. For a sender that does its own waiting (the deadline
    /// fallback in D-8); the caller says when by calling this then.
    ///
    /// # Errors
    ///
    /// When the kernel refuses the event.
    pub fn send_clock_pulse_now(&self, port: &OutputPort, tag: u8) -> Result<(), AlsaOutputError> {
        let mut event: Event<'static> = self.transport_event(EventType::Clock);
        event.set_tag(tag);
        self.send_now(port, event, &format!("clock pulse (tag {tag})"))
    }

    /// The queue number, so another client can stamp arrivals with this
    /// queue's real time (the screen, D-8, and measurement).
    #[must_use]
    pub fn queue_id(&self) -> i32 {
        self.queue_id
    }

    /// This client's number, as listed by `aconnect -l`.
    #[must_use]
    pub fn client_id(&self) -> i32 {
        self.own_client_id
    }

    /// A System Real-Time event: Timing Clock, Start, Stop or Continue.
    /// ALSA carries these with a queue-control payload; the queue number in
    /// it is not used by a MIDI receiver.
    fn transport_event(&self, event_type: EventType) -> Event<'static> {
        Event::new(
            event_type,
            &EvQueueControl {
                queue: self.queue_id,
                value: (),
            },
        )
    }

    fn stamp_and_queue(
        &self,
        port: &OutputPort,
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
        event.set_source(port.port_id);
        event.set_subs();
        event.schedule_real(self.queue_id, false, stamp);
        self.seq.event_output(&mut event).map(|_| ()).map_err(|e| {
            let doing: String =
                format!("queueing {what} on port {:?} at {} ns", port.name, at.ns());
            // Our own buffer is full only when events keep being queued while
            // flush reports PoolFull; callers must stop queuing then.
            if is_pool_full(&e) {
                AlsaOutputError::OutputBufferFull { doing }
            } else {
                AlsaOutputError::alsa(doing, e)
            }
        })
    }

    fn send_now(
        &self,
        port: &OutputPort,
        mut event: Event<'static>,
        what: &str,
    ) -> Result<(), AlsaOutputError> {
        event.set_source(port.port_id);
        event.set_subs();
        event.set_direct();
        self.seq
            .event_output_direct(&mut event)
            .map(|_| ())
            .map_err(|e| {
                AlsaOutputError::alsa(format!("sending {what} now on port {:?}", port.name), e)
            })
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

/// EAGAIN ("try again"): the sequencer, opened non-blocking, cannot take
/// more events now.
fn is_pool_full(error: &alsa::Error) -> bool {
    std::io::Error::from_raw_os_error(error.errno()).kind() == std::io::ErrorKind::WouldBlock
}

fn note_event(
    event_type: EventType,
    channel: u8,
    note: u8,
    velocity: u8,
) -> Result<Event<'static>, AlsaOutputError> {
    check_midi_value("channel", channel, LARGEST_CHANNEL)?;
    check_midi_value("note number", note, LARGEST_DATA_BYTE)?;
    Ok(Event::new(
        event_type,
        &EvNote {
            channel,
            note,
            velocity,
            off_velocity: 0,
            duration: 0,
        },
    ))
}

fn control_change_event(
    channel: u8,
    controller: u8,
    value: u8,
) -> Result<Event<'static>, AlsaOutputError> {
    check_midi_value("channel", channel, LARGEST_CHANNEL)?;
    check_midi_value("controller number", controller, LARGEST_DATA_BYTE)?;
    check_midi_value("controller value", value, LARGEST_DATA_BYTE)?;
    Ok(Event::new(
        EventType::Controller,
        &EvCtrl {
            channel,
            param: u32::from(controller),
            value: i32::from(value),
        },
    ))
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
