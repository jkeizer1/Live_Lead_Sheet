//! Minor keys (`minor_key_treatment_decision.md`, Conclusion): a key is a
//! tonic and major or minor, so a song in any key loads.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use player::song_plan::plan_song;
use song_text::parse_error::ParseError;
use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use vocabulary::key::Mode;
use vocabulary::song::Song;

const NOTES: &str = include_str!("../../../songs_canonical/minor_key/minor_key_notes.song");
const WITHOUT_STANDARD: &str =
    include_str!("../../../songs_canonical/minor_key/numerals_without_standard.song");
const KEY_DEPENDENT: &str =
    include_str!("../../../songs_canonical/minor_key/numerals_key_dependent.song");
const MAJOR_REFERENTIAL: &str =
    include_str!("../../../songs_canonical/minor_key/numerals_major_referential.song");
const DORIAN: &str = include_str!("../../../songs_canonical/minor_key/key_dorian.song");
const STUDIO: &str = include_str!("../../../songs_canonical/minor_key/minor_key.studio");

#[test]
fn a_song_in_minor_keys_loads_and_plans() {
    let song: Song = parse_song("minor_key_notes.song", NOTES).unwrap();
    let keys: Vec<String> = song
        .sections()
        .iter()
        .map(|section| section.key().unwrap().to_string())
        .collect();
    assert_eq!(keys, vec!["C minor", "A minor"]);
    assert_eq!(song.sections()[0].key().unwrap().mode(), Mode::Minor);
    let studio = parse_studio("minor_key.studio", STUDIO).unwrap();
    plan_song(&song, &studio).unwrap();
}

/// The root of each chord of the Keys line, as a pitch class (0 is C): the
/// lowest note sounding at each start.
fn roots_of(file_name: &str, text: &str) -> Vec<u8> {
    let song: Song = parse_song(file_name, text).unwrap();
    let studio = parse_studio("minor_key.studio", STUDIO).unwrap();
    let plan = plan_song(&song, &studio).unwrap();
    let keys = plan
        .note_tracks
        .iter()
        .find(|track| track.track_name() == "Keys")
        .unwrap();
    let mut starts: Vec<u64> = keys.notes().iter().map(|note| note.start_ticks).collect();
    starts.dedup();
    starts
        .iter()
        .map(|start| {
            keys.notes()
                .iter()
                .filter(|note| note.start_ticks == *start)
                .map(|note| note.note.value())
                .min()
                .unwrap()
                % 12
        })
        .collect()
}

#[test]
fn key_dependent_numerals_count_in_the_natural_minor_scale() {
    // C minor: i C, III E-flat, iv F, V G, VI A-flat, VII B-flat, vii° B, i C.
    assert_eq!(
        roots_of("numerals_key_dependent.song", KEY_DEPENDENT),
        vec![0, 3, 5, 7, 8, 10, 11, 0]
    );
}

#[test]
fn major_referential_numerals_count_in_the_major_scale() {
    // The same chords as above, written bIII, bVI and bVII.
    assert_eq!(
        roots_of("numerals_major_referential.song", MAJOR_REFERENTIAL),
        vec![0, 3, 5, 7, 8, 10, 11, 0]
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
fn numerals_in_a_minor_key_need_the_standard() {
    refused(
        "numerals_without_standard.song",
        WITHOUT_STANDARD,
        "chord_numeral_standard key_dependent or chord_numeral_standard major_referential",
    );
}

#[test]
fn an_unknown_standard_is_refused() {
    refused(
        "standard_unknown_word.song",
        include_str!("../../../songs_canonical/minor_key/standard_unknown_word.song"),
        "\"classical\" is not a chord numeral standard",
    );
}

#[test]
fn the_standard_goes_before_the_first_track() {
    refused(
        "standard_after_a_track.song",
        include_str!("../../../songs_canonical/minor_key/standard_after_a_track.song"),
        "on the line after song",
    );
}

#[test]
fn a_second_standard_line_is_refused() {
    refused(
        "standard_twice.song",
        include_str!("../../../songs_canonical/minor_key/standard_twice.song"),
        "a second chord_numeral_standard line",
    );
}

#[test]
fn a_mode_other_than_major_or_minor_is_refused() {
    let errors: Vec<ParseError> = parse_song("key_dorian.song", DORIAN).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].message().contains("not a mode V1 plays"),
        "{}",
        errors[0].message()
    );
}
