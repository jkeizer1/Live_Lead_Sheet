//! The written pitches of a chord named by a Roman numeral: the notes the
//! player sends (its plain voicing), each spelled with the letter its place
//! in the chord calls for. In C major `ii7` is D F A C, and in E♭ major
//! `V7` is B♭ D F A♭, never A♯.

use player::voicing::{VoicingError, plain_voicing};
use vocabulary::key::{Key, Mode};
use vocabulary::midi_values::NoteNumber;
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::pitch::{Accidental, NoteLetter, NoteName, Pitch};
use vocabulary::roman_chord::{ChordKind, Inversion, RomanChord};

const SEMITONES_PER_OCTAVE: i16 = 12;
const LETTERS_PER_OCTAVE: usize = 7;

const LETTERS: [NoteLetter; LETTERS_PER_OCTAVE] = [
    NoteLetter::C,
    NoteLetter::D,
    NoteLetter::E,
    NoteLetter::F,
    NoteLetter::G,
    NoteLetter::A,
    NoteLetter::B,
];

/// The chord's notes, lowest first, as written pitches.
///
/// # Errors
///
/// When the voicing falls outside the MIDI range.
pub fn chord_pitches(
    chord: RomanChord,
    key: Key,
    numeral_standard: Option<NumeralStandard>,
    home_octave: i8,
) -> Result<Vec<Pitch>, VoicingError> {
    let notes: Vec<NoteNumber> = plain_voicing(chord, key, numeral_standard, home_octave)?;
    let tonic_index: usize = letter_index(key.tonic().letter());
    let root_index: usize = tonic_index + usize::from(chord.degree().number()) - 1;
    let prefers_flats: bool = key_prefers_flats(key);
    Ok(notes
        .iter()
        .zip(letter_steps(chord.kind()))
        .map(|(note, step): (&NoteNumber, &usize)| {
            let letter: NoteLetter = LETTERS[(root_index + step) % LETTERS_PER_OCTAVE];
            spelled_as(*note, letter).unwrap_or_else(|| by_pitch_class(*note, prefers_flats))
        })
        .collect())
}

/// The chord's notes in an inversion, lowest first: the notes that were
/// below the new lowest note move up one octave.
///
/// # Errors
///
/// When the voicing falls outside the MIDI range.
pub fn inverted_chord_pitches(
    chord: RomanChord,
    key: Key,
    numeral_standard: Option<NumeralStandard>,
    home_octave: i8,
    inversion: Inversion,
) -> Result<Vec<Pitch>, VoicingError> {
    let pitches: Vec<Pitch> = chord_pitches(chord, key, numeral_standard, home_octave)?;
    let (below, rest): (&[Pitch], &[Pitch]) = pitches.split_at(usize::from(inversion.number()));
    let mut inverted: Vec<Pitch> = rest.to_vec();
    inverted.extend(
        below
            .iter()
            .map(|pitch: &Pitch| Pitch::new(pitch.name(), pitch.octave() + 1)),
    );
    Ok(inverted)
}

/// Letters above the root for each note of the plain voicing, in its
/// order: third 2, fifth 4, seventh 6, sixth 5; the second (or ninth, an
/// octave up) is 1 letter above, the fourth 3, the thirteenth 5 letters above.
fn letter_steps(kind: ChordKind) -> &'static [usize] {
    match kind {
        ChordKind::Major | ChordKind::Minor | ChordKind::Diminished | ChordKind::Augmented => {
            &[0, 2, 4]
        }
        ChordKind::DominantSeventh
        | ChordKind::MajorSeventh
        | ChordKind::MinorSeventh
        | ChordKind::DiminishedSeventh
        | ChordKind::HalfDiminishedSeventh
        | ChordKind::AugmentedSeventh
        | ChordKind::MinorMajorSeventh => &[0, 2, 4, 6],
        ChordKind::DominantSeventhFlatNinth
        | ChordKind::DominantNinth
        | ChordKind::MajorNinth
        | ChordKind::MinorNinth
        | ChordKind::DominantSeventhSharpNinth => &[0, 2, 4, 6, 1],
        ChordKind::SuspendedSecond => &[0, 1, 4],
        ChordKind::SuspendedFourth => &[0, 3, 4],
        ChordKind::MajorSixth | ChordKind::MinorSixth => &[0, 2, 4, 5],
        ChordKind::AddedNinth => &[0, 2, 4, 1],
        ChordKind::DominantSeventhSuspendedFourth => &[0, 3, 4, 6],
        ChordKind::PowerChord => &[0, 4],
        ChordKind::DominantThirteenth => &[0, 2, 6, 1, 5],
    }
}

