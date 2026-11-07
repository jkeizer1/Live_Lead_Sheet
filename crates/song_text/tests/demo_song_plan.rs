//! The test song realised in the test studio: the notes and cues the
//! player would send, checked against the score.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::note_track::{NoteTrack, TimedNote};
use player::song_plan::{SongPlan, plan_song};
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::control_track::ControlTrack;
use vocabulary::duration::Duration;
use vocabulary::song::Song;

const DEMO_SONG: &str = include_str!("../../../songs/song_test_count_in_and_verse.song");
const DEMO_STUDIO: &str = include_str!("../../../songs/song_test_count_in_and_verse.studio");

/// Ticks in a quarter note.
const QUARTER: u64 = 5040;
const BAR: u64 = 4 * QUARTER;

fn plan() -> SongPlan {
    let song: Song = parse_song("song_test_count_in_and_verse.song", DEMO_SONG).unwrap();
    let studio = parse_studio("song_test_count_in_and_verse.studio", DEMO_STUDIO).unwrap();
    plan_song(&song, &studio).unwrap()
}

fn track<'a>(plan: &'a SongPlan, name: &str) -> &'a NoteTrack {
    plan.note_tracks
        .iter()
        .find(|track| track.track_name() == name)
        .unwrap()
}

fn starts_and_notes(track: &NoteTrack) -> Vec<(u64, u8)> {
    track
        .notes()
        .iter()
        .map(|note: &TimedNote| (note.start_ticks, note.note.value()))
        .collect()
}

#[test]
fn the_song_is_a_count_in_bar_and_two_passes_of_the_verse() {
    let plan: SongPlan = plan();
    // 1 count-in bar + 2 passes × 2 bars.
    assert_eq!(plan.stretch_length.ticks(), 5 * BAR);
    assert_eq!(plan.tempo.milli_quarters_per_minute(), 120_000);
}

#[test]
fn the_keys_play_the_chords_of_the_verse_after_the_count_in() {
    let plan: SongPlan = plan();
    let keys = starts_and_notes(track(&plan, "Keys"));
    // Count-in bar: nothing. First verse bar 1: four quarter notes of C major
    // (C4 E4 G4 = 60 64 67), then a quarter rest, D minor 7 on beat 2 for a
    // half note, G7 on beat 4.
    let first_pass: Vec<(u64, u8)> = keys
        .iter()
        .copied()
        .filter(|(start, _)| *start < 3 * BAR)
        .collect();
    assert_eq!(first_pass.first(), Some(&(BAR, 60)));
    assert!(first_pass.contains(&(BAR + 3 * QUARTER, 67)));
    // ii7 = D4 F4 A4 C5 on bar 2 beat 2.
    for note in [62, 65, 69, 72] {
        assert!(first_pass.contains(&(2 * BAR + QUARTER, note)), "{note}");
    }
    // V7 = G4 B4 D5 F5 on bar 2 beat 4.
    for note in [67, 71, 74, 77] {
        assert!(
            first_pass.contains(&(2 * BAR + 3 * QUARTER, note)),
            "{note}"
        );
    }
}

#[test]
fn the_bass_is_an_octave_lower_than_written_because_of_the_studio_offset() {
    let plan: SongPlan = plan();
    let bass = starts_and_notes(track(&plan, "Bass"));
    // C2 is note 36; the studio shifts the Bass down one octave: 24.
    assert_eq!(bass.first(), Some(&(BAR, 24)));
}

#[test]
fn the_start_cue_fires_on_each_pass_and_transport_brackets_the_song() {
    let plan: SongPlan = plan();
    let cues: &ControlTrack = plan
        .control_tracks
        .iter()
        .find(|track| track.name() == "Cues")
        .unwrap();
    let positions: Vec<u64> = cues
        .cues()
        .iter()
        .map(|cue| cue.position().ticks())
        .collect();
    assert_eq!(positions, vec![BAR, 3 * BAR]);
    let clock: &ControlTrack = plan
        .control_tracks
        .iter()
        .find(|track| track.name() == "Clock")
        .unwrap();
    let transport: Vec<u64> = clock
        .cues()
        .iter()
        .map(|cue| cue.position().ticks())
        .collect();
    assert_eq!(transport, vec![0, 5 * BAR]);
}

#[test]
fn the_retrigger_gap_is_applied_and_no_longer_reported() {
    let plan: SongPlan = plan();
    assert!(
        plan.not_yet_applied
            .iter()
            .all(|note: &String| !note.contains("retrigger_gap")),
        "{:?}",
        plan.not_yet_applied
    );
    assert_eq!(track(&plan, "Bass").retrigger_gap(), Duration::from_ms(2));
}
