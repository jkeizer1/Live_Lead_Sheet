//! Test: loads a rhythm library, prints every rhythm in the musician's
//! terms, and checks that mistakes in a rhythm file are reported as
//! EFS-6.3 asks (file, line, what is wrong, what to type instead).
//!
//! With `--play`, then plays that rhythm on the beat-pulse port, repeated to
//! fill the bars asked for, for listening on the FH-2.
//!
//! ```text
//! cargo run -p song_text --example rhythm_notation_test -- [LIBRARY_FILE]
//!     [--play "Name" [--bpm N] [--bars N] [--channel C] [--note N] [--transport]]
//! ```
//!
//! | Option | Default | Meaning |
//! |---|---|---|
//! | `LIBRARY_FILE` | `library/Rhythms.library` | the rhythm library to load |
//! | `--play "Name"` | none | play this rhythm after the checks |
//! | `--bpm N` | 120 | tempo, quarter notes per minute |
//! | `--bars N` | 8 | bars to play, the rhythm repeating to fill them |
//! | `--channel C` | 15 | beat-pulse MIDI channel, 1 to 16 |
//! | `--note N` | 60 | beat-pulse note number |
//! | `--transport` | off | send Start at the first note and Stop at the end, on the `Clock` port; the FH-2's clock output pulses only between them |
//!
//! The defaults only choose what this test sends. Exit status is 0 when
//! the library loads, every built-in mistake is reported as expected, and
//! (with `--play`) the rhythm plays. See `explainer.md`.

mod play;

#[path = "../support/event_player.rs"]
mod event_player;

use std::collections::BTreeMap;
use std::process::ExitCode;

use song_text::length_words::{describe_length, describe_place};
use song_text::parse_error::ParseError;
use song_text::rhythm_library::{RhythmLibraryEntry, parse_rhythm_library};
use vocabulary::rhythm_pattern::{AccentLevel, ElementKind, RhythmElement};
use vocabulary::time_signature::TimeSignature;

const DEFAULT_LIBRARY_FILE: &str = "library/Rhythms.library";

/// A bad file, and the one message it must produce.
struct MistakeCase {
    what: &'static str,
    text: &'static str,
    expected_message: &'static str,
}

