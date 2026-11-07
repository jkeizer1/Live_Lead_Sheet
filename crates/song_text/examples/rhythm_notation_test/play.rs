//! The play half of the test: a chosen rhythm as a mono voice playing one
//! fixed note on the "Beat pulse" port, with Timing Clock on its own port,
//! for listening on the FH-2. The sending loop is the shared
//! `support/event_player.rs`.

use player::note_track::NoteTrack;
use player::schedule::{EventSource, OutputAction, ScheduledEvent, schedule};
use player::studio_configuration::VelocityLevels;
use song_text::length_words::describe_place;
use song_text::rhythm_library::RhythmLibraryEntry;
use vocabulary::control_track::ControlTrack;
use vocabulary::midi_values::{MidiChannel, NoteNumber, Velocity};
use vocabulary::phrase_cue::{ControlCueMessage, OutputPortName, PhraseCue, TransportMessage};
use vocabulary::score_length::ScoreLength;
use vocabulary::score_position::ScorePosition;
use vocabulary::tempo::Tempo;
use vocabulary::time_signature::TimeSignature;

use crate::event_player;

const CLOCK_PORT: &str = "Clock";
const PULSE_PORT: &str = "Beat pulse";

/// The velocities this test gives the levels of the rhythm's marks: `>` and
/// `>>` both send 127. (A studio configuration would give them per track.)
const ACCENT_VELOCITY: u8 = 127;
const NORMAL_VELOCITY: u8 = 64;

/// What to play, from the command line.
pub struct PlayRequest {
    rhythm_name: String,
    quarter_notes_per_minute: u64,
    bars: u64,
    channel: MidiChannel,
    note: NoteNumber,
    /// Start at the first note and Stop at the end, on the clock port.
    transport: bool,
}

/// The library file and, when `--play` is given, what to play.
///
/// # Errors
///
/// A message saying which option is wrong and what to type instead.
pub fn parse_arguments(
    arguments: &[String],
    default_library_file: &str,
) -> Result<(String, Option<PlayRequest>), String> {
    let mut library_file: String = default_library_file.to_string();
    let mut rhythm_name: Option<String> = None;
    let mut quarter_notes_per_minute: u64 = 120;
    let mut bars: u64 = 8;
    let mut channel: u8 = 15;
    let mut note: u8 = 60;
    let mut transport: bool = false;
    let mut index: usize = 0;
    while index < arguments.len() {
        let option: &str = arguments[index].as_str();
        if !option.starts_with("--") {
            library_file = option.to_string();
            index += 1;
            continue;
        }
        // The one option without a value.
        if option == "--transport" {
            transport = true;
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
            // Spaces at either end come from typing or tab completion.
            "--play" => rhythm_name = Some(value.trim().to_string()),
            "--bpm" => quarter_notes_per_minute = number()?,
            "--bars" => bars = number()?,
            "--channel" => channel = byte()?,
            "--note" => note = byte()?,
            _ => {
                return Err(format!(
                    "Unknown option {option}. Options: --play \"Name\" --bpm --bars \
                     --channel --note --transport"
                ));
            }
        }
        index += 2;
    }
    if bars == 0 {
        return Err(String::from("--bars 0: use 1 or more."));
    }
    let request: Option<PlayRequest> = match rhythm_name {
        None => None,
        Some(rhythm_name) => Some(PlayRequest {
            rhythm_name,
            quarter_notes_per_minute,
            bars,
            channel: MidiChannel::new(channel).map_err(|e| format!("--channel: {e}"))?,
            note: NoteNumber::new(note).map_err(|e| format!("--note: {e}"))?,
            transport,
        }),
    };
    Ok((library_file, request))
}

