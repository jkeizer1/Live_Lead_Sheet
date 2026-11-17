//! Tempo marks on the lead line (Song Notation, "Tempo marks on the lead
//! line" and "Tempo ramp"): steps, ramps, carrying and repeats. Checked from
//! text as the loader and the planner would take it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::song_plan::{SongPlan, TempoChange, plan_song};
use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;

const SONG: &str = include_str!("../../../songs_canonical/tempo_marks/tempo_marks.song");
const STUDIO: &str = include_str!("../../../songs_canonical/tempo_marks/tempo_marks.studio");

fn song() -> Song {
    parse_song("tempo_marks.song", SONG).unwrap()
}

fn plan() -> SongPlan {
    let studio = parse_studio("tempo_marks.studio", STUDIO).unwrap();
    plan_song(&song(), &studio).unwrap()
}

/// (position in ticks, tempo in mq/min) of each change.
fn changes() -> Vec<(u64, u64)> {
    plan()
        .tempo_changes
        .iter()
        .map(|change: &TempoChange| {
            (
                change.position.ticks(),
                change.tempo.milli_quarters_per_minute(),
            )
        })
        .collect()
}

#[test]
fn a_step_changes_the_tempo_at_its_bar() {
    // "Steps" has 4/4 bars of 20160 ticks; the mark is in bar 3.
    assert_eq!(plan().tempo.milli_quarters_per_minute(), 100_000);
    assert_eq!(changes()[0], (40_320, 120_000));
}

#[test]
fn a_section_that_writes_no_tempo_carries_the_tempo_in_force_at_the_end() {
    let song: Song = song();
    assert_eq!(
        song.sections()[1].tempo().milli_quarters_per_minute(),
        120_000
    );
    // The ramp section ends on the ramp's end tempo.
    assert_eq!(
        song.sections()[2].end_tempo().milli_quarters_per_minute(),
        60_000
    );
}

#[test]
fn a_ramp_makes_a_change_at_every_clock_pulse_and_then_holds() {
    let changes: Vec<(u64, u64)> = changes();
    // "Ramp" starts at 120960 ticks; the ramp is in its bar 2, so it starts
    // at 141120 ticks, and 2 bars of 4/4 are 40320 ticks: 192 clock pulses
    // of 210 ticks. The first pulse is at the start tempo and is no change.
    let in_ramp: Vec<&(u64, u64)> = changes
        .iter()
        .filter(|(position, _)| (141_120..=181_440).contains(position))
        .collect();
    assert_eq!(in_ramp.len(), 192);
    // 120000 mq/min - 60000 mq/min over 192 pulses is 312.5 mq/min a pulse,
    // rounded half away from zero to a whole mq/min (312.5 becomes 313).
    assert_eq!(*in_ramp[0], (141_330, 119_687));
    assert_eq!(*in_ramp[191], (181_440, 60_000));
    // Nothing changes between the end of the ramp and the next section.
    assert!(
        !changes
            .iter()
            .any(|(position, _)| *position > 181_440 && *position < 201_600)
    );
    // Falling all the way: each tempo is lower than the one before.
    assert!(in_ramp.windows(2).all(|pair| pair[1].1 < pair[0].1));
}

#[test]
fn a_repeated_section_starts_again_at_its_own_tempo() {
    let changes: Vec<(u64, u64)> = changes();
    // "Again" starts at 201600 ticks and its 2 bars are 40320 ticks. Bar 2
    // of each play is at 80 beats/min.
    let after_ramp: Vec<&(u64, u64)> = changes.iter().filter(|(p, _)| *p >= 201_600).collect();
    assert_eq!(
        after_ramp,
        vec![
            &(201_600, 100_000),
            &(221_760, 80_000),
            &(241_920, 100_000),
            &(262_080, 80_000)
        ]
    );
}

fn refused(file_name: &str, text: &str, expected: &str) -> ParseError {
    let errors: Vec<ParseError> = parse_song(file_name, text).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].message().contains(expected),
        "expected {expected:?} in {:?}",
        errors[0].message()
    );
    errors.into_iter().next().unwrap()
}

#[test]
fn mark_on_bass() {
    refused(
        "mark_on_bass.song",
        include_str!("../../../songs_canonical/tempo_marks/mark_on_bass.song"),
        "first voice layer of the lead track",
    );
}

#[test]
fn mark_on_second_layer() {
    refused(
        "mark_on_second_layer.song",
        include_str!("../../../songs_canonical/tempo_marks/mark_on_second_layer.song"),
        "first voice layer of the lead track",
    );
}

#[test]
fn two_marks_in_a_bar() {
    refused(
        "two_marks_in_a_bar.song",
        include_str!("../../../songs_canonical/tempo_marks/two_marks_in_a_bar.song"),
        "a bar holds one tempo mark",
    );
}

#[test]
fn mark_inside_ramp() {
    refused(
        "mark_inside_ramp.song",
        include_str!("../../../songs_canonical/tempo_marks/mark_inside_ramp.song"),
        "a tempo mark in a bar the ramp from bar 1 covers",
    );
}

#[test]
fn ramp_past_section_end() {
    let error: ParseError = refused(
        "ramp_past_section_end.song",
        include_str!("../../../songs_canonical/tempo_marks/ramp_past_section_end.song"),
        "the tempo ramp runs 3 bars but the section has 2 left",
    );
    assert!(error.message().contains("bar 3"), "{}", error.message());
}

#[test]
fn ramp_over_no_bars() {
    refused(
        "ramp_over_no_bars.song",
        include_str!("../../../songs_canonical/tempo_marks/ramp_over_no_bars.song"),
        "a ramp over 0 bars never moves",
    );
}

#[test]
fn tempo_without_unit() {
    refused(
        "tempo_without_unit.song",
        include_str!("../../../songs_canonical/tempo_marks/tempo_without_unit.song"),
        "needs its unit",
    );
}
