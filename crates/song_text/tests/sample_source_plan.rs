//! Sample sources: a rhythm line that takes what it plays from a
//! `sample_source` line (Song Notation, section 5), checked from text as
//! the player would send it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::note_track::{NoteTrack, TimedNote};
use player::sample_source::{HeldStretch, held_stretches};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::{Line, LineRole, Song, VoiceLayerNumber};

const SAMPLE_SOURCE_SONG: &str = include_str!("../../../songs/Sample_source.song");
const DEMO_STUDIO: &str = include_str!("../../../songs/Demo.studio");

/// Ticks in a quarter note, and in a 4/4 bar.
const QUARTER: u64 = 5040;
const BAR: u64 = 4 * QUARTER;

/// A one-track song in C major 4/4 with `lines` in its one section.
fn song_with(lines: &str) -> String {
    format!(
        "song \"T\"\ntrack \"Clock\"\n  type clock_and_transport\n\
         track \"Keys\"\n  type poly_voice\n  clef grand\n  lead\n\
         section \"A\"\n  key C major  meter 4/4  tempo 120 beats/min\n{lines}"
    )
}

fn errors_of(text: &str) -> Vec<String> {
    parse_song("test.song", text)
        .unwrap_err()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn source_line(source_bars: &str) -> Line {
    let text: String = song_with(&format!(
        "  line \"Keys\" sample_source {source_bars}\n  line \"Keys\" | :w | :w |\n"
    ));
    let song: Song = parse_song("test.song", &text).unwrap();
    song.sections()[0]
        .lines()
        .iter()
        .find(|line: &&Line| line.role() == LineRole::SampleSource(VoiceLayerNumber::FIRST))
        .unwrap()
        .clone()
}

#[test]
fn a_source_bar_is_shared_evenly_between_its_elements() {
    // 20160 ticks/bar ÷ 3 = 6720 ticks each.
    let source: Line = source_line("| I | IV IV V |");
    let places: Vec<(u64, u64)> = source
        .elements()
        .iter()
        .map(|line_element| {
            (
                line_element.element().start().ticks(),
                line_element.element().length().ticks(),
            )
        })
        .collect();
    assert_eq!(
        places,
        vec![
            (0, BAR),
            (BAR, 6720),
            (BAR + 6720, 6720),
            (BAR + 13_440, 6720)
        ]
    );
}

#[test]
fn an_element_over_one_chord_is_one_stretch() {
    let source: Line = source_line("| I | IV |");
    let stretches: Vec<HeldStretch> = held_stretches(&source, 0, BAR);
    assert_eq!(stretches.len(), 1);
    assert_eq!(stretches[0].length_ticks, BAR);
}

#[test]
fn an_element_follows_the_source_when_it_changes_while_sounding() {
    // A half note from beat 3 of bar 2 (10080 ticks into the bar) runs from
    // the second IV into V, which starts 13440 ticks into the bar.
    let source: Line = source_line("| I | IV IV V |");
    let start_ticks: u64 = BAR + 2 * QUARTER;
    let stretches: Vec<HeldStretch> = held_stretches(&source, start_ticks, 2 * QUARTER);
    assert_eq!(stretches.len(), 2, "{stretches:?}");
    assert_eq!(
        (stretches[0].start_ticks, stretches[0].length_ticks),
        (start_ticks, 3360)
    );
    assert_eq!(
        (stretches[1].start_ticks, stretches[1].length_ticks),
        (BAR + 13_440, 2 * QUARTER - 3360)
    );
    assert_ne!(stretches[0].contents, stretches[1].contents);
}

#[test]
fn the_same_chord_twice_is_not_a_change() {
    let source: Line = source_line("| I | IV IV V |");
    let stretches: Vec<HeldStretch> = held_stretches(&source, BAR, 13_440);
    assert_eq!(stretches.len(), 1, "{stretches:?}");
    assert_eq!(stretches[0].length_ticks, 13_440);
}

#[test]
fn an_element_takes_the_chord_held_when_it_starts() {
    // A quarter note on beat 4 of bar 2 is inside V, which started at 13440
    // ticks into the bar.
    let source: Line = source_line("| I | IV IV V |");
    let start_ticks: u64 = BAR + 3 * QUARTER;
    let stretches: Vec<HeldStretch> = held_stretches(&source, start_ticks, QUARTER);
    assert_eq!(stretches.len(), 1);
    assert_eq!(
        (stretches[0].start_ticks, stretches[0].length_ticks),
        (start_ticks, QUARTER)
    );
}

fn plan() -> SongPlan {
    let song: Song = parse_song("Sample_source.song", SAMPLE_SOURCE_SONG).unwrap();
    let studio = parse_studio("Demo.studio", DEMO_STUDIO).unwrap();
    plan_song(&song, &studio).unwrap()
}

fn notes_of<'a>(plan: &'a SongPlan, name: &str) -> &'a [TimedNote] {
    plan.note_tracks
        .iter()
        .find(|track: &&NoteTrack| track.track_name() == name)
        .unwrap()
        .notes()
}