const MISTAKE_CASES: [MistakeCase; 6] = [
    MistakeCase {
        what: "a bar that is short",
        text: "rhythm \"X\"\n group \"G\"\n meter 4/4\n | q q q e |\n",
        expected_message: "mistake.library line 4: bar 1 is short by an eighth note. Add e or er.",
    },
    MistakeCase {
        what: "a bar that is long",
        text: "rhythm \"X\"\n group \"G\"\n meter 3/4\n | q q q q |\n",
        expected_message: "mistake.library line 4: bar 1 is over by a quarter note. \
                           Shorten or remove an element.",
    },
    MistakeCase {
        what: "a letter that is not a note value",
        text: "rhythm \"X\"\n group \"G\"\n meter 4/4\n | q q q z |\n",
        expected_message: "mistake.library line 4: 'z' is not a note value. Use w, h, q, e, sx \
                           or tx, e.g. q for a quarter note",
    },
    MistakeCase {
        what: "a rhythm without a group",
        text: "rhythm \"X\"\n meter 4/4\n | w |\n",
        expected_message: "mistake.library line 1: rhythm \"X\" has no group. \
                           Add a line such as group \"Swing\"",
    },
    MistakeCase {
        what: "a tie with nothing to join",
        text: "rhythm \"X\"\n group \"G\"\n meter 4/4\n | hr qr er e~ |\n",
        expected_message: "mistake.library line 1: rhythm \"X\": the tie at bar 1 beat 4 has no note \
                           after it to join. Remove the ~ or put a note after it",
    },
    MistakeCase {
        what: "a meter that is not one",
        text: "rhythm \"X\"\n group \"G\"\n meter 4/3\n | w |\n",
        expected_message: "mistake.library line 3: 4/3 is not a time signature. Use 1 or more \
                           on top, and 1, 2, 4, 8 or 16 below: e.g. 4/4, 6/8",
    },
];

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (library_file, play_request) = match play::parse_arguments(&arguments, DEFAULT_LIBRARY_FILE)
    {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    let entries: Option<Vec<RhythmLibraryEntry>> = print_library(&library_file);
    let mistakes_ok: bool = check_mistakes();
    let (Some(entries), true) = (entries, mistakes_ok) else {
        return ExitCode::FAILURE;
    };
    println!("\nAll good.");

    if let Some(request) = play_request
        && let Err(message) = play::play_rhythm(&entries, &request)
    {
        eprintln!("{message}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Loads the file and prints its rhythms by meter, then group. `None`
/// when it cannot be loaded; the reason has been printed.
fn print_library(library_file: &str) -> Option<Vec<RhythmLibraryEntry>> {
    let text: String = match std::fs::read_to_string(library_file) {
        Ok(text) => text,
        Err(error) => {
            eprintln!(
                "ERROR: cannot read the rhythm library {library_file}: {error}. \
                 Run from the project folder, or give the file as the argument."
            );
            return None;
        }
    };
    let entries: Vec<RhythmLibraryEntry> = match parse_rhythm_library(library_file, &text) {
        Ok(entries) => entries,
        Err(errors) => {
            println!("{library_file} is not valid; nothing was loaded.");
            for (index, error) in errors.iter().enumerate() {
                println!("  Issue {}: {error}", index + 1);
            }
            return None;
        }
    };

    let mut listing: BTreeMap<(String, String), Vec<&RhythmLibraryEntry>> = BTreeMap::new();
    for entry in &entries {
        let time_signature: TimeSignature = entry.pattern().time_signature();
        let meter: String = format!(
            "{}/{}",
            time_signature.pulses_per_bar(),
            time_signature.pulses_per_whole_note()
        );
        listing
            .entry((meter, entry.group().to_string()))
            .or_default()
            .push(entry);
    }
    println!("{library_file}: {} rhythms", entries.len());
    for ((meter, group), group_entries) in &listing {
        println!("\n{meter} / {group}");
        for entry in group_entries {
            print_pattern(entry);
        }
    }
    Some(entries)
}

fn print_pattern(entry: &RhythmLibraryEntry) {
    let pattern = entry.pattern();
    let bars: u8 = pattern.bar_count();
    println!(
        "  {} ({bars} bar{})",
        pattern.name(),
        if bars == 1 { "" } else { "s" }
    );
    for element in pattern.elements() {
        println!(
            "    {}",
            describe_element(element, pattern.time_signature())
        );
    }
}

/// "bar 1 beat 2: a dotted quarter note, accent".
fn describe_element(element: &RhythmElement, time_signature: TimeSignature) -> String {
    let place: String = describe_place(element.start().ticks(), time_signature);
    let what: String = match element.kind() {
        ElementKind::Sounding => describe_length(element.length().ticks()).description,
        ElementKind::Rest => format!(
            "{} rest",
            describe_length(element.length().ticks())
                .description
                .trim_end_matches(" note")
        ),
    };
    let marks = element.marks();
    let mut notes: Vec<&str> = Vec::new();
    match marks.accent() {
        AccentLevel::Normal => {}
        AccentLevel::Accent => notes.push("accent"),
        AccentLevel::Strong => notes.push("strong accent"),
    }
    if marks.staccato() {
        notes.push("staccato");
    }
    if marks.legato() {
        notes.push("legato");
    }
    if marks.fermata() {
        notes.push("fermata");
    }
    if element.tied_to_next() {
        notes.push("tied to the next note");
    }
    if notes.is_empty() {
        format!("{place}: {what}")
    } else {
        format!("{place}: {what}, {}", notes.join(", "))
    }
}

/// Feeds each bad file to the parser and compares the one message.
fn check_mistakes() -> bool {
    println!("\nMistakes are reported (EFS-6.3):");
    let mut all_ok: bool = true;
    for case in &MISTAKE_CASES {
        let result: Result<Vec<RhythmLibraryEntry>, Vec<ParseError>> =
            parse_rhythm_library("mistake.library", case.text);
        let message: String = match result {
            Ok(_) => String::from("(the file loaded)"),
            Err(errors) if errors.len() == 1 => errors[0].to_string(),
            Err(errors) => format!("({} messages)", errors.len()),
        };
        if message == case.expected_message {
            println!("  ok    {}: {message}", case.what);
        } else {
            all_ok = false;
            println!(
                "  WRONG {}:\n          got      {message}\n          expected {}",
                case.what, case.expected_message
            );
        }
    }
    all_ok
}