fn letter_index(letter: NoteLetter) -> usize {
    match letter {
        NoteLetter::C => 0,
        NoteLetter::D => 1,
        NoteLetter::E => 2,
        NoteLetter::F => 3,
        NoteLetter::G => 4,
        NoteLetter::A => 5,
        NoteLetter::B => 6,
    }
}

/// `note` written with `letter`, when one sharp, one flat or none does it.
/// The octave is the one that makes the written pitch sound as `note`.
fn spelled_as(note: NoteNumber, letter: NoteLetter) -> Option<Pitch> {
    let note_value: i16 = i16::from(note.value());
    let pitch_class: i16 = note_value.rem_euclid(SEMITONES_PER_OCTAVE);
    // Semitones from the plain letter to the note, folded to −6 … +5.
    let difference: i16 =
        (pitch_class - letter.semitones_above_c() + 6).rem_euclid(SEMITONES_PER_OCTAVE) - 6;
    let accidental: Accidental = match difference {
        -1 => Accidental::Flat,
        0 => Accidental::Natural,
        1 => Accidental::Sharp,
        _ => return None,
    };
    pitch_sounding_as(note_value, NoteName::new(letter, accidental))
}

/// A chord note that would need a double sharp or flat with its own letter
/// (V1 has neither), spelled by its sound: flats in a flat key, else sharps.
fn by_pitch_class(note: NoteNumber, prefers_flats: bool) -> Pitch {
    const SHARP_NAMES: [(NoteLetter, Accidental); 12] = [
        (NoteLetter::C, Accidental::Natural),
        (NoteLetter::C, Accidental::Sharp),
        (NoteLetter::D, Accidental::Natural),
        (NoteLetter::D, Accidental::Sharp),
        (NoteLetter::E, Accidental::Natural),
        (NoteLetter::F, Accidental::Natural),
        (NoteLetter::F, Accidental::Sharp),
        (NoteLetter::G, Accidental::Natural),
        (NoteLetter::G, Accidental::Sharp),
        (NoteLetter::A, Accidental::Natural),
        (NoteLetter::A, Accidental::Sharp),
        (NoteLetter::B, Accidental::Natural),
    ];
    const FLAT_NAMES: [(NoteLetter, Accidental); 12] = [
        (NoteLetter::C, Accidental::Natural),
        (NoteLetter::D, Accidental::Flat),
        (NoteLetter::D, Accidental::Natural),
        (NoteLetter::E, Accidental::Flat),
        (NoteLetter::E, Accidental::Natural),
        (NoteLetter::F, Accidental::Natural),
        (NoteLetter::G, Accidental::Flat),
        (NoteLetter::G, Accidental::Natural),
        (NoteLetter::A, Accidental::Flat),
        (NoteLetter::A, Accidental::Natural),
        (NoteLetter::B, Accidental::Flat),
        (NoteLetter::B, Accidental::Natural),
    ];
    let note_value: i16 = i16::from(note.value());
    // 0 … 11 by rem_euclid, so the conversion cannot fail and the
    // fallback is never taken.
    let pitch_class: usize =
        usize::try_from(note_value.rem_euclid(SEMITONES_PER_OCTAVE)).unwrap_or(0);
    let (letter, accidental): (NoteLetter, Accidental) = if prefers_flats {
        FLAT_NAMES[pitch_class]
    } else {
        SHARP_NAMES[pitch_class]
    };
    let name: NoteName = NoteName::new(letter, accidental);
    // These names never cross an octave line (no C flat or B sharp), so the
    // octave is the note's own: MIDI 60 is C4.
    // MIDI 0 … 127 gives octave −1 … 9, which always fits an i8; the
    // fallback is never taken.
    let octave: i8 = i8::try_from(note_value / SEMITONES_PER_OCTAVE - 1).unwrap_or(i8::MAX);
    Pitch::new(name, octave)
}