/// (start, length) of each chord or note played, once per start and length.
fn places(notes: &[TimedNote]) -> Vec<(u64, u64)> {
    let mut places: Vec<(u64, u64)> = notes
        .iter()
        .map(|note: &TimedNote| (note.start_ticks, note.length_ticks))
        .collect();
    places.dedup();
    places
}

#[test]
fn the_keys_follow_the_chord_source_and_split_where_it_changes() {
    let plan: SongPlan = plan();
    let keys: &[TimedNote] = notes_of(&plan, "Keys");
    let first_pass: Vec<(u64, u64)> = places(keys)
        .into_iter()
        .filter(|&(start, _)| start < 4 * BAR)
        .collect();
    assert_eq!(
        first_pass,
        vec![
            // Bar 1, I: a half note, then two quarter notes.
            (0, 2 * QUARTER),
            (2 * QUARTER, QUARTER),
            (3 * QUARTER, QUARTER),
            // Bar 2, IV IV V: a half note on IV, then a half note that
            // starts on IV and moves to V a third of the way into the bar.
            (BAR, 2 * QUARTER),
            (BAR + 2 * QUARTER, 3360),
            (BAR + 13_440, 2 * QUARTER - 3360),
            // Bar 3, vi: four quarter notes.
            (2 * BAR, QUARTER),
            (2 * BAR + QUARTER, QUARTER),
            (2 * BAR + 2 * QUARTER, QUARTER),
            (2 * BAR + 3 * QUARTER, QUARTER),
            // Bar 4, V: a whole note.
            (3 * BAR, BAR),
        ]
    );
}

#[test]
fn the_chord_before_and_after_the_change_is_the_one_the_source_holds() {
    let plan: SongPlan = plan();
    let keys: &[TimedNote] = notes_of(&plan, "Keys");
    let notes_at = |start_ticks: u64| -> Vec<u8> {
        let mut found: Vec<u8> = keys
            .iter()
            .filter(|note: &&TimedNote| note.start_ticks == start_ticks)
            .map(|note: &TimedNote| note.note.value())
            .collect();
        found.sort_unstable();
        found
    };
    // C major (I), F major (IV) and G major (V), root position from C4,
    // F4 and G4.
    assert_eq!(notes_at(0), vec![60, 64, 67]);
    assert_eq!(notes_at(BAR + 2 * QUARTER), notes_at(BAR));
    assert_ne!(notes_at(BAR + 13_440), notes_at(BAR + 2 * QUARTER));
    assert_eq!(notes_at(BAR + 13_440), notes_at(3 * BAR));
}

#[test]
fn a_mono_bass_follows_its_note_source() {
    let plan: SongPlan = plan();
    let bass: &[TimedNote] = notes_of(&plan, "Bass");
    let first_pass: Vec<(u64, u8)> = bass
        .iter()
        .filter(|note: &&TimedNote| note.start_ticks < BAR * 2)
        .map(|note: &TimedNote| (note.start_ticks, note.note.value()))
        .collect();
    // C2 on each beat of bar 1. Bar 2 is F2 F2 G2, a third of the bar each
    // (G2 starts 13440 ticks in). Beats 1 and 2 are inside F2. Beat 3 starts
    // at 10080 ticks inside the second F2 and runs into G2, so it splits at
    // 13440 ticks. Beat 4 is inside G2. The Demo studio's octave_offset is 1
    // octaves, so C2 (MIDI 36) sounds as 48, F2 (41) as 53 and G2 (43) as 55.
    assert_eq!(
        first_pass,
        vec![
            (0, 48),
            (QUARTER, 48),
            (2 * QUARTER, 48),
            (3 * QUARTER, 48),
            (BAR, 53),
            (BAR + QUARTER, 53),
            (BAR + 2 * QUARTER, 53),
            (BAR + 13_440, 55),
            (BAR + 3 * QUARTER, 55),
        ]
    );
}

