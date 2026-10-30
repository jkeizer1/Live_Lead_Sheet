//! Shared by the delivery measurements (`delivery_spread`, `deadline_spread`):
//! a second ALSA client receives the clock pulses, the kernel stamps each
//! arrival with the queue's real time (so the measuring program's own
//! scheduling does not distort the result: the method of the Linux MIDI
//! Router's spike 1), and the delay of every pulse is reported:
//!
//!     delay_ns = arrival instant_ns − scheduled instant_ns
//!
//! The spread of the delay (p99 − minimum) is the jitter of the sender.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration as StdDuration, Instant as StdInstant};

use alsa::Direction;
use alsa::seq::{EventType, PortCap, PortInfo, PortType, Seq};
use alsa_output::sequencer_output::{MidiDestination, SequencerOutput};
use alsa_output::warning::OutputWarning;
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;

/// 5040 ticks/q ÷ 24 pulses/q = 210 ticks/pulse.
const TICKS_PER_PULSE: u64 = 210;
const NANOSECONDS_PER_MINUTE: u128 = 60_000_000_000;
const MILLI_QUARTERS_PER_QUARTER_NOTE: u128 = 1000;
pub const FIRST_PULSE_DELAY: Duration = Duration::from_ms(1000);
/// The receiver gives up when no pulse has arrived for this long.
const RECEIVER_PATIENCE: StdDuration = StdDuration::from_secs(2);
const RECEIVER_POLL: StdDuration = StdDuration::from_millis(1);

pub struct Settings {
    pub quarter_notes_per_minute: u64,
    pub pulses: u64,
    pub load_threads: u64,
    /// Final stretch before a deadline spent busy-waiting instead of
    /// sleeping; zero for senders that do not wait themselves.
    pub spin: Duration,
}

struct Arrival {
    tag: u8,
    arrival: Instant,
}

/// Opens the output and a receiving client, connects them, starts the load,
/// lets `send` send every pulse, and reports. `send` returns the instant
/// each pulse was meant to be delivered, in order.
pub fn run_measurement<F>(client_name: &str, settings: &Settings, send: F) -> Result<(), String>
where
    F: FnOnce(&SequencerOutput, &Settings) -> Result<Vec<Instant>, String>,
{
    let (output, warnings): (SequencerOutput, Vec<OutputWarning>) =
        SequencerOutput::open(client_name).map_err(|e| e.to_string())?;
    for warning in &warnings {
        eprintln!("WARNING: {warning}");
    }

    let (ready_sender, ready_receiver) = mpsc::channel::<Result<MidiDestination, String>>();
    let queue_id: i32 = output.queue_id();
    let pulses: u64 = settings.pulses;
    let receiver_thread: thread::JoinHandle<Result<Vec<Arrival>, String>> =
        thread::spawn(move || receive_pulses(queue_id, pulses, &ready_sender));
    let destination: MidiDestination = ready_receiver
        .recv()
        .map_err(|_| "The receiving client stopped before it was ready.".to_string())??;
    output.connect_to(destination).map_err(|e| e.to_string())?;

    let stop_load: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let load: Vec<thread::JoinHandle<()>> = start_load(settings.load_threads, &stop_load);

    let send_result: Result<Vec<Instant>, String> = send(&output, settings);

    // Stop the load before reporting, whatever happened.
    stop_load.store(true, Ordering::Relaxed);
    for handle in load {
        handle
            .join()
            .map_err(|_| "A load thread panicked.".to_string())?;
    }
    let scheduled: Vec<Instant> = send_result?;
    let arrivals: Vec<Arrival> = receiver_thread
        .join()
        .map_err(|_| "The receiving client panicked.".to_string())??;
    report(settings, &scheduled, &arrivals)
}

/// T = T₀ + P × 60000000000 ns/min × 1000 mq/q ÷ (B × 5040 ticks/q), with
/// P = `pulse_number` × 210 ticks/pulse × 1 pulse; ticks, mq and q cancel,
/// min cancels, leaving ns. Multiply first, divide once, round to nearest ns.
pub fn pulse_instant(
    first_pulse: Instant,
    pulse_number: u64,
    quarter_notes_per_minute: u64,
) -> Result<Instant, String> {
    let milli_quarters_per_minute: u64 = quarter_notes_per_minute * 1000;
    let position_ticks: u128 = u128::from(pulse_number) * u128::from(TICKS_PER_PULSE);
    let numerator: u128 = position_ticks * NANOSECONDS_PER_MINUTE * MILLI_QUARTERS_PER_QUARTER_NOTE;
    let denominator: u128 =
        u128::from(milli_quarters_per_minute) * u128::from(TICKS_PER_QUARTER_NOTE);
    let offset_ns: u128 = (numerator + denominator / 2) / denominator;
    let offset_ns: u64 = u64::try_from(offset_ns)
        .map_err(|_| format!("Pulse {pulse_number} is too far from the start to time."))?;
    first_pulse
        .checked_add(Duration::from_ns(offset_ns))
        .map_err(|e| e.to_string())
}

