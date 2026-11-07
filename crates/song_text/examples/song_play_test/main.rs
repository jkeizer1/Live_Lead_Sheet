//! Test: plays a song through a studio configuration on the MIDI output
//! ports, for listening on Jim's studio: the count-in, the beat, the
//! harmony, the bass and the cues of `songs/Demo.song`, sent where
//! `songs/Demo.studio` says.
//!
//! ```text
//! cargo run --release -p song_text --example song_play_test -- [SONG_FILE [STUDIO_FILE]]
//! ```
//!
//! | Argument | Default | Meaning |
//! |---|---|---|
//! | `SONG_FILE` | `songs/Demo.song` | the song to play |
//! | `STUDIO_FILE` | `songs/Demo.studio` | where its tracks go |
//!
//! The tempo is the song's. The program opens one output port for each port
//! the studio lists, names them, and waits for you to connect them (MIDI
//! router or `aconnect`) and press Enter. See `explainer.md`.

#[path = "../support/event_player.rs"]
mod event_player;

use std::fmt::Write;
use std::process::ExitCode;

use player::schedule::{EventSource, OutputAction, ScheduledEvent, schedule};
use player::song_plan::{SongPlan, plan_song};
use player::studio_configuration::StudioConfiguration;
use song_text::length_words::describe_place;
use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;
use vocabulary::time_signature::TimeSignature;

const DEFAULT_SONG_FILE: &str = "songs/Demo.song";
const DEFAULT_STUDIO_FILE: &str = "songs/Demo.studio";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let song_file: &str = arguments.first().map_or(DEFAULT_SONG_FILE, String::as_str);
    let studio_file: &str = arguments.get(1).map_or(DEFAULT_STUDIO_FILE, String::as_str);
    match run(song_file, studio_file) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(song_file: &str, studio_file: &str) -> Result<(), String> {
    let song: Song = load(song_file, "song", parse_song)?;
    let studio: StudioConfiguration = load(studio_file, "studio", parse_studio)?;
    let plan: SongPlan = plan_song(&song, &studio)
        .map_err(|e| format!("ERROR: cannot play {song_file} with {studio_file}: {e}"))?;
    let events: Vec<ScheduledEvent> = schedule(
        &plan.clock_port,
        &plan.note_tracks,
        &plan.control_tracks,
        plan.stretch_length,
    )
    .map_err(|e| format!("ERROR: cannot schedule {song_file}: {e}"))?;

    let port_names: Vec<&str> = studio.port_names().iter().map(String::as_str).collect();
    let output = event_player::open_output(&port_names)?;
    event_player::print_ports(&output);
    println!(
        "\nPlaying \"{}\" with studio \"{}\" at {} q/min: {} events.",
        song.name(),
        studio.name(),
        plan.tempo.milli_quarters_per_minute() / 1000,
        events.len()
    );
    for note in &plan.not_yet_applied {
        println!("NOTE: {note}");
    }
    event_player::wait_for_enter()?;

    // Places are counted from the song's start in the first section's meter.
    let time_signature: TimeSignature = song
        .sections()
        .first()
        .map(vocabulary::song::Section::time_signature)
        .ok_or_else(|| String::from("ERROR: the song has no sections."))?;
    let mut last_announced: Option<(String, u64)> = None;
    event_player::play(
        &output,
        &events,
        plan.tempo,
        &mut |event: &ScheduledEvent| {
            announce(event, time_signature, &mut last_announced);
        },
    )?;
    for track in &plan.note_tracks {
        event_player::silence(&output, track.port().as_str(), track.channel())?;
    }
    println!("Done.");
    Ok(())
}

/// Reads and parses one file, or returns every mistake, ready to print.
fn load<T>(
    file_name: &str,
    what: &str,
    parse: fn(&str, &str) -> Result<T, Vec<ParseError>>,
) -> Result<T, String> {
    let text: String = std::fs::read_to_string(file_name).map_err(|e| {
        format!(
            "ERROR: cannot read the {what} {file_name}: {e}. Run from the project folder, or \
             give the file as the argument."
        )
    })?;
    parse(file_name, &text).map_err(|errors| {
        let mut message: String = format!("ERROR: {file_name} is not valid; nothing was played.");
        for (index, error) in errors.iter().enumerate() {
            let _ = write!(message, "\n  Issue {}: {error}", index + 1);
        }
        message
    })
}

/// Shows each voice's notes once per place and each cue as it is queued.
fn announce(
    event: &ScheduledEvent,
    time_signature: TimeSignature,
    last_announced: &mut Option<(String, u64)>,
) {
    let place: String = describe_place(event.position.ticks(), time_signature);
    match (&event.source, event.action) {
        (EventSource::Notes { track_name }, OutputAction::NoteOn { velocity, .. }) => {
            let key: (String, u64) = (track_name.clone(), event.position.ticks());
            if last_announced.as_ref() != Some(&key) {
                println!("{place}: {track_name}, velocity {}", velocity.value());
                *last_announced = Some(key);
            }
        }
        (EventSource::Cue { cue_name, .. }, _) => println!("{place}: cue \"{cue_name}\""),
        _ => {}
    }
}