#[test]
fn following_the_source_is_legato_so_nothing_is_left_unapplied() {
    let plan: SongPlan = plan();
    assert!(
        plan.not_yet_applied.is_empty(),
        "{:?}",
        plan.not_yet_applied
    );
    // Each chord that ends because the source moved on is released after
    // the next chord starts: its notes are marked legato_to_next.
    let keys: &[TimedNote] = notes_of(&plan, "Keys");
    let legato_starts: Vec<u64> = keys
        .iter()
        .filter(|note: &&TimedNote| note.legato_to_next)
        .map(|note: &TimedNote| note.start_ticks)
        .collect();
    // Bar 2's half note from beat 3: IV (3 notes) ends where V starts.
    assert!(legato_starts.contains(&(BAR + 2 * QUARTER)));
    // A note that ends at its written end is not legato: bar 1's chord.
    assert!(
        keys.iter()
            .filter(|note: &&TimedNote| note.start_ticks == 0)
            .all(|note: &TimedNote| !note.legato_to_next)
    );
}

#[test]
fn a_pitch_shared_by_the_old_and_new_chord_is_held_not_struck_again() {
    // IV (F4 A4 C5) over the first half of the bar, vi (A4 C5 E5) over the
    // second: A4 and C5 are held through the whole bar, F4 ends legato where
    // E5 starts.
    let text: String =
        song_with("  line \"Keys\" sample_source | IV vi |\n  line \"Keys\" | :w |\n");
    let song: Song = parse_song("test.song", &text).unwrap();
    let studio = parse_studio("Demo.studio", DEMO_STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let mut found: Vec<(u8, u64, u64, bool)> = notes_of(&plan, "Keys")
        .iter()
        .map(|note: &TimedNote| {
            (
                note.note.value(),
                note.start_ticks,
                note.length_ticks,
                note.legato_to_next,
            )
        })
        .collect();
    found.sort_unstable();
    let half: u64 = BAR / 2;
    assert_eq!(
        found,
        vec![
            (65, 0, half, true),     // F4: IV only, released after E5 starts
            (69, 0, BAR, false),     // A4: in both chords, held
            (72, 0, BAR, false),     // C5: in both chords, held
            (76, half, half, false), // E5: vi only
        ]
    );
}

#[test]
fn a_rhythm_element_with_no_source_is_refused() {
    let errors: Vec<String> = errors_of(&song_with("  line \"Keys\" | :w |\n"));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("takes what it plays from a"),
        "{}",
        errors[0]
    );
}

#[test]
fn a_source_with_no_rhythm_line_is_refused() {
    let errors: Vec<String> = errors_of(&song_with("  line \"Keys\" sample_source | I |\n"));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("no rhythm line"), "{}", errors[0]);
}

#[test]
fn a_length_a_mark_a_tie_or_a_rest_on_a_source_is_refused() {
    for (written, expected) in [
        ("| I:q |", "no lengths, marks or ties"),
        ("| I> |", "no lengths, marks or ties"),
        ("| I~ |", "no lengths, marks or ties"),
        ("| r |", "not rests"),
    ] {
        let errors: Vec<String> = errors_of(&song_with(&format!(
            "  line \"Keys\" sample_source {written}\n  line \"Keys\" | :w |\n"
        )));
        assert_eq!(errors.len(), 1, "{written}: {errors:?}");
        assert!(errors[0].contains(expected), "{written}: {}", errors[0]);
        assert!(errors[0].starts_with("test.song line 10:"), "{}", errors[0]);
    }
}

#[test]
fn a_chord_with_no_length_on_an_ordinary_line_is_still_an_error() {
    let errors: Vec<String> = errors_of(&song_with("  line \"Keys\" | I IV V I |\n"));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("has no length"), "{}", errors[0]);
}

#[test]
fn a_rest_on_the_rhythm_line_closes_the_gate_so_a_change_inside_it_is_not_heard() {
    let text: String = song_with(
        "  line \"Keys\" sample_source | I | V |\n  line \"Keys\" | :q :q r:h | r:h :h |\n",
    );
    let song: Song = parse_song("test.song", &text).unwrap();
    let studio = parse_studio("Demo.studio", DEMO_STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let keys: &[TimedNote] = notes_of(&plan, "Keys");
    assert_eq!(
        places(keys),
        vec![
            (0, QUARTER),
            (QUARTER, QUARTER),
            (BAR + 2 * QUARTER, 2 * QUARTER)
        ]
    );
}

#[test]
fn a_tied_rhythm_element_samples_once_then_holds_one_gate_and_follows_the_source() {
    // Two tied half notes make one whole-bar note over IV IV V. It samples
    // the source once, at its start, and the tie adds no second start: the
    // only other note is the follow to V at 13440 ticks into the bar.
    let text: String =
        song_with("  line \"Keys\" sample_source | IV IV V |\n  line \"Keys\" | :h~ :h |\n");
    let song: Song = parse_song("test.song", &text).unwrap();
    let studio = parse_studio("Demo.studio", DEMO_STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    assert_eq!(
        places(notes_of(&plan, "Keys")),
        vec![(0, 13_440), (13_440, BAR - 13_440)]
    );
}
