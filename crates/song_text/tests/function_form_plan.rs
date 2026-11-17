//! A function after the colon on a song line (Song Notation, section 3,
//! "Writing a function"): `C2:trp(q q q)`, repeated by `/`, with a rest in
//! it. Checked from text as the player would send it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::note_track::{NoteTrack, TimedNote};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;

const SONG: &str = include_str!("../../../songs_canonical/function_form/function_form.song");
const STUDIO: &str = include_str!("../../../songs_canonical/function_form/function_form.studio");

/// Ticks in a quarter note and a 4/4 bar.
const QUARTER: u64 = 5040;
const BAR: u64 = 4 * QUARTER;
/// One note of a triplet of quarters: a half note (2 × QUARTER) ÷ 3.
const TRIPLET: u64 = 2 * QUARTER / 3;

/// (start within the bar, length) of the Bass notes of bar `bar` (from 0).
fn notes_of_bar(bar: u64) -> Vec<(u64, u64)> {
    let song: Song = parse_song("function_form.song", SONG).unwrap();
    let studio = parse_studio("function_form.studio", STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let mut notes: Vec<(u64, u64)> = plan
        .note_tracks
        .iter()
        .find(|track: &&NoteTrack| track.track_name() == "Bass")
        .unwrap()
        .notes()
        .iter()
        .filter(|note: &&TimedNote| (bar * BAR..(bar + 1) * BAR).contains(&note.start_ticks))
        .map(|note: &TimedNote| (note.start_ticks - bar * BAR, note.length_ticks))
        .collect();
    notes.sort_unstable();
    notes
}

const NOTES_SONG: &str = include_str!("../../../songs_canonical/function_form/function_notes.song");

/// (start within the bar, MIDI note number) of the Bass notes of bar `bar`
/// (from 0) of `function_notes.song`.
fn notes_with_pitch_of_bar(bar: u64) -> Vec<(u64, u8)> {
    let song: Song = parse_song("function_notes.song", NOTES_SONG).unwrap();
    let studio = parse_studio("function_form.studio", STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let mut notes: Vec<(u64, u8)> = plan
        .note_tracks
        .iter()
        .find(|track: &&NoteTrack| track.track_name() == "Bass")
        .unwrap()
        .notes()
        .iter()
        .filter(|note: &&TimedNote| (bar * BAR..(bar + 1) * BAR).contains(&note.start_ticks))
        .map(|note: &TimedNote| (note.start_ticks - bar * BAR, note.note.value()))
        .collect();
    notes.sort_unstable();
    notes
}

/// Semitones from one note number to the next, in order.
fn steps(notes: &[(u64, u8)]) -> Vec<i16> {
    notes
        .windows(2)
        .map(|pair: &[(u64, u8)]| i16::from(pair[1].1) - i16::from(pair[0].1))
        .collect()
}

#[test]
fn notes_in_a_triplet_then_a_half_note() {
    let notes: Vec<(u64, u8)> = notes_with_pitch_of_bar(0);
    let starts: Vec<u64> = notes.iter().map(|note| note.0).collect();
    assert_eq!(starts, vec![0, TRIPLET, 2 * TRIPLET, 2 * QUARTER]);
    // F2 to A2 is up a major third, A2 to C2 down a major sixth, C2 to D2 up a tone.
    assert_eq!(steps(&notes), vec![4, -9, 2]);
}

#[test]
fn a_slash_plays_the_triplet_of_notes_again() {
    let notes: Vec<(u64, u8)> = notes_with_pitch_of_bar(1);
    let starts: Vec<u64> = notes.iter().map(|note| note.0).collect();
    assert_eq!(
        starts,
        vec![
            0,
            TRIPLET,
            2 * TRIPLET,
            2 * QUARTER,
            2 * QUARTER + TRIPLET,
            2 * QUARTER + 2 * TRIPLET
        ]
    );
    assert_eq!(notes[0].1, notes[3].1);
    assert_eq!(notes[2].1, notes[5].1);
}

#[test]
fn bare_lengths_play_what_is_before_the_colon_and_notes_play_their_own() {
    let notes: Vec<(u64, u8)> = notes_with_pitch_of_bar(2);
    assert_eq!(notes.len(), 4);
    // C2, E2, C2 (up a major third, then back), then G2.
    assert_eq!(steps(&notes)[..2], [4, -4]);
    assert_eq!(notes[3].0, 2 * QUARTER);
}

#[test]
fn three_eighths_fit_a_quarter() {
    let starts: Vec<u64> = notes_with_pitch_of_bar(3)
        .iter()
        .map(|note| note.0)
        .collect();
    // Each of the three is a third of a quarter note; G2 a quarter, A2 a half.
    assert_eq!(
        starts,
        vec![0, QUARTER / 3, 2 * QUARTER / 3, QUARTER, 2 * QUARTER]
    );
}

#[test]
fn fit_bar_keeps_the_proportions_of_its_notes() {
    let starts: Vec<u64> = notes_with_pitch_of_bar(4)
        .iter()
        .map(|note| note.0)
        .collect();
    // A quarter, an eighth, an eighth, a half note.
    assert_eq!(starts, vec![0, QUARTER, QUARTER + QUARTER / 2, 2 * QUARTER]);
}

#[test]
fn a_triplet_then_two_quarters() {
    let start_of_g2: u64 = 2 * QUARTER;
    let lengths: Vec<u64> = notes_of_bar(0).iter().map(|note| note.1).collect();
    assert_eq!(notes_of_bar(0).len(), 5);
    assert_eq!(notes_of_bar(0)[3].0, start_of_g2);
    // The triplet's notes may be shortened by the retrigger gap; none exceeds its place.
    assert!(lengths[..3].iter().all(|length| *length <= TRIPLET));
}

#[test]
fn a_slash_plays_the_triplet_again() {
    let notes: Vec<(u64, u64)> = notes_of_bar(1);
    assert_eq!(notes.len(), 6);
    assert_eq!(notes[3].0, 2 * QUARTER);
}

#[test]
fn a_rest_in_the_triplet_is_silent() {
    let notes: Vec<(u64, u64)> = notes_of_bar(2);
    // First and third of the triplet sound (the third after the rest, two
    // triplet notes in), then D2 holds a half note from the half bar.
    assert_eq!(notes.len(), 3);
    assert_eq!(notes[1].0, 2 * TRIPLET);
    assert_eq!(notes[2].0, 2 * QUARTER);
}