/// The tag a pulse carries, so the receiver can check none was lost.
pub fn tag_for(pulse_number: u64) -> Result<u8, String> {
    u8::try_from(pulse_number % 256).map_err(|e| e.to_string())
}

/// The second ALSA client: one input port stamping arrivals with the
/// sender's queue time.
fn receive_pulses(
    queue_id: i32,
    pulses_wanted: u64,
    ready: &mpsc::Sender<Result<MidiDestination, String>>,
) -> Result<Vec<Arrival>, String> {
    let seq: Seq = match open_receiving_client(queue_id) {
        Ok((seq, destination)) => {
            // If the main thread is gone there is nobody to measure for.
            ready
                .send(Ok(destination))
                .map_err(|_| "The sender stopped before the receiver was ready.".to_string())?;
            seq
        }
        Err(message) => {
            // The main thread may already have given up; the message goes
            // back through the join result as well.
            let _ = ready.send(Err(message.clone()));
            return Err(message);
        }
    };

    let mut input = seq.input();
    let mut arrivals: Vec<Arrival> = Vec::new();
    let mut last_event: StdInstant = StdInstant::now();
    while (arrivals.len() as u64) < pulses_wanted {
        let pending: u32 = input
            .event_input_pending(true)
            .map_err(|e| format!("Receiver: checking for pulses: {e}"))?;
        if pending == 0 {
            if last_event.elapsed() > RECEIVER_PATIENCE {
                return Err(format!(
                    "Receiver: no pulse for {} s; {} of {pulses_wanted} arrived. \
                     A pulse was lost or the connection failed.",
                    RECEIVER_PATIENCE.as_secs(),
                    arrivals.len()
                ));
            }
            thread::sleep(RECEIVER_POLL);
            continue;
        }
        let event = input
            .event_input()
            .map_err(|e| format!("Receiver: reading pulse {}: {e}", arrivals.len()))?;
        if event.get_type() != EventType::Clock {
            continue;
        }
        last_event = StdInstant::now();
        let Some(stamp) = event.get_time() else {
            return Err(format!(
                "Receiver: pulse {} arrived without a real-time stamp; \
                 port time-stamping is not working.",
                arrivals.len()
            ));
        };
        let arrival_ns: u64 = u64::try_from(stamp.as_nanos())
            .map_err(|_| format!("Receiver: arrival stamp {stamp:?} does not fit 64 bits of ns"))?;
        arrivals.push(Arrival {
            tag: event.get_tag(),
            arrival: Instant::from_ns(arrival_ns),
        });
    }
    Ok(arrivals)
}

fn open_receiving_client(queue_id: i32) -> Result<(Seq, MidiDestination), String> {
    let seq: Seq = Seq::open(None, Some(Direction::Capture), true)
        .map_err(|e| format!("Receiver: opening the ALSA sequencer: {e}"))?;
    let client_name: CString =
        CString::new("Live Lead Sheet receiver").map_err(|e| e.to_string())?;
    seq.set_client_name(&client_name)
        .map_err(|e| format!("Receiver: naming the client: {e}"))?;
    let mut port: PortInfo =
        PortInfo::empty().map_err(|e| format!("Receiver: preparing the port: {e}"))?;
    let port_name: CString = CString::new("In").map_err(|e| e.to_string())?;
    port.set_name(&port_name);
    port.set_capability(PortCap::WRITE | PortCap::SUBS_WRITE);
    port.set_type(PortType::MIDI_GENERIC | PortType::APPLICATION);
    port.set_timestamping(true);
    port.set_timestamp_real(true);
    port.set_timestamp_queue(queue_id);
    seq.create_port(&port)
        .map_err(|e| format!("Receiver: creating the input port on queue {queue_id}: {e}"))?;
    let client_id: i32 = seq
        .client_id()
        .map_err(|e| format!("Receiver: reading our client number: {e}"))?;
    Ok((
        seq,
        MidiDestination {
            client_id,
            port_id: port.get_port(),
        },
    ))
}