/// Plays the rhythm, repeated to fill `request.bars` bars.
///
/// # Errors
///
/// When the rhythm is not in the library, or the output or the schedule
/// fails; the message says what was being done.
pub fn play_rhythm(entries: &[RhythmLibraryEntry], request: &PlayRequest) -> Result<(), String> {
    let Some(entry) = entries
        .iter()
        .find(|entry: &&RhythmLibraryEntry| entry.pattern().name() == request.rhythm_name)
    else {
        let names: Vec<&str> = entries
            .iter()
            .map(|entry: &RhythmLibraryEntry| entry.pattern().name())
            .collect();
        return Err(format!(
            "No rhythm named {:?} in the library. Choose one of: {}.",
            request.rhythm_name,
            names.join(", ")
        ));
    };
    let pattern = entry.pattern();
    let time_signature: TimeSignature = pattern.time_signature();
    let stretch_length: ScoreLength = time_signature
        .bar_length()
        .checked_mul(request.bars)
        .map_err(|e| e.to_string())?;

    let velocity = |value: u8| Velocity::new(value).map_err(|e| e.to_string());
    let levels: VelocityLevels = VelocityLevels::new(
        velocity(NORMAL_VELOCITY)?,
        Some(velocity(ACCENT_VELOCITY)?),
        Some(velocity(ACCENT_VELOCITY)?),
    );
    let beat: NoteTrack = NoteTrack::from_rhythm(
        "Beat",
        port_name(PULSE_PORT)?,
        request.channel,
        request.note,
        levels,
        pattern,
        stretch_length,
    )
    .map_err(|e| e.to_string())?;
    let cues: Vec<PhraseCue> = if request.transport {
        transport_cues(stretch_length)?
    } else {
        Vec::new()
    };
    let control: ControlTrack =
        ControlTrack::new("Rhythm test", cues).map_err(|e| e.to_string())?;
    let events: Vec<ScheduledEvent> =
        schedule(&port_name(CLOCK_PORT)?, &[beat], &[control], stretch_length)
            .map_err(|e| e.to_string())?;

    let output = event_player::open_output(&[CLOCK_PORT, PULSE_PORT])?;
    event_player::print_ports(&output);
    print_plan(request, pattern.name(), time_signature);
    event_player::wait_for_enter()?;

    let tempo: Tempo = Tempo::from_quarter_notes_per_minute(request.quarter_notes_per_minute)
        .map_err(|e| e.to_string())?;
    event_player::play(&output, &events, tempo, &mut |event: &ScheduledEvent| {
        announce(event, time_signature);
    })?;
    event_player::silence(&output, PULSE_PORT, request.channel)?;
    println!("Done.");
    Ok(())
}

/// What is about to play.
fn print_plan(request: &PlayRequest, rhythm_name: &str, time_signature: TimeSignature) {
    println!(
        "Playing {:?} ({}/{}) for {} bar{} at {} q/min on {PULSE_PORT:?}, channel {}, note {}. \
         Timing Clock{} on {CLOCK_PORT:?}.",
        rhythm_name,
        time_signature.pulses_per_bar(),
        time_signature.pulses_per_whole_note(),
        request.bars,
        if request.bars == 1 { "" } else { "s" },
        request.quarter_notes_per_minute,
        request.channel.number(),
        request.note.value(),
        if request.transport {
            " and Start/Stop"
        } else {
            ""
        }
    );
}

/// Start at the first note and Stop at the end, on the clock port. The FH-2's
/// clock output pulses only between a Start and a Stop, so it can be watched
/// only with these.
fn transport_cues(stretch_length: ScoreLength) -> Result<Vec<PhraseCue>, String> {
    let end: ScorePosition = ScorePosition::ZERO
        .checked_add(stretch_length)
        .map_err(|e| e.to_string())?;
    let mut cues: Vec<PhraseCue> = Vec::new();
    for (name, position, message) in [
        ("Start", ScorePosition::ZERO, TransportMessage::Start),
        ("Stop", end, TransportMessage::Stop),
    ] {
        cues.push(
            PhraseCue::new(
                name,
                position,
                port_name(CLOCK_PORT)?,
                ControlCueMessage::Transport(message),
            )
            .map_err(|e| e.to_string())?,
        );
    }
    Ok(cues)
}

/// Shows each note as it is queued: its place, and whether it is an accent.
fn announce(event: &ScheduledEvent, time_signature: TimeSignature) {
    if let (EventSource::Notes { .. }, OutputAction::NoteOn { velocity, .. }) =
        (&event.source, event.action)
    {
        let accent: &str = if velocity.value() == ACCENT_VELOCITY {
            "  (accent)"
        } else {
            ""
        };
        println!(
            "{}{accent}",
            describe_place(event.position.ticks(), time_signature)
        );
    }
}

fn port_name(name: &str) -> Result<OutputPortName, String> {
    OutputPortName::new(name).map_err(|e| e.to_string())
}
