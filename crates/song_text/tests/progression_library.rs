//! The progression library (Song Notation, section 7): entries of Roman
//! numerals with a `mode` label, and the `chord_numeral_standard` line a
//! library with a minor entry needs.

// A test may unwrap: a failure here is the test failing.
#![allow(clippy::unwrap_used)]

use song_text::parse_error::ParseError;
use song_text::progression_library::{ProgressionLibrary, parse_progression_library};
use vocabulary::key::Mode;
use vocabulary::numeral_standard::NumeralStandard;

#[test]
fn a_library_with_a_minor_entry_and_its_standard_loads() {
    let library: ProgressionLibrary = parse_progression_library(
        "progressions.library",
        include_str!("../../../songs_canonical/progression_library/progressions.library"),
    )
    .unwrap();
    assert_eq!(
        library.numeral_standard(),
        Some(NumeralStandard::KeyDependent)
    );
    let names: Vec<&str> = library
        .progressions()
        .iter()
        .map(vocabulary::progression::Progression::name)
        .collect();
    assert_eq!(names, vec!["12-bar blues", "ii-V", "Andalusian cadence"]);
    let blues = &library.progressions()[0];
    assert_eq!(blues.idiom(), "Blues");
    assert_eq!(blues.kind(), "Full form");
    assert_eq!(blues.mode(), Mode::Major);
    assert_eq!(blues.bars().len(), 12);
    // The chords of a bar share it: ii-V is one bar of two chords.
    assert_eq!(library.progressions()[1].bars()[0].len(), 2);
    assert_eq!(library.progressions()[2].mode(), Mode::Minor);
}

#[test]
fn a_major_only_library_needs_no_standard() {
    let library: ProgressionLibrary = parse_progression_library(
        "major_only.library",
        include_str!("../../../songs_canonical/progression_library/major_only.library"),
    )
    .unwrap();
    assert_eq!(library.numeral_standard(), None);
    assert_eq!(library.progressions().len(), 1);
}

fn refused(file_name: &str, text: &str, expected: &str) -> ParseError {
    let errors: Vec<ParseError> = parse_progression_library(file_name, text).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].message().contains(expected),
        "expected {expected:?} in {:?}",
        errors[0].message()
    );
    errors.into_iter().next().unwrap()
}

#[test]
fn a_minor_progression_needs_the_standard() {
    let error: ParseError = refused(
        "minor_without_standard.library",
        include_str!("../../../songs_canonical/progression_library/minor_without_standard.library"),
        "chord_numeral_standard key_dependent or chord_numeral_standard major_referential",
    );
    // The progression's line, counting the two comment lines.
    assert_eq!(error.line_number(), 3);
}

#[test]
fn a_letter_chord_is_refused_for_now() {
    refused(
        "letter_chord.library",
        include_str!("../../../songs_canonical/progression_library/letter_chord.library"),
        "\"Dm7\" is a letter chord",
    );
}

#[test]
fn a_mode_other_than_major_or_minor_is_refused() {
    refused(
        "mode_dorian.library",
        include_str!("../../../songs_canonical/progression_library/mode_dorian.library"),
        "Write mode major or mode minor",
    );
}

#[test]
fn an_entry_needs_its_mode() {
    refused(
        "mode_missing.library",
        include_str!("../../../songs_canonical/progression_library/mode_missing.library"),
        "write mode major or mode minor",
    );
}

#[test]
fn the_standard_goes_at_the_top() {
    refused(
        "standard_after_progression.library",
        include_str!(
            "../../../songs_canonical/progression_library/standard_after_progression.library"
        ),
        "at the top of the file",
    );
}