/// Busy threads that compete for the CPU while the pulses are measured.
fn start_load(threads: u64, stop: &Arc<AtomicBool>) -> Vec<thread::JoinHandle<()>> {
    (0..threads)
        .map(|_| {
            let stop: Arc<AtomicBool> = Arc::clone(stop);
            thread::spawn(move || {
                let mut counter: u64 = 0;
                while !stop.load(Ordering::Relaxed) {
                    counter = std::hint::black_box(counter.wrapping_add(1));
                }
            })
        })
        .collect()
}

fn report(settings: &Settings, scheduled: &[Instant], arrivals: &[Arrival]) -> Result<(), String> {
    let mut delays_ns: Vec<i128> = Vec::new();
    for (pulse_number, (instant, arrival)) in scheduled.iter().zip(arrivals).enumerate() {
        let expected_tag: u8 = tag_for(pulse_number as u64)?;
        if arrival.tag != expected_tag {
            return Err(format!(
                "Pulse {pulse_number} arrived with tag {}, expected {expected_tag}: \
                 a pulse was lost or reordered.",
                arrival.tag
            ));
        }
        delays_ns.push(i128::from(arrival.arrival.ns()) - i128::from(instant.ns()));
    }
    let mut intervals_ns: Vec<i128> = arrivals
        .windows(2)
        .map(|pair| i128::from(pair[1].arrival.ns()) - i128::from(pair[0].arrival.ns()))
        .collect();
    delays_ns.sort_unstable();
    intervals_ns.sort_unstable();

    let smallest: i128 = delays_ns.first().copied().ok_or("No pulses measured.")?;
    let p99: i128 = percentile(&delays_ns, 990);
    println!(
        "{} pulses at {} q/min, {} load threads, final {} µs busy-waited",
        arrivals.len(),
        settings.quarter_notes_per_minute,
        settings.load_threads,
        settings.spin.ns() / 1000
    );
    println!("Delay of delivery after the scheduled instant");
    println!("  minimum            {}", ms(smallest));
    println!("  median (p50)       {}", ms(percentile(&delays_ns, 500)));
    println!("  p99                {}", ms(p99));
    println!("  p99.9              {}", ms(percentile(&delays_ns, 999)));
    println!(
        "  maximum            {}",
        ms(delays_ns[delays_ns.len() - 1])
    );
    println!("  jitter (p99 - min) {}", ms(p99 - smallest));
    if let (Some(shortest), Some(longest)) = (intervals_ns.first(), intervals_ns.last()) {
        println!(
            "Interval between pulses as received: {} to {}",
            ms(*shortest),
            ms(*longest)
        );
    }
    Ok(())
}

/// `permille` is thousandths: 990 is p99. Index rounded to nearest.
fn percentile(sorted_ns: &[i128], permille: usize) -> i128 {
    let index: usize = ((sorted_ns.len() - 1) * permille + 500) / 1000;
    sorted_ns[index]
}

fn ms(value_ns: i128) -> String {
    let sign: &str = if value_ns < 0 { "-" } else { "" };
    let magnitude_ns: i128 = value_ns.abs();
    format!(
        "{sign}{}.{:03} ms",
        magnitude_ns / 1_000_000,
        (magnitude_ns % 1_000_000) / 1000
    )
}

/// `--bpm N`, `--pulses N`, `--load N`, and (only when `allow_spin`)
/// `--spin-us N`: whole numbers.
pub fn parse_settings(allow_spin: bool) -> Result<Settings, String> {
    let mut settings: Settings = Settings {
        quarter_notes_per_minute: 120,
        pulses: 3000,
        load_threads: 0,
        spin: Duration::ZERO,
    };
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut remaining = arguments.iter();
    while let Some(name) = remaining.next() {
        let value: &String = remaining
            .next()
            .ok_or_else(|| format!("{name} needs a number after it."))?;
        let number: u64 = value
            .parse()
            .map_err(|_| format!("{name} needs a whole number, got {value:?}."))?;
        match name.as_str() {
            "--bpm" => settings.quarter_notes_per_minute = number,
            "--pulses" => settings.pulses = number,
            "--load" => settings.load_threads = number,
            "--spin-us" if allow_spin => settings.spin = Duration::from_ns(number * 1000),
            _ => return Err(format!("Unknown option {name}.")),
        }
    }
    if settings.quarter_notes_per_minute == 0 || settings.pulses < 10 {
        return Err("--bpm must be at least 1 and --pulses at least 10.".to_string());
    }
    Ok(settings)
}
