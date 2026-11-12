//! Test: loads a song file and a studio file, prints the song in the
//! musician's terms, checks that the studio covers the song, and checks that
//! mistakes in either file are reported as EFS-6.3 asks (file, line, what is
//! wrong and what to type instead).
//!
//! ```text
//! cargo run -p song_text --example song_notation_test -- [SONG_FILE [STUDIO_FILE]]
//! ```
//!
//! | Argument | Default | Meaning |
//! |---|---|---|
//! | `SONG_FILE` | `songs/Demo.song` | the song to load |
//! | `STUDIO_FILE` | `songs/Demo.studio` | the studio to check it against |
//!
//! Exit status is 0 when both files load, the studio covers the song and
//! every built-in mistake is reported as expected. See `explainer.md`.

use std::fmt::Write;
use std::process::ExitCode;

use player::studio_configuration::{StudioConfiguration, UnmetNeed};
use song_text::length_words::{describe_length, describe_place};
use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::rhythm_pattern::{AccentLevel, ElementKind};
use vocabulary::song::{
    Clef, CueDestination, CueMessage, ElementContents, Line, LineElement, LineRole, Section, Song,
    Track, TrackKind, VoiceKind, VoiceLayerName, VoiceLayerNumber,
};

const DEFAULT_SONG_FILE: &str = "songs/Demo.song";
const DEFAULT_STUDIO_FILE: &str = "songs/Demo.studio";

/// Which file a mistake case is written for.
#[derive(Clone, Copy)]
enum Kind {
    Song,
    Studio,
}

/// A bad file, and the one message it must produce.
struct MistakeCase {
    what: &'static str,
    kind: Kind,
    text: &'static str,
    expected_message: &'static str,
}

/// The tracks every song case starts with; each case adds its own sections.
const SONG_HEAD: &str = "song \"T\"\ntrack \"Clock\"\n type clock_and_transport\n\
                         track \"Keys\"\n type poly_voice\n clef grand\n lead\n\
                         track \"Bass\"\n type mono_voice\n clef bass\n";

