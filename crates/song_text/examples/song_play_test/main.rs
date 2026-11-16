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
use player::song_plan::{SectionPass, SongPlan, plan_song};
use player::studio_configuration::StudioConfiguration;
use song_text::length_words::describe_place_after_bars;
use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;

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

    let mut last_announced: Option<(String, u64)> = None;
    let mut next_pass: usize = 0;
    event_player::play(
        &output,
        &events,
        plan.tempo,
        &plan.tempo_changes,
        &mut |event: &ScheduledEvent| {
            // The first event at or after a pass's start says the pass begins.
            while let Some(pass) = plan.section_passes.get(next_pass) {
                if event.position.ticks() < pass.start_ticks {
                    break;
                }
                println!("{}", describe_pass(pass, &plan.section_passes));
                next_pass += 1;
            }
            announce(event, &plan.section_passes, &mut last_announced);
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

/// One line for the start of a play of a section: its name and pass, key,
/// meter and tempo (in the meter's beats), and where it starts.
fn describe_pass(pass: &SectionPass, passes: &[SectionPass]) -> String {
    let key: String = pass
        .key
        .map_or_else(|| String::from("no key"), |key| key.to_string());
    // milli-quarters/min × ticks/quarter ÷ ticks/beat → milli-beats/min.
    let milli_beats_per_minute: u64 = pass.tempo.milli_quarters_per_minute()
        * TICKS_PER_QUARTER_NOTE
        / pass.time_signature.beat_length().ticks();
    let tempo: String = if milli_beats_per_minute.is_multiple_of(1000) {
        format!("{}", milli_beats_per_minute / 1000)
    } else {
        format!(
            "{}.{:03}",
            milli_beats_per_minute / 1000,
            milli_beats_per_minute % 1000
        )
        .trim_end_matches('0')
        .to_string()
    };
    format!(
        "\nSection \"{}\"{}, pass {} of {}: {key}, {}/{}, {tempo} beats/min ({})",
        pass.section_name,
        if pass.count_in { " (count-in)" } else { "" },
        pass.pass_number,
        pass.pass_count,
        pass.time_signature.pulses_per_bar(),
        pass.time_signature.pulses_per_whole_note(),
        place_in_song(pass.start_ticks, passes)
    )
}

/// Where a position is in the song, as bar and beat, counted through every
/// section before it in that section's own meter, so a meter change does not
/// shift the bar numbers after it.
fn place_in_song(position_ticks: u64, passes: &[SectionPass]) -> String {
    let Some(index) = passes
        .iter()
        .rposition(|pass: &SectionPass| pass.start_ticks <= position_ticks)
    else {
        // Before the first pass: nothing to count from, so the first pass's
        // meter is the only one there is.
        return passes
            .first()
            .map_or_else(String::new, |first: &SectionPass| {
                describe_place_after_bars(position_ticks, first.time_signature, 0)
            });
    };
    // bars before = sum over earlier passes of (ticks of the pass ÷ ticks/bar).
    let bars_before: u64 = passes[..index]
        .iter()
        .zip(&passes[1..=index])
        .map(|(pass, next): (&SectionPass, &SectionPass)| {
            (next.start_ticks - pass.start_ticks) / pass.time_signature.bar_length().ticks()
        })
        .sum();
    describe_place_after_bars(
        position_ticks - passes[index].start_ticks,
        passes[index].time_signature,
        bars_before,
    )
}

/// Shows each voice's notes once per place and each cue as it is queued.
fn announce(
    event: &ScheduledEvent,
    passes: &[SectionPass],
    last_announced: &mut Option<(String, u64)>,
) {
    let place: String = place_in_song(event.position.ticks(), passes);
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
