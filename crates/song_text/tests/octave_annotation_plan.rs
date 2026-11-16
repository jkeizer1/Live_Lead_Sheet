//! Octave annotations (Song Notation, section 4): `+8va` and `-8va` move what
//! follows one octave from the clef's octave until changed; `loco` returns.
//! Checked from text as the player would send it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::note_track::{NoteTrack, TimedNote};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;

const SONG: &str =
    include_str!("../../../songs_canonical/octave_annotation/octave_annotation.song");
const STUDIO: &str =
    include_str!("../../../songs_canonical/octave_annotation/octave_annotation.studio");

/// Ticks in a 4/4 bar.
const BAR_TICKS: u64 = 4 * 5040;

/// The MIDI notes sounding at the start of bar `bar` (counted from 0), lowest first.
fn notes_of_bar(bar: u64) -> Vec<u8> {
    let song: Song = parse_song("octave_annotation.song", SONG).unwrap();
    let studio = parse_studio("octave_annotation.studio", STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let mut notes: Vec<u8> = plan
        .note_tracks
        .iter()
        .find(|track: &&NoteTrack| track.track_name() == "Keys")
        .unwrap()
        .notes()
        .iter()
        .filter(|note: &&TimedNote| note.start_ticks == bar * BAR_TICKS)
        .map(|note: &TimedNote| note.note.value())
        .collect();
    notes.sort_unstable();
    notes
}

#[test]
fn plus_8va_moves_the_bar_and_the_bars_after_it_up_an_octave() {
    assert_eq!(notes_of_bar(0), vec![60]);
    assert_eq!(notes_of_bar(1), vec![72]);
    assert_eq!(notes_of_bar(2), vec![72], "stays in effect until changed");
}

#[test]
fn minus_8va_moves_down_and_loco_returns_to_the_clefs_octave() {
    assert_eq!(notes_of_bar(3), vec![48]);
    assert_eq!(notes_of_bar(4), vec![60]);
}

#[test]
fn a_chord_moves_with_the_annotation() {
    let plain: Vec<u8> = notes_of_bar(5);
    let raised: Vec<u8> = notes_of_bar(6);
    let expected: Vec<u8> = plain.iter().map(|note: &u8| note + 12).collect();
    assert_eq!(raised, expected);
}

#[test]
fn writing_plus_8va_again_is_still_one_octave_up() {
    assert_eq!(notes_of_bar(7), vec![72]);
    assert_eq!(notes_of_bar(8), vec![72]);
}
