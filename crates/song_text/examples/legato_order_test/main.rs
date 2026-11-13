//! Test: prints what the player hands to the ALSA output for a song, track
//! by track, with the instant of each message, and checks that where a
//! sample source changes inside a sounding note the new note on is sent
//! before the old note off, at the same instant. No MIDI hardware is used.
//!
//! ```text
//! cargo run -p song_text --example legato_order_test -- [SONG_FILE [STUDIO_FILE]]
//! ```
//!
//! | Argument | Default | Meaning |
//! |---|---|---|
//! | `SONG_FILE` | `songs/Legato_test.song` | the song to list |
//! | `STUDIO_FILE` | `songs/Demo.studio` | where its tracks go |
//!
//! Instants count from the first beat of the song. See `explainer.md`.

use std::fmt::Write;
use std::process::ExitCode;

use alsa_output::sequencer_output::TOUCHING_NOTE_OFF_LEAD;
use player::anchor::Anchor;
use player::schedule::{EventSource, OutputAction, ScheduledEvent, schedule};
use player::song_plan::{SongPlan, plan_song};
use player::studio_configuration::StudioConfiguration;
use song_text::length_words::describe_place;
use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::score_position::ScorePosition;
use vocabulary::song::Song;
use vocabulary::time_signature::TimeSignature;

const DEFAULT_SONG_FILE: &str = "songs/Legato_test.song";
const DEFAULT_STUDIO_FILE: &str = "songs/Demo.studio";
const NANOSECONDS_PER_MILLISECOND: u64 = 1_000_000;
const NANOSECONDS_PER_MICROSECOND: u64 = 1_000;

