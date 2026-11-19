//! Meter patterns (Song Notation, "Meter Control"): a `meter_control` line
//! gives a section a repeating pattern of meters. Checked from text as the
//! loader and the planner would take it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::song_plan::{SongPlan, TempoChange, plan_song};
use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;
use vocabulary::time_signature::TimeSignature;

const SONG: &str = include_str!("../../../songs_canonical/meter_control/meter_pattern.song");
const STUDIO: &str = include_str!("../../../songs_canonical/meter_control/meter_pattern.studio");

fn meter(pulses_per_bar: u8, pulses_per_whole_note: u8) -> TimeSignature {
    TimeSignature::new(pulses_per_bar, pulses_per_whole_note).unwrap()
}

fn song() -> Song {
    parse_song("meter_pattern.song", SONG).unwrap()
}

fn plan() -> SongPlan {
    let studio = parse_studio("meter_pattern.studio", STUDIO).unwrap();
    plan_song(&song(), &studio).unwrap()
}

fn bass_start_ticks() -> Vec<u64> {
    plan()
        .note_tracks
        .iter()
        .find(|track| track.track_name() == "Bass")
        .unwrap()
        .notes()
        .iter()
        .map(|note| note.start_ticks)
        .collect()
}

fn errors_of(file_name: &str, text: &str) -> Vec<ParseError> {
    parse_song(file_name, text).unwrap_err()
}

#[test]
fn a_section_takes_its_bars_meters_from_the_pattern() {
    let song: Song = song();
    let first = song.sections()[0].bar_meters();
    assert_eq!(first.pattern_meters(), &[meter(4, 4), meter(5, 4)]);
    // 4/4 is 20160 ticks and 5/4 is 25200 ticks: four bars are 90720 ticks.
    assert_eq!(song.sections()[0].length_ticks(), 90_720);
    assert_eq!(first.meter_of_bar(2), meter(4, 4));
    assert_eq!(first.meter_of_bar(3), meter(5, 4));
}

#[test]
fn a_section_writes_its_pattern_again_and_it_is_the_same() {
    let song: Song = song();
    assert_eq!(
        song.sections()[1].bar_meters(),
        song.sections()[0].bar_meters()
    );
}

#[test]
fn a_plain_meter_after_a_pattern_is_every_bars_meter() {
    let song: Song = song();
    assert!(song.sections()[2].bar_meters().is_uniform());
    assert_eq!(song.sections()[2].time_signature(), meter(3, 4));
}

#[test]
fn notes_start_where_each_bar_of_each_meter_starts() {
    let starts: Vec<u64> = bass_start_ticks();
    // "Four and five": 4 quarter notes, then 5, then 4, then 5.
    assert_eq!(starts[4], 20_160);
    assert_eq!(starts[9], 45_360);
    assert_eq!(starts[13], 65_520);
    // "Carried" starts at 90720 ticks and plays the pattern once: 4/4 then 5/4.
    assert_eq!(starts[18], 90_720);
    assert_eq!(starts[22], 90_720 + 20_160);
    // "Waltz" starts at 136080 ticks; its 3/4 bars are 15120 ticks.
    assert_eq!(starts[27], 136_080);
    assert_eq!(starts[30], 136_080 + 15_120);
    // "Six and nine" starts at 166320 ticks; 6/8 is 15120 ticks, 9/8 is 22680.
    assert_eq!(starts[33], 166_320);
    assert_eq!(starts[35], 166_320 + 15_120);
    assert_eq!(starts.len(), 38);
}

#[test]
fn the_song_is_as_long_as_its_bars() {
    // 166320 ticks to "Six and nine", then 15120 ticks (6/8) and 22680 ticks (9/8).
    assert_eq!(plan().stretch_length.ticks(), 204_120);
}

#[test]
fn the_tempo_changes_only_where_the_beat_changes() {
    let plan: SongPlan = plan();
    assert_eq!(plan.tempo.milli_quarters_per_minute(), 100_000);
    let changes: Vec<(u64, u64)> = plan
        .tempo_changes
        .iter()
        .map(|change: &TempoChange| {
            (
                change.position.ticks(),
                change.tempo.milli_quarters_per_minute(),
            )
        })
        .collect();
    // 4/4 to 5/4 to 3/4 keep a quarter-note beat: no change. The dotted
    // quarter beat of 6/8 and 9/8 at 100 beats/min is 150000 mq/min.
    assert_eq!(changes, vec![(166_320, 150_000)]);
}

#[test]
fn a_meter_control_track_needs_no_studio_block() {
    // The studio file has no block for the "Meter" track, and the plan works.
    let _plan: SongPlan = plan();
}

fn refused(file_name: &str, text: &str, expected: &str) -> ParseError {
    let errors: Vec<ParseError> = errors_of(file_name, text);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].message().contains(expected),
        "expected {expected:?} in {:?}",
        errors[0].message()
    );
    errors.into_iter().next().unwrap()
}

#[test]
fn meter_and_pattern() {
    refused(
        "meter_and_pattern.song",
        include_str!("../../../songs_canonical/meter_control/meter_and_pattern.song"),
        "writes a meter and also has a meter_control line",
    );
}

#[test]
fn pattern_beat_differs() {
    refused(
        "pattern_beat_differs.song",
        include_str!("../../../songs_canonical/meter_control/pattern_beat_differs.song"),
        "do not have the same beat",
    );
}

#[test]
fn pattern_length_not_multiple() {
    refused(
        "pattern_length_not_multiple.song",
        include_str!("../../../songs_canonical/meter_control/pattern_length_not_multiple.song"),
        "has 3 bars, but its meter pattern is 2 bars long",
    );
}

#[test]
fn bar_not_the_bars_meter() {
    let error: ParseError = refused(
        "bar_not_the_bars_meter.song",
        include_str!("../../../songs_canonical/meter_control/bar_not_the_bars_meter.song"),
        "bar 2",
    );
    assert!(
        error
            .message()
            .contains("bar 2 is 5/4 and short by a quarter note"),
        "{error}"
    );
    // The Keys line is on line 15 of the file, counting its two comment lines.
    assert_eq!(error.line_number(), 15);
}

#[test]
fn pattern_two_marks() {
    refused(
        "pattern_two_marks.song",
        include_str!("../../../songs_canonical/meter_control/pattern_two_marks.song"),
        "holds one mp mark",
    );
}

#[test]
fn pattern_with_end() {
    refused(
        "pattern_with_end.song",
        include_str!("../../../songs_canonical/meter_control/pattern_with_end.song"),
        "Nothing is written after it",
    );
}

#[test]
fn two_meter_lines() {
    refused(
        "two_meter_lines.song",
        include_str!("../../../songs_canonical/meter_control/two_meter_lines.song"),
        "a second meter_control line",
    );
}

#[test]
fn pattern_meter_with_slash() {
    refused(
        "pattern_meter_with_slash.song",
        include_str!("../../../songs_canonical/meter_control/pattern_meter_with_slash.song"),
        "with an underscore",
    );
}

#[test]
fn tempo_control_track() {
    refused(
        "tempo_control_track.song",
        include_str!("../../../songs_canonical/meter_control/tempo_control_track.song"),
        "tempo_control no longer exists",
    );
}