const MISTAKE_CASES: [MistakeCase; 9] = [
    MistakeCase {
        what: "a line for a track the song does not have",
        kind: Kind::Song,
        text: concat!(
            "song \"T\"\ntrack \"Clock\"\n type clock_and_transport\n",
            "section \"A\"\n meter 4/4  tempo 120 beats/min\n line \"Drums\" | C4:q D4 E4 F4 |\n"
        ),
        expected_message: "mistake.song line 6: \"Drums\" is not a track of the song. Declare it \
                           with track \"Drums\" before the sections, or fix the name",
    },
    MistakeCase {
        what: "a chord on the mono bass",
        kind: Kind::Song,
        text: concat!(
            "song \"T\"\ntrack \"Clock\"\n type clock_and_transport\n",
            "track \"Keys\"\n type poly_voice\n clef grand\n lead\n",
            "track \"Bass\"\n type mono_voice\n clef bass\n",
            "section \"A\"\n key C major  meter 4/4  tempo 120 beats/min\n",
            " line \"Bass\" | C2:q / / V7 |\n"
        ),
        expected_message: "mistake.song line 13: line \"Bass\": bar 1 beat 4: track \"Bass\" cannot \
                           play that: a mono voice plays one note at a time; write a single note \
                           such as C2",
    },
    MistakeCase {
        what: "a first note with no length",
        kind: Kind::Song,
        text: concat!(
            "song \"T\"\ntrack \"Clock\"\n type clock_and_transport\n",
            "track \"Keys\"\n type poly_voice\n clef grand\n lead\n",
            "section \"A\"\n key C major  meter 4/4  tempo 120 beats/min\n",
            " line \"Keys\" | I / / / |\n"
        ),
        expected_message: "mistake.song line 10: line \"Keys\": \"I\" has no length, and nothing \
                           before it in this line gives one. Write it, e.g. I:q",
    },
    MistakeCase {
        what: "a line in the old order, length first",
        kind: Kind::Song,
        text: concat!(
            "song \"T\"\ntrack \"Clock\"\n type clock_and_transport\n",
            "track \"Keys\"\n type poly_voice\n clef grand\n lead\n",
            "section \"A\"\n key C major  meter 4/4  tempo 120 beats/min\n",
            " line \"Keys\" | q:I q q q |\n"
        ),
        expected_message: "mistake.song line 10: line \"Keys\": \"q:I\": a song line writes what it \
                           plays first, then its length, e.g. C4:q, I:q or r:q, not q:C4",
    },
    MistakeCase {
        what: "chords in a section with no key",
        kind: Kind::Song,
        text: concat!(
            "song \"T\"\ntrack \"Clock\"\n type clock_and_transport\n",
            "track \"Keys\"\n type poly_voice\n clef grand\n lead\n",
            "section \"A\"\n meter 4/4  tempo 120 beats/min\n line \"Keys\" | I:q / / / |\n"
        ),
        expected_message: "mistake.song line 8: section \"A\" has chords but no key. Add key to the \
                           section, e.g. key C major",
    },
    MistakeCase {
        what: "a song with no clock and transport track",
        kind: Kind::Song,
        text: concat!(
            "song \"T\"\ntrack \"Beat\"\n type mono_voice\n clef treble\n",
            "section \"A\"\n meter 4/4  tempo 120 beats/min\n line \"Beat\" | C4:q / / / |\n"
        ),
        expected_message: "mistake.song line 1: the song has 0 clock and transport tracks. It needs \
                           exactly 1: add a track with type clock_and_transport",
    },
    MistakeCase {
        what: "a chord the notation does not know",
        kind: Kind::Song,
        text: concat!(
            "song \"T\"\ntrack \"Clock\"\n type clock_and_transport\n",
            "track \"Keys\"\n type poly_voice\n clef grand\n lead\n",
            "section \"A\"\n key C major  meter 4/4  tempo 120 beats/min\n",
            " line \"Keys\" | IV9b:q / / / |\n"
        ),
        expected_message: "mistake.song line 10: line \"Keys\": bar 1 beat 1: \"IV9b\" is not a chord. \
                           After an upper case numeral write nothing, 7, M7 or maj7, + or 7b9",
    },
    MistakeCase {
        what: "a studio block with no port",
        kind: Kind::Studio,
        text: "studio \"S\"\ntrack \"Keys\"\n  channel 1\n",
        expected_message: "mistake.studio line 2: \"Keys\" has no port. Add a line such as port \"Harmony\"",
    },
    MistakeCase {
        what: "a duration without its unit",
        kind: Kind::Studio,
        text: "studio \"S\"\nport \"P\"\ntrack \"Bass\"\n  port \"P\"  channel 2  retrigger_gap 2\n",
        expected_message: "mistake.studio line 4: write retrigger_gap with its unit, e.g. \
                           retrigger_gap 2 ms",
    },
];

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let song_file: &str = arguments.first().map_or(DEFAULT_SONG_FILE, String::as_str);
    let studio_file: &str = arguments.get(1).map_or(DEFAULT_STUDIO_FILE, String::as_str);

    let song: Option<Song> = load_and_print_song(song_file);
    let studio: Option<StudioConfiguration> = load_studio(studio_file);
    let coverage_ok: bool = match (&song, &studio) {
        (Some(song), Some(studio)) => check_coverage(song, studio, studio_file),
        _ => false,
    };
    let mistakes_ok: bool = check_mistakes();
    if song.is_some() && studio.is_some() && coverage_ok && mistakes_ok {
        println!("\nAll good.");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn read_file(file_name: &str, what: &str) -> Option<String> {
    match std::fs::read_to_string(file_name) {
        Ok(text) => Some(text),
        Err(error) => {
            eprintln!(
                "ERROR: cannot read the {what} {file_name}: {error}. Run from the project \
                 folder, or give the file as the argument."
            );
            None
        }
    }
}

fn print_errors(errors: &[ParseError]) {
    for (index, error) in errors.iter().enumerate() {
        eprintln!("  Issue {}: {error}", index + 1);
    }
}

fn load_and_print_song(song_file: &str) -> Option<Song> {
    let text: String = read_file(song_file, "song")?;
    match parse_song(song_file, &text) {
        Ok(song) => {
            print_song(&song);
            Some(song)
        }
        Err(errors) => {
            eprintln!("ERROR: {song_file} is not valid; nothing was applied.");
            print_errors(&errors);
            None
        }
    }
}

fn load_studio(studio_file: &str) -> Option<StudioConfiguration> {
    let text: String = read_file(studio_file, "studio")?;
    match parse_studio(studio_file, &text) {
        Ok(studio) => {
            println!(
                "\nStudio \"{}\": {} ports, {} track entries, {} destinations.",
                studio.name(),
                studio.port_names().len(),
                studio.tracks().len(),
                studio.destinations().len()
            );
            Some(studio)
        }
        Err(errors) => {
            eprintln!("ERROR: {studio_file} is not valid; nothing was applied.");
            print_errors(&errors);
            None
        }
    }
}

fn check_coverage(song: &Song, studio: &StudioConfiguration, studio_file: &str) -> bool {
    let unmet: Vec<UnmetNeed> = studio.unmet_needs(song);
    if unmet.is_empty() {
        println!(
            "The studio covers every track of \"{}\" and every cue destination it uses.",
            song.name()
        );
        return true;
    }
    eprintln!(
        "ERROR: {studio_file} does not cover \"{}\"; nothing was applied.",
        song.name()
    );
    for (index, need) in unmet.iter().enumerate() {
        eprintln!("  Issue {}: {studio_file}: {need}", index + 1);
    }
    false
}

fn print_song(song: &Song) {
    println!("Song \"{}\"", song.name());
    println!("\nTracks, in the order they are shown:");
    for track in song.tracks() {
        println!("  {:<8} {}", track.name(), describe_track(track));
    }
    for section in song.sections() {
        print_section(song, section);
    }
}

fn describe_track(track: &Track) -> String {
    match track.kind() {
        TrackKind::Voice {
            voice,
            clef,
            lead,
            home_octave,
            voice_layer_names,
        } => {
            let voice_text: &str = match voice {
                VoiceKind::Poly => "poly voice (chords and notes)",
                VoiceKind::Mono => "mono voice (one note at a time)",
            };
            let lead_text: &str = if *lead {
                ", lead: its chords drive the key wheel"
            } else {
                ""
            };
            let octave_text: String =
                home_octave.map_or_else(String::new, |octave| format!(", home octave {octave}"));
            let mut names_text: String = String::new();
            for named in voice_layer_names {
                let named: &VoiceLayerName = named;
                let _ = write!(
                    names_text,
                    ", voice layer {} named \"{}\"",
                    named.number().value(),
                    named.name()
                );
            }
            format!(
                "{voice_text}, {} clef{octave_text}{lead_text}{names_text}",
                clef_name(*clef)
            )
        }
        TrackKind::ClockAndTransport => String::from("clock and transport"),
        TrackKind::CueControl { cues } => {
            let mut text: String = String::from("cue control");
            for cue in cues {
                let destination: String = match cue.destination() {
                    CueDestination::Named(name) => format!("destination \"{name}\""),
                    CueDestination::VoiceTrack(name) => format!("voice track \"{name}\""),
                };
                let _ = write!(
                    text,
                    "\n           cue \"{}\" → {destination}: {}",
                    cue.name(),
                    describe_message(cue.message())
                );
            }
            text
        }
    }
}

fn clef_name(clef: Clef) -> &'static str {
    match clef {
        Clef::Treble => "treble",
        Clef::Bass => "bass",
        Clef::Alto => "alto",
        Clef::Tenor => "tenor",
        Clef::Percussion => "percussion",
        Clef::Grand => "grand (treble and bass braced)",
    }
}

