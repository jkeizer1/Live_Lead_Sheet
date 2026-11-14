//! The `_` legato mark on a mono voice (Song Notation, section 3), checked
//! from text as the player would send it: a marked note is released after
//! the note it joins starts, with no gap.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::note_track::{NoteTrack, TimedNote};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::song::Song;

const LEGATO_SONG: &str = include_str!("../../../songs_canonical/legato_mark/legato_mark.song");
const LEGATO_STUDIO: &str = include_str!("../../../songs_canonical/legato_mark/legato_mark.studio");

/// Ticks in a quarter note, an eighth note and a 4/4 bar.
const QUARTER: u64 = 5040;
const EIGHTH: u64 = QUARTER / 2;
const BAR: u64 = 4 * QUARTER;

/// The planned notes of the song's bars from `first_bar` (counted from 0)
/// for `bar_count` bars: one section of `songs_canonical/legato_mark/legato_mark.song`.
fn bass_notes(first_bar: u64, bar_count: u64) -> Vec<TimedNote> {
    let song: Song = parse_song("legato_mark.song", LEGATO_SONG).unwrap();
    let studio = parse_studio("legato_mark.studio", LEGATO_STUDIO).unwrap();
    let plan: SongPlan = plan_song(&song, &studio).unwrap();
    let from_ticks: u64 = first_bar * BAR;
    let to_ticks: u64 = (first_bar + bar_count) * BAR;
    plan.note_tracks
        .iter()
        .find(|track: &&NoteTrack| track.track_name() == "Bass")
        .unwrap()
        .notes()
        .iter()
        .filter(|note: &&TimedNote| (from_ticks..to_ticks).contains(&note.start_ticks))
        .map(|note: &TimedNote| TimedNote {
            // Relative to the section's first bar.
            start_ticks: note.start_ticks - from_ticks,
            ..*note
        })
        .collect()
}

/// (start, length, joined to the next) of each note.
fn places(notes: &[TimedNote]) -> Vec<(u64, u64, bool)> {
    notes
        .iter()
        .map(|note: &TimedNote| (note.start_ticks, note.length_ticks, note.legato_to_next))
        .collect()
}

#[test]
fn a_marked_note_is_joined_to_the_note_that_follows() {
    let notes: Vec<TimedNote> = bass_notes(0, 1);
    assert_eq!(
        places(&notes),
        vec![
            (0, QUARTER, true),
            (QUARTER, QUARTER, false),
            (2 * QUARTER, 2 * QUARTER, false),
        ]
    );
}

#[test]
fn unmarked_notes_are_not_joined() {
    let notes: Vec<TimedNote> = bass_notes(1, 1);
    assert!(notes.iter().all(|note: &TimedNote| !note.legato_to_next));
}

#[test]
fn a_marked_note_before_a_rest_has_nothing_to_join() {
    let notes: Vec<TimedNote> = bass_notes(2, 1);
    assert_eq!(
        places(&notes),
        vec![
            (0, EIGHTH, false),
            (QUARTER, QUARTER, false),
            (2 * QUARTER, 2 * QUARTER, false),
        ]
    );
}

#[test]
fn a_marked_note_before_the_next_bar_joins_the_first_note_of_that_bar() {
    let notes: Vec<TimedNote> = bass_notes(3, 2);
    assert_eq!(
        places(&notes),
        vec![(2 * QUARTER, 2 * QUARTER, true), (BAR, BAR, false)]
    );
}

#[test]
fn a_marked_note_before_a_note_of_the_same_pitch_makes_one_longer_note() {
    // A second note on of D2, then a note off of D2, would end the second
    // note: the two become one note, a half note, with one gate.
    let notes: Vec<TimedNote> = bass_notes(5, 1);
    assert_eq!(
        places(&notes),
        vec![(0, 2 * QUARTER, false), (2 * QUARTER, 2 * QUARTER, false)]
    );
}

#[test]
fn a_chain_of_marked_notes_of_one_pitch_ends_with_the_last_mark() {
    // D2 D2 D2 joined, then joined on to A2: one dotted half note of D2.
    let notes: Vec<TimedNote> = bass_notes(6, 1);
    assert_eq!(
        places(&notes),
        vec![(0, 3 * QUARTER, true), (3 * QUARTER, QUARTER, false)]
    );
}

#[test]
fn staccato_cuts_the_note_short_so_it_touches_nothing() {
    let notes: Vec<TimedNote> = bass_notes(7, 1);
    assert_eq!(places(&notes)[0], (0, QUARTER / 2, false));
}
