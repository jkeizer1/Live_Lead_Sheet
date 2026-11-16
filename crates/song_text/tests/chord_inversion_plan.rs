//! Chord inversions (Song Notation, section 4): `I[1stInversion]` has the
//! third lowest, and the notes that were below it move up one octave.
//! Checked from text as the player would send it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::note_track::{NoteTrack, TimedNote};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;

const SONG: &str = include_str!("../../../songs_canonical/chord_inversion/chord_inversion.song");
const STUDIO: &str =
    include_str!("../../../songs_canonical/chord_inversion/chord_inversion.studio");
const TOO_HIGH: &str =
    include_str!("../../../songs_canonical/chord_inversion/chord_inversion_too_high.song");

/// Ticks in a 4/4 bar.
const BAR_TICKS: u64 = 4 * 5040;

/// The MIDI notes sounding at the start of bar `bar` (counted from 0), lowest first.
fn notes_of_bar(bar: u64) -> Vec<u8> {
    let song: Song = parse_song("chord_inversion.song", SONG).unwrap();
    let studio = parse_studio("chord_inversion.studio", STUDIO).unwrap();
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
fn the_first_inversion_has_the_third_lowest() {
    let plain: Vec<u8> = notes_of_bar(0);
    let first: Vec<u8> = notes_of_bar(1);
    // Root, third, fifth become third, fifth, root an octave up.
    assert_eq!(first, vec![plain[1], plain[2], plain[0] + 12]);
}

#[test]
fn the_second_inversion_has_the_fifth_lowest() {
    let plain: Vec<u8> = notes_of_bar(0);
    let second: Vec<u8> = notes_of_bar(2);
    assert_eq!(second, vec![plain[2], plain[0] + 12, plain[1] + 12]);
}

#[test]
fn the_third_inversion_of_a_seventh_chord_has_the_seventh_lowest() {
    let plain: Vec<u8> = notes_of_bar(3);
    let third: Vec<u8> = notes_of_bar(4);
    assert_eq!(
        third,
        vec![plain[3], plain[0] + 12, plain[1] + 12, plain[2] + 12]
    );
}

#[test]
fn a_third_inversion_of_a_triad_is_refused() {
    let errors = parse_song("too_high.song", TOO_HIGH).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let message: String = errors[0].to_string();
    assert!(message.starts_with("too_high.song line 15:"), "{message}");
    assert!(
        message.contains("inversion 3 does not exist for this chord: the highest is 2"),
        "{message}"
    );
}
