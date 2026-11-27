//! Test: plays a song through a studio configuration on the MIDI output
//! ports, for listening on Jim's studio, through the same sending loop the
//! window uses (`crates/sender`). It plays from the start or from any bar,
//! loops bars, stops, and plays again, all on one set of open ports, so
//! what the window does between plays can be heard here without the window.
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
//! router or `aconnect`). It then takes commands, one per line:
//!
//! | Command | Does |
//! |---|---|
//! | Enter, or `play` | plays from the start |
//! | `from N` | plays from bar N (counted from 1 through the whole song) |
//! | `loop A B` | plays bars A to B again and again |
//! | `stop` | stops, silencing every note |
//! | `quit`, or Ctrl+D | ends the program |
//!
//! After each play it prints how the sender's timing went. See
//! `explainer.md`.

use std::fmt::Write;
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration as StdDuration;

use player::schedule::{ScheduledEvent, schedule};
use player::song_plan::{SectionPass, SongPlan, plan_song};
use player::studio_configuration::StudioConfiguration;
use sender::play_request::PlayRequest;
use sender::player::Player;
use song_text::length_words::{describe_place_after_bars, describe_place_in_bars};
use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;

const DEFAULT_SONG_FILE: &str = "songs/Demo.song";
const DEFAULT_STUDIO_FILE: &str = "songs/Demo.studio";
/// How often the program looks at the player and for a command.
const LOOK_INTERVAL: StdDuration = StdDuration::from_millis(20);

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

/// A song planned and scheduled once; each play is a request made from it.
struct Prepared {
    events: Vec<ScheduledEvent>,
    plan: SongPlan,
}

impl Prepared {
    /// Built the way the window builds it, so both send the same requests.
    fn request(&self, start_ticks: u64, end_ticks: Option<u64>, looping: bool) -> PlayRequest {
        PlayRequest {
            events: self.events.clone(),
            tempo: self.plan.tempo,
            tempo_changes: self.plan.tempo_changes.clone(),
            start_ticks,
            end_ticks,
            looping,
            clock_port: self.plan.clock_port.clone(),
            note_outputs: self
                .plan
                .note_tracks
                .iter()
                .map(|track| (track.port().clone(), track.channel()))
                .collect(),
        }
    }
}

fn run(song_file: &str, studio_file: &str) -> Result<(), String> {
    let song: Song = load(song_file, "song", parse_song)?;
    let studio: StudioConfiguration = load(studio_file, "studio", parse_studio)?;
    let plan: SongPlan = plan_song(&song, &studio)
        .map_err(|e| format!("\n\nError: cannot play {song_file} with {studio_file}: {e}"))?;
    let events: Vec<ScheduledEvent> = schedule(
        &plan.clock_port,
        &plan.note_tracks,
        &plan.control_tracks,
        plan.stretch_length,
    )
    .map_err(|e| format!("\n\nError: cannot schedule {song_file}: {e}"))?;
    let player: Player = Player::start(studio.port_names().to_vec())?;
    println!("\nPorts:");
    for port in player.ports() {
        println!("  {port}");
    }
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
    println!(
        "Connect the ports now (MIDI router or aconnect). Commands: Enter plays from the \
         start, `from N`, `loop A B`, `stop`, `quit`."
    );
    let prepared: Prepared = Prepared { events, plan };
    command_loop(&player, &prepared);
    println!("Done.");
    Ok(())
}

/// Reads commands from the terminal in a thread of its own, so the program
/// can keep announcing sections and reporting while it waits for one.
fn read_commands() -> Receiver<String> {
    let (sender, receiver): (Sender<String>, Receiver<String>) = mpsc::channel();
    thread::spawn(move || {
        let mut line: String = String::new();
        while std::io::stdin()
            .read_line(&mut line)
            .is_ok_and(|bytes| bytes > 0)
        {
            if sender.send(line.trim().to_string()).is_err() {
                return;
            }
            line.clear();
        }
        // End of input: the receiver sees the channel close.
    });
    receiver
}