/// The written pitch `name` in the octave that sounds as `note_value`:
/// MIDI = 12 × (octave + 1) + letter + accidental.
fn pitch_sounding_as(note_value: i16, name: NoteName) -> Option<Pitch> {
    let above_octave_c: i16 =
        name.letter().semitones_above_c() + name.accidental().semitone_offset();
    let octave: i16 = (note_value - above_octave_c) / SEMITONES_PER_OCTAVE - 1;
    i8::try_from(octave)
        .ok()
        .map(|octave: i8| Pitch::new(name, octave))
}

/// Flat keys spell a chord note that needs a double sharp or flat by its
/// sound with flats, the others with sharps. A major key is flat when its
/// tonic is written with a flat, and F major. A minor key takes its relative
/// major's signature (C minor has the three flats of E-flat major), so it is
/// flat when its tonic is written with a flat, and C, D, F and G minor.
fn key_prefers_flats(key: Key) -> bool {
    let natural_tonic: bool = key.tonic().accidental() == Accidental::Natural;
    let flat_natural_tonic: bool = match key.mode() {
        Mode::Major => key.tonic().letter() == NoteLetter::F,
        Mode::Minor => matches!(
            key.tonic().letter(),
            NoteLetter::C | NoteLetter::D | NoteLetter::F | NoteLetter::G
        ),
    };
    key.tonic().accidental() == Accidental::Flat || (natural_tonic && flat_natural_tonic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::key::Mode;
    use vocabulary::roman_chord::ScaleDegree;

    fn key(letter: NoteLetter, accidental: Accidental) -> Key {
        Key::new(NoteName::new(letter, accidental), Mode::Major)
    }

    fn names(pitches: &[Pitch]) -> Vec<String> {
        pitches.iter().map(ToString::to_string).collect()
    }

    /// The tonic chord of C major of `kind`, spelled from C4.
    fn on_c(kind: ChordKind) -> Vec<String> {
        let chord: RomanChord = RomanChord::new(Accidental::Natural, ScaleDegree::First, kind);
        names(&chord_pitches(chord, key(NoteLetter::C, Accidental::Natural), None, 4).unwrap())
    }

    #[test]
    fn the_added_chord_types_are_spelled_with_the_right_letters() {
        assert_eq!(on_c(ChordKind::SuspendedSecond), vec!["C4", "D4", "G4"]);
        assert_eq!(on_c(ChordKind::SuspendedFourth), vec!["C4", "F4", "G4"]);
        assert_eq!(on_c(ChordKind::MajorSixth), vec!["C4", "E4", "G4", "A4"]);
        assert_eq!(on_c(ChordKind::MinorSixth), vec!["C4", "Eb4", "G4", "A4"]);
        assert_eq!(
            on_c(ChordKind::DominantNinth),
            vec!["C4", "E4", "G4", "Bb4", "D5"]
        );
        assert_eq!(
            on_c(ChordKind::MajorNinth),
            vec!["C4", "E4", "G4", "B4", "D5"]
        );
        assert_eq!(
            on_c(ChordKind::MinorNinth),
            vec!["C4", "Eb4", "G4", "Bb4", "D5"]
        );
        assert_eq!(on_c(ChordKind::AddedNinth), vec!["C4", "E4", "G4", "D5"]);
        assert_eq!(
            on_c(ChordKind::DominantSeventhSuspendedFourth),
            vec!["C4", "F4", "G4", "Bb4"]
        );
        assert_eq!(
            on_c(ChordKind::DominantSeventhSharpNinth),
            vec!["C4", "E4", "G4", "Bb4", "D#5"]
        );
        assert_eq!(
            on_c(ChordKind::AugmentedSeventh),
            vec!["C4", "E4", "G#4", "Bb4"]
        );
        assert_eq!(
            on_c(ChordKind::MinorMajorSeventh),
            vec!["C4", "Eb4", "G4", "B4"]
        );
        assert_eq!(on_c(ChordKind::PowerChord), vec!["C4", "G4"]);
        assert_eq!(
            on_c(ChordKind::DominantThirteenth),
            vec!["C4", "E4", "Bb4", "D5", "A5"]
        );
    }

    #[test]
    fn ii7_in_c_major_is_d_f_a_c() {
        let chord: RomanChord = RomanChord::new(
            Accidental::Natural,
            ScaleDegree::Second,
            ChordKind::MinorSeventh,
        );
        let pitches: Vec<Pitch> =
            chord_pitches(chord, key(NoteLetter::C, Accidental::Natural), None, 4).unwrap();
        assert_eq!(names(&pitches), vec!["D4", "F4", "A4", "C5"]);
    }

    #[test]
    fn v7_in_e_flat_major_is_b_flat_d_f_a_flat() {
        let chord: RomanChord = RomanChord::new(
            Accidental::Natural,
            ScaleDegree::Fifth,
            ChordKind::DominantSeventh,
        );
        let pitches: Vec<Pitch> =
            chord_pitches(chord, key(NoteLetter::E, Accidental::Flat), None, 3).unwrap();
        assert_eq!(names(&pitches), vec!["Bb3", "D4", "F4", "Ab4"]);
    }

    #[test]
    fn vii_diminished_in_c_major_is_b_d_f_and_sounds_as_voiced() {
        let chord: RomanChord = RomanChord::new(
            Accidental::Natural,
            ScaleDegree::Seventh,
            ChordKind::Diminished,
        );
        let c_major: Key = key(NoteLetter::C, Accidental::Natural);
        let pitches: Vec<Pitch> = chord_pitches(chord, c_major, None, 4).unwrap();
        assert_eq!(names(&pitches), vec!["B4", "D5", "F5"]);
        let sounding: Vec<u8> = pitches
            .iter()
            .map(|pitch: &Pitch| pitch.note_number().unwrap().value())
            .collect();
        let voiced: Vec<u8> = plain_voicing(chord, c_major, None, 4)
            .unwrap()
            .iter()
            .map(|note: &NoteNumber| note.value())
            .collect();
        assert_eq!(sounding, voiced);
    }

    #[test]
    fn a_flat_seven_chord_takes_its_flat() {
        let chord: RomanChord =
            RomanChord::new(Accidental::Flat, ScaleDegree::Seventh, ChordKind::Major);
        let pitches: Vec<Pitch> =
            chord_pitches(chord, key(NoteLetter::C, Accidental::Natural), None, 3).unwrap();
        assert_eq!(names(&pitches), vec!["Bb3", "D4", "F4"]);
    }

    fn minor(letter: NoteLetter, accidental: Accidental) -> Key {
        Key::new(NoteName::new(letter, accidental), Mode::Minor)
    }

    #[test]
    fn a_minor_key_is_flat_or_sharp_by_its_relative_major() {
        let flat = |letter, accidental| key_prefers_flats(minor(letter, accidental));
        // Relative majors E-flat, B-flat, A-flat, F and the flat-tonic keys.
        assert!(flat(NoteLetter::C, Accidental::Natural));
        assert!(flat(NoteLetter::G, Accidental::Natural));
        assert!(flat(NoteLetter::F, Accidental::Natural));
        assert!(flat(NoteLetter::D, Accidental::Natural));
        assert!(flat(NoteLetter::B, Accidental::Flat));
        // Relative majors C (no accidentals), G, D and A: sharps.
        assert!(!flat(NoteLetter::A, Accidental::Natural));
        assert!(!flat(NoteLetter::E, Accidental::Natural));
        assert!(!flat(NoteLetter::B, Accidental::Natural));
        assert!(!flat(NoteLetter::F, Accidental::Sharp));
    }

    #[test]
    fn key_dependent_iii_in_c_minor_is_e_flat_g_b_flat() {
        let chord: RomanChord =
            RomanChord::new(Accidental::Natural, ScaleDegree::Third, ChordKind::Major);
        let pitches: Vec<Pitch> = chord_pitches(
            chord,
            minor(NoteLetter::C, Accidental::Natural),
            Some(NumeralStandard::KeyDependent),
            4,
        )
        .unwrap();
        assert_eq!(names(&pitches), vec!["Eb4", "G4", "Bb4"]);
    }

    #[test]
    fn the_leading_tone_chord_in_a_minor_is_g_sharp_b_d() {
        let chord: RomanChord = RomanChord::new(
            Accidental::Natural,
            ScaleDegree::Seventh,
            ChordKind::Diminished,
        );
        let pitches: Vec<Pitch> = chord_pitches(
            chord,
            minor(NoteLetter::A, Accidental::Natural),
            Some(NumeralStandard::KeyDependent),
            4,
        )
        .unwrap();
        assert_eq!(names(&pitches), vec!["G#4", "B4", "D5"]);
    }
}
