//! One note for the same pitch in two voice layers (Song Notation, section
//! 5, "Voice layers"): overlapping equal pitches merge, notes that only
//! touch do not. Checked from text as the player would send it.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::note_track::{NoteTrack, TimedNote};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;

const SONG: &str = include_str!("../../../songs_canonical/layer_merge/layer_merge.song");
const STUDIO: &str = include_str!("../../../songs_canonical/layer_merge/layer_merge.studio");

/// Ticks in a quarter note and a 4/4 bar.
const QUARTER: u64 = 5040;
const BAR: u64 = 4 * QUARTER;

/// (start within the bar, length, MIDI note) of the notes of the section
/// that is bar `bar` (counted from 0), in start order.
fn notes_of_bar(bar: u64) -> Vec<(u64, u64, u8)> {
    let song: Song = parse_song("layer_merge.song", SONG).unwrap();
    let studio = parse_studio("layer_merge.studio", STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let mut notes: Vec<(u64, u64, u8)> = plan
        .note_tracks
        .iter()
        .find(|track: &&NoteTrack| track.track_name() == "Keys")
        .unwrap()
        .notes()
        .iter()
        .filter(|note: &&TimedNote| (bar * BAR..(bar + 1) * BAR).contains(&note.start_ticks))
        .map(|note: &TimedNote| {
            (
                note.start_ticks - bar * BAR,
                note.length_ticks,
                note.note.value(),
            )
        })
        .collect();
    notes.sort_unstable();
    notes
}

#[test]
fn overlapping_equal_pitches_of_two_layers_are_one_note() {
    // C4 is MIDI note 60: from beat 1 to the end of beat 3.
    assert_eq!(notes_of_bar(0), vec![(0, 3 * QUARTER, 60)]);
}

#[test]
fn equal_pitches_that_only_touch_stay_two_notes() {
    assert_eq!(
        notes_of_bar(1),
        vec![(0, QUARTER, 60), (QUARTER, QUARTER, 60)]
    );
}

#[test]
fn identical_notes_of_two_layers_are_one_note() {
    assert_eq!(notes_of_bar(2), vec![(0, BAR, 60)]);
}

#[test]
fn different_pitches_are_not_merged() {
    assert_eq!(
        notes_of_bar(3),
        vec![(0, 2 * QUARTER, 60), (QUARTER, 2 * QUARTER, 64)]
    );
}