fn command_loop(player: &Player, prepared: &Prepared) {
    let commands: Receiver<String> = read_commands();
    let passes: &[SectionPass] = &prepared.plan.section_passes;
    let mut announced_pass: Option<usize> = None;
    loop {
        while let Some(problem) = player.next_problem() {
            eprintln!("WARNING: {problem}");
        }
        while let Some(report) = player.next_report() {
            println!("{}", report.summary());
        }
        announce_pass(player, passes, &mut announced_pass);
        match commands.recv_timeout(LOOK_INTERVAL) {
            Ok(line) => {
                if !carry_out(&line, player, prepared) {
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                if let Err(message) = player.stop() {
                    eprintln!("{message}");
                }
                return;
            }
        }
    }
}

/// Prints the line for a section's pass when the song reaches it.
fn announce_pass(player: &Player, passes: &[SectionPass], announced: &mut Option<usize>) {
    let position_ticks: Option<u64> = player.position_ticks().filter(|_| player.playing());
    let Some(position_ticks) = position_ticks else {
        *announced = None;
        return;
    };
    let index: Option<usize> = passes
        .iter()
        .rposition(|pass: &SectionPass| pass.start_ticks <= position_ticks);
    if index != *announced
        && let Some(index) = index
    {
        println!("{}", describe_pass(&passes[index], passes));
        *announced = Some(index);
    }
}

/// Carries out one command; false when the program should end.
fn carry_out(line: &str, player: &Player, prepared: &Prepared) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    let total_ticks: u64 = prepared.plan.stretch_length.ticks();
    let passes: &[SectionPass] = &prepared.plan.section_passes;
    let result: Result<(), String> = match words.as_slice() {
        [] | ["play"] => player.play(prepared.request(0, None, false)),
        ["stop"] => player.stop(),
        ["quit" | "q"] => return false,
        ["from", bar] => bar_ticks(passes, bar, total_ticks, 0)
            .and_then(|start| player.play(prepared.request(start, None, false))),
        ["loop", first, last] => bar_ticks(passes, first, total_ticks, 0).and_then(|start| {
            // The loop ends where the bar after the last one starts.
            let end: u64 = bar_ticks(passes, last, total_ticks, 1)?;
            if end <= start {
                return Err(String::from("Loop: the last bar comes before the first."));
            }
            player.play(prepared.request(start, Some(end), true))
        }),
        _ => Err(String::from(
            "Commands: Enter (play), from N, loop A B, stop, quit.",
        )),
    };
    if let Err(message) = result {
        println!("{message}");
    }
    true
}

/// Where bar `text` (counted from 1 through the whole song, each section in
/// its own meter), plus `extra_bars`, starts, in ticks.
fn bar_ticks(
    passes: &[SectionPass],
    text: &str,
    total_ticks: u64,
    extra_bars: u32,
) -> Result<u64, String> {
    let bar_number: u32 = text
        .parse::<u32>()
        .ok()
        .filter(|number| *number >= 1)
        .ok_or_else(|| format!("\"{text}\" is not a bar number (1 or more)."))?
        + extra_bars;
    let mut bars_before: u32 = 0;
    for pass in passes {
        if bar_number <= bars_before + pass.bar_count {
            let bar_index: usize = usize::try_from(bar_number - bars_before - 1)
                .map_err(|_| String::from("That bar number is too large."))?;
            return Ok(pass.start_ticks + pass.bar_meters.bar_start_ticks(bar_index));
        }
        bars_before += pass.bar_count;
    }
    if bar_number == bars_before + 1 {
        // Just past the last bar: the end of the song.
        return Ok(total_ticks);
    }
    Err(format!(
        "The song has {bars_before} bars; there is no bar {bar_number}."
    ))
}

/// The file's own name without its folders, for messages the musician reads
/// at a glance. A path with no file name is shown whole: harmless, it is
/// only a message.
fn short_name(path: &str) -> &str {
    std::path::Path::new(path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or(path)
}

/// Reads and parses one file, or returns every mistake, ready to print.
fn load<T>(
    file_name: &str,
    what: &str,
    parse: fn(&str, &str) -> Result<T, Vec<ParseError>>,
) -> Result<T, String> {
    let text: String = std::fs::read_to_string(file_name).map_err(|e| {
        format!(
            "\n\nError: cannot read the {what} {file_name}: {e}. Run from the project folder, or \
             give the file as the argument."
        )
    })?;
    let short_file_name: &str = short_name(file_name);
    parse(short_file_name, &text).map_err(|errors| {
        let mut message: String = format!("\n\nError: {short_file_name} is not valid.");
        for (index, error) in errors.iter().enumerate() {
            let _ = write!(
                message,
                "\n\n  Issue {}: {}",
                index + 1,
                error.line_and_message()
            );
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
    // bars before = the bars of every earlier pass.
    let bars_before: u64 = passes[..index]
        .iter()
        .map(|pass: &SectionPass| u64::from(pass.bar_count))
        .sum();
    describe_place_in_bars(
        position_ticks - passes[index].start_ticks,
        &passes[index].bar_meters,
        bars_before,
    )
}
