//! The `dynamics` line (Song Notation, section 5, "Dynamics, in rhythm
//! notation"): each note takes its velocity from the level in force where it
//! starts. Checked from text as the player would send it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::note_track::{NoteTrack, TimedNote};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;

const SONG: &str = include_str!("../../../songs_canonical/dynamics/dynamics.song");
const STUDIO: &str = include_str!("../../../songs_canonical/dynamics/dynamics.studio");
const RAMP_WITHOUT_TARGET: &str =
    include_str!("../../../songs_canonical/dynamics/dynamics_ramp_without_target.song");
const WORD_NOT_KNOWN: &str =
    include_str!("../../../songs_canonical/dynamics/dynamics_word_not_known.song");

/// Ticks in a 4/4 bar.
const BAR_TICKS: u64 = 4 * 5040;

/// The velocity of the note starting at the start of bar `bar` (counted from 0).
fn velocity_of_bar(bar: u64) -> u8 {
    let song: Song = parse_song("dynamics.song", SONG).unwrap();
    let studio = parse_studio("dynamics.studio", STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    plan.note_tracks
        .iter()
        .find(|track: &&NoteTrack| track.track_name() == "Keys")
        .unwrap()
        .notes()
        .iter()
        .find(|note: &&TimedNote| note.start_ticks == bar * BAR_TICKS)
        .unwrap()
        .velocity
        .value()
}

#[test]
fn the_six_levels_follow_the_table_at_normal_64() {
    let levels: Vec<u8> = (0..6).map(velocity_of_bar).collect();
    assert_eq!(levels, vec![6, 35, 50, 64, 93, 121]);
}

#[test]
fn a_crescendo_grows_in_a_straight_line_to_the_next_level() {
    // Bars 6 to 9: p (35), then a crescendo over two bars, then f (93).
    assert_eq!(velocity_of_bar(6), 35);
    assert_eq!(velocity_of_bar(7), 35);
    assert_eq!(velocity_of_bar(8), 64);
    assert_eq!(velocity_of_bar(9), 93);
}

#[test]
fn a_section_with_no_dynamics_line_plays_at_normal() {
    assert_eq!(velocity_of_bar(10), 64);
}

#[test]
fn a_studio_accent_is_absolute_and_a_soft_mark_is_measured_from_the_level() {
    // Bar 11: p. Bar 12: `>` with the studio accent 115, whatever the level.
    assert_eq!(velocity_of_bar(11), 35);
    assert_eq!(velocity_of_bar(12), 115);
    // Bar 13: `<` is 3/8 of the way from p (35) toward 1: 22.
    assert_eq!(velocity_of_bar(13), 22);
}

#[test]
fn a_crescendo_with_no_level_after_it_is_refused() {
    let errors = parse_song("ramp.song", RAMP_WITHOUT_TARGET).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message: String = errors[0].to_string();
    assert!(message.starts_with("ramp.song line 17:"), "{message}");
    assert!(message.contains("target level"), "{message}");
}

#[test]
fn a_word_that_is_not_a_level_is_refused_with_the_marks_that_exist() {
    let errors = parse_song("word.song", WORD_NOT_KNOWN).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message: String = errors[0].to_string();
    assert!(message.starts_with("word.song line 17:"), "{message}");
    assert!(message.contains("is not a dynamics mark"), "{message}");
}

const WITHIN_BAR: &str = include_str!("../../../songs_canonical/dynamics/dynamics_within_bar.song");
const RAMP_HAS_NO_TIME: &str =
    include_str!("../../../songs_canonical/dynamics/dynamics_ramp_has_no_time.song");

/// Ticks in a quarter note.
const QUARTER_TICKS: u64 = 5040;

/// The velocities of the Keys notes of bar `bar` (from 0) of
/// `dynamics_within_bar.song`, one per quarter note.
fn within_bar_velocities(bar: u64) -> Vec<u8> {
    let song: Song = parse_song("dynamics_within_bar.song", WITHIN_BAR).unwrap();
    let studio = parse_studio("dynamics.studio", STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let mut notes: Vec<(u64, u8)> = plan
        .note_tracks
        .iter()
        .find(|track: &&NoteTrack| track.track_name() == "Keys")
        .unwrap()
        .notes()
        .iter()
        .filter(|note: &&TimedNote| {
            (bar * BAR_TICKS..(bar + 1) * BAR_TICKS).contains(&note.start_ticks)
        })
        .map(|note: &TimedNote| (note.start_ticks, note.velocity.value()))
        .collect();
    notes.sort_unstable();
    notes.into_iter().map(|note| note.1).collect()
}

#[test]
fn within_bar_a_ramp_with_no_length_fills_the_bar_between_two_levels() {
    // pp (6) at beat 1 and ff (121) on beat 4, the last beat, so the four
    // notes of the bar cover the whole range; ff holds into the next bar.
    // The ramp is 3 quarters less a tick for pp: beat 2 is 6 plus
    // 115 × (5040 − 1) ÷ (15120 − 1), and so on.
    assert_eq!(within_bar_velocities(0), vec![6, 44, 83, 121]);
    assert_eq!(within_bar_velocities(1), vec![121]);
}

#[test]
fn within_bar_a_ramp_between_two_written_lengths_takes_the_time_between() {
    // pp for a quarter, a ramp through the half note, ff on beat 4.
    assert_eq!(within_bar_velocities(2), vec![6, 6, 64, 121]);
}

#[test]
fn within_bar_two_levels_with_no_ramp_jump() {
    assert_eq!(within_bar_velocities(3), vec![6, 6, 121, 121]);
}

#[test]
fn within_bar_a_diminuendo_fills_what_the_written_lengths_leave() {
    // ff at beat 1, down through beats 1 to 3 (the ramp's 3 quarters less a
    // tick), pp from beat 4.
    let velocities: Vec<u8> = within_bar_velocities(4);
    assert_eq!(velocities, vec![121, 83, 44, 6]);
    assert_eq!(QUARTER_TICKS * 4, BAR_TICKS);
}

#[test]
fn a_ramp_with_no_time_left_in_its_bar_is_refused() {
    let errors = parse_song("no_time.song", RAMP_HAS_NO_TIME).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message: String = errors[0].to_string();
    assert!(message.starts_with("no_time.song line 17:"), "{message}");
    assert!(message.contains("has no time"), "{message}");
}