/// One note message of one track, as the player sends it.
struct NoteMessage {
    position_ticks: u64,
    sent_at_ns: u64,
    text: String,
    kind: MessageKind,
    note: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MessageKind {
    NoteOn,
    LegatoNoteOff,
    OtherNoteOff,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let song_file: &str = arguments.first().map_or(DEFAULT_SONG_FILE, String::as_str);
    let studio_file: &str = arguments.get(1).map_or(DEFAULT_STUDIO_FILE, String::as_str);
    match run(song_file, studio_file) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Returns whether every legato note off came right after the note on of
/// the note that replaced it.
fn run(song_file: &str, studio_file: &str) -> Result<bool, String> {
    let song: Song = load(song_file, "song", parse_song)?;
    let studio: StudioConfiguration = load(studio_file, "studio", parse_studio)?;
    let plan: SongPlan = plan_song(&song, &studio)
        .map_err(|e| format!("ERROR: cannot plan {song_file} with {studio_file}: {e}"))?;
    let events: Vec<ScheduledEvent> = schedule(
        &plan.clock_port,
        &plan.note_tracks,
        &plan.control_tracks,
        plan.stretch_length,
    )
    .map_err(|e| format!("ERROR: cannot schedule {song_file}: {e}"))?;
    let time_signature: TimeSignature = song
        .sections()
        .first()
        .map(vocabulary::song::Section::time_signature)
        .ok_or_else(|| String::from("ERROR: the song has no sections."))?;
    // Instant 0 stands for the first beat; the sending loop adds a start
    // delay to every instant alike.
    let anchor: Anchor = Anchor::new(ScorePosition::ZERO, Instant::from_ns(0), plan.tempo);

    println!(
        "\"{}\" with studio \"{}\" at {} q/min. Messages in the order the player hands them to ALSA:",
        song.name(),
        studio.name(),
        plan.tempo.milli_quarters_per_minute() / 1000
    );
    let mut track_names: Vec<String> = Vec::new();
    for event in &events {
        if let EventSource::Notes { track_name } = &event.source
            && !track_names.contains(track_name)
        {
            track_names.push(track_name.clone());
        }
    }
    let mut all_in_order: bool = true;
    for track_name in &track_names {
        let messages: Vec<NoteMessage> = note_messages(&events, track_name, anchor)?;
        println!("\nTrack \"{track_name}\":");
        for message in &messages {
            println!(
                "  {:<24} {}  {}",
                describe_place(message.position_ticks, time_signature),
                describe_instant(message.sent_at_ns),
                message.text
            );
        }
        let (legato_count, wrong_count): (usize, usize) = check_legato_order(&messages);
        if legato_count > 0 {
            println!(
                "  Legato check: {legato_count} legato note off(s); {wrong_count} not right after \
                 the note on of the note that replaced it."
            );
        }
        all_in_order &= wrong_count == 0;
    }
    Ok(all_in_order)
}

/// The track's note messages in sending order, each with the instant the
/// ALSA output stamps on it.
fn note_messages(
    events: &[ScheduledEvent],
    track_name: &str,
    anchor: Anchor,
) -> Result<Vec<NoteMessage>, String> {
    let mut messages: Vec<NoteMessage> = Vec::new();
    for event in events {
        let EventSource::Notes {
            track_name: event_track,
        } = &event.source
        else {
            continue;
        };
        if event_track != track_name {
            continue;
        }
        let at: Instant = anchor
            .instant_at(event.position)
            .map_err(|e| format!("ERROR: cannot place an event of \"{track_name}\": {e}"))?;
        let (kind, note, sent_at_ns, text): (MessageKind, u8, u64, String) = match event.action {
            OutputAction::NoteOn {
                channel,
                note,
                velocity,
            } => (
                MessageKind::NoteOn,
                note.value(),
                at.ns(),
                format!(
                    "note on   channel {} note {} velocity {}",
                    channel.number(),
                    note.value(),
                    velocity.value()
                ),
            ),
            OutputAction::NoteOff { channel, note } => (
                MessageKind::OtherNoteOff,
                note.value(),
                at.ns(),
                format!(
                    "note off  channel {} note {}",
                    channel.number(),
                    note.value()
                ),
            ),
            OutputAction::LegatoNoteOff { channel, note } => (
                MessageKind::LegatoNoteOff,
                note.value(),
                at.ns(),
                format!(
                    "note off  channel {} note {}   (legato: after the note on at the same instant)",
                    channel.number(),
                    note.value()
                ),
            ),
            OutputAction::TouchingNoteOff {
                channel,
                note,
                retrigger_gap,
                ..
            } => {
                // As the ALSA output sends it: earlier than the next note
                // on by the larger of its lead and the studio's gap.
                let lead: Duration = retrigger_gap.max(TOUCHING_NOTE_OFF_LEAD);
                (
                    MessageKind::OtherNoteOff,
                    note.value(),
                    at.ns().saturating_sub(lead.ns()),
                    format!(
                        "note off  channel {} note {}   (touching: {} ms before the next note on)",
                        channel.number(),
                        note.value(),
                        lead.ns() / NANOSECONDS_PER_MILLISECOND
                    ),
                )
            }
            _ => continue,
        };
        messages.push(NoteMessage {
            position_ticks: event.position.ticks(),
            sent_at_ns,
            text,
            kind,
            note,
        });
    }
    Ok(messages)
}

/// How many legato note offs there are, and how many of them are not
/// directly after a note on, at the same instant, of a different note.
fn check_legato_order(messages: &[NoteMessage]) -> (usize, usize) {
    let mut legato_count: usize = 0;
    let mut wrong_count: usize = 0;
    for (index, message) in messages.iter().enumerate() {
        if message.kind != MessageKind::LegatoNoteOff {
            continue;
        }
        legato_count += 1;
        let after_note_on: bool = index
            .checked_sub(1)
            .and_then(|previous| messages.get(previous))
            .is_some_and(|previous: &NoteMessage| {
                previous.kind == MessageKind::NoteOn
                    && previous.sent_at_ns == message.sent_at_ns
                    && previous.note != message.note
            });
        if !after_note_on {
            wrong_count += 1;
        }
    }
    (legato_count, wrong_count)
}

/// An instant in milliseconds with microseconds, e.g. `1000.000 ms`.
fn describe_instant(instant_ns: u64) -> String {
    format!(
        "{:>9}.{:03} ms",
        instant_ns / NANOSECONDS_PER_MILLISECOND,
        instant_ns % NANOSECONDS_PER_MILLISECOND / NANOSECONDS_PER_MICROSECOND
    )
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
        let mut message: String = format!("ERROR: {file_name} is not valid; nothing was listed.");
        for (index, error) in errors.iter().enumerate() {
            let _ = write!(message, "\n  Issue {}: {error}", index + 1);
        }
        message
    })
}
