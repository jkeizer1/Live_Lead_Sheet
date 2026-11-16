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
    assert_eq!(levels, vec![17, 40, 52, 64, 88, 111]);
}

#[test]
fn a_crescendo_grows_in_a_straight_line_to_the_next_level() {
    // Bars 6 to 9: p (40), then a crescendo over two bars, then f (88).
    assert_eq!(velocity_of_bar(6), 40);
    assert_eq!(velocity_of_bar(7), 40);
    assert_eq!(velocity_of_bar(8), 64);
    assert_eq!(velocity_of_bar(9), 88);
}

#[test]
fn a_section_with_no_dynamics_line_plays_at_normal() {
    assert_eq!(velocity_of_bar(10), 64);
}

#[test]
fn a_studio_accent_is_absolute_and_a_soft_mark_is_measured_from_the_level() {
    // Bar 11: p. Bar 12: `>` with the studio accent 115, whatever the level.
    assert_eq!(velocity_of_bar(11), 40);
    assert_eq!(velocity_of_bar(12), 115);
    // Bar 13: `<` is 3/8 of the way from p (40) toward 1: 25.
    assert_eq!(velocity_of_bar(13), 25);
}

#[test]
fn a_crescendo_with_no_level_after_it_is_refused() {
    let errors = parse_song("ramp.song", RAMP_WITHOUT_TARGET).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message: String = errors[0].to_string();
    assert!(message.starts_with("ramp.song line 16:"), "{message}");
    assert!(message.contains("target level"), "{message}");
}

#[test]
fn a_word_that_is_not_a_level_is_refused_with_the_marks_that_exist() {
    let errors = parse_song("word.song", WORD_NOT_KNOWN).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message: String = errors[0].to_string();
    assert!(message.starts_with("word.song line 16:"), "{message}");
    assert!(message.contains("is not a dynamics mark"), "{message}");
}