fn describe_message(message: CueMessage) -> String {
    match message {
        CueMessage::Start => String::from("Start"),
        CueMessage::Stop => String::from("Stop"),
        CueMessage::Continue => String::from("Continue"),
        CueMessage::Note { note, velocity } => {
            format!("note {} at velocity {}", note.value(), velocity.value())
        }
        CueMessage::ControlChange { controller, value } => {
            format!("controller {} = {}", controller.value(), value.value())
        }
        CueMessage::ProgramChange { program } => format!("program {}", program.value()),
    }
}

fn print_section(song: &Song, section: &Section) {
    let key_text: String = section
        .key()
        .map_or_else(|| String::from("no key"), |key| key.to_string());
    let meter = section.time_signature();
    println!(
        "\nSection \"{}\"{}: {} {}{}, {}/{} time, {} quarter notes per minute, {}",
        section.name(),
        if section.count_in() {
            " (count-in)"
        } else {
            ""
        },
        section.bar_count(),
        if section.bar_count() == 1 {
            "bar"
        } else {
            "bars"
        },
        if section.repeat_count() > 1 {
            format!(", played {} times", section.repeat_count())
        } else {
            String::new()
        },
        meter.pulses_per_bar(),
        meter.pulses_per_whole_note(),
        section.tempo().milli_quarters_per_minute() / 1000,
        key_text,
    );
    // The tracks in the order the song gives them, not the order the lines
    // were written; a track's voice layers in the order they are written.
    for track in song.tracks() {
        let lines: Vec<&Line> = section
            .lines()
            .iter()
            .filter(|line: &&Line| line.track_name() == track.name())
            .collect();
        if lines.is_empty() {
            if !matches!(track.kind(), TrackKind::ClockAndTransport) {
                println!("  {}: rests for the whole section", track.name());
            }
            continue;
        }
        let several: bool = lines.len() > 1;
        for line in lines {
            print_line(track, line, several, section);
        }
    }
}

/// A track's line, headed by the track's name, and by its voice layer when
/// the track has several in the section or the line is not layer 1.
fn print_line(track: &Track, line: &Line, several: bool, section: &Section) {
    match line.role() {
        LineRole::VoiceLayer(number) if several || number != VoiceLayerNumber::FIRST => {
            let name: String = track.voice_layer_display_name(number);
            if name == number.value().to_string() {
                println!("  {}, voice layer {}:", line.track_name(), number.value());
            } else {
                println!(
                    "  {}, voice layer {} \"{name}\":",
                    line.track_name(),
                    number.value()
                );
            }
        }
        LineRole::VoiceLayer(_) | LineRole::CuePlacement => {
            println!("  {}:", line.track_name());
        }
    }
    for line_element in line.elements() {
        let element = line_element.element();
        let place: String = describe_place(element.start().ticks(), section.time_signature());
        let length: String = describe_length(element.length().ticks()).description;
        if element.kind() == ElementKind::Rest {
            println!("    {place}: {length} rest");
            continue;
        }
        println!(
            "    {place}: {length}{}{}",
            describe_marks(line_element),
            describe_contents(line_element.contents())
        );
    }
}

fn describe_marks(line_element: &LineElement) -> String {
    let marks = line_element.element().marks();
    let mut text: String = String::new();
    match marks.accent() {
        AccentLevel::Normal => {}
        AccentLevel::Accent => text.push_str(", accent"),
        AccentLevel::Marcato => text.push_str(", marcato"),
    }
    if marks.staccato() {
        text.push_str(", staccato");
    }
    if marks.legato() {
        text.push_str(", legato");
    }
    if line_element.element().tied_to_next() {
        text.push_str(", tied to the next");
    }
    text
}

fn describe_contents(contents: Option<&ElementContents>) -> String {
    match contents {
        None => String::new(),
        Some(ElementContents::Chord(chord)) => format!(" — chord {chord}"),
        Some(ElementContents::Notes(notes)) if notes.len() == 1 => format!(" — note {}", notes[0]),
        Some(ElementContents::Notes(notes)) => format!(" — notes {}", join_pitches(notes)),
        Some(ElementContents::ChordWithNotes { chord, notes }) => {
            format!(" — chord {chord} voiced {}", join_pitches(notes))
        }
        Some(ElementContents::Cue(name)) => format!(" — cue \"{name}\""),
    }
}

fn join_pitches(notes: &[vocabulary::pitch::Pitch]) -> String {
    let texts: Vec<String> = notes.iter().map(ToString::to_string).collect();
    format!("[{}]", texts.join(" "))
}

/// Feeds each bad file in and checks the one message that comes out.
fn check_mistakes() -> bool {
    println!("\nMistakes (each must give the one message shown):");
    let mut all_ok: bool = true;
    for case in &MISTAKE_CASES {
        let errors: Vec<ParseError> = match case.kind {
            Kind::Song => {
                let text: String = if case.text.starts_with("song") {
                    case.text.to_string()
                } else {
                    format!("{SONG_HEAD}{}", case.text)
                };
                parse_song("mistake.song", &text).err().unwrap_or_default()
            }
            Kind::Studio => parse_studio("mistake.studio", case.text)
                .err()
                .unwrap_or_default(),
        };
        let ok: bool = errors.len() == 1 && errors[0].to_string() == case.expected_message;
        println!("  {}: {}", case.what, if ok { "ok" } else { "WRONG" });
        if !ok {
            all_ok = false;
            eprintln!("    expected: {}", case.expected_message);
            if errors.is_empty() {
                eprintln!("    got:      (the file loaded)");
            }
            for error in &errors {
                eprintln!("    got:      {error}");
            }
        }
    }
    all_ok
}
