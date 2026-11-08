//! The written pitches of a chord named by a Roman numeral: the notes the
//! player sends (its plain voicing), each spelled with the letter its place
//! in the chord calls for. In C major `ii7` is D F A C, and in E♭ major
//! `V7` is B♭ D F A♭, never A♯.

use player::voicing::{VoicingError, plain_voicing};
use vocabulary::key::Key;
use vocabulary::midi_values::NoteNumber;
use vocabulary::pitch::{Accidental, NoteLetter, NoteName, Pitch};
use vocabulary::roman_chord::{ChordKind, RomanChord};

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
    home_octave: i8,
) -> Result<Vec<Pitch>, VoicingError> {
    let notes: Vec<NoteNumber> = plain_voicing(chord, key, home_octave)?;
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

/// Letters above the root for each note of the plain voicing, in its
/// order: third 2, fifth 4, seventh 6; the flat ninth is 1 letter above,
/// an octave up.
fn letter_steps(kind: ChordKind) -> &'static [usize] {
    match kind {
        ChordKind::Major | ChordKind::Minor | ChordKind::Diminished | ChordKind::Augmented => {
            &[0, 2, 4]
        }
        ChordKind::DominantSeventh
        | ChordKind::MajorSeventh
        | ChordKind::MinorSeventh
        | ChordKind::DiminishedSeventh
        | ChordKind::HalfDiminishedSeventh => &[0, 2, 4, 6],
        ChordKind::DominantSeventhFlatNinth => &[0, 2, 4, 6, 1],
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

/// F major and the keys whose tonic is written with a flat are flat keys.
fn key_prefers_flats(key: Key) -> bool {
    key.tonic().accidental() == Accidental::Flat
        || (key.tonic().letter() == NoteLetter::F
            && key.tonic().accidental() == Accidental::Natural)
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

    #[test]
    fn ii7_in_c_major_is_d_f_a_c() {
        let chord: RomanChord = RomanChord::new(
            Accidental::Natural,
            ScaleDegree::Second,
            ChordKind::MinorSeventh,
        );
        let pitches: Vec<Pitch> =
            chord_pitches(chord, key(NoteLetter::C, Accidental::Natural), 4).unwrap();
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
            chord_pitches(chord, key(NoteLetter::E, Accidental::Flat), 3).unwrap();
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
        let pitches: Vec<Pitch> = chord_pitches(chord, c_major, 4).unwrap();
        assert_eq!(names(&pitches), vec!["B4", "D5", "F5"]);
        let sounding: Vec<u8> = pitches
            .iter()
            .map(|pitch: &Pitch| pitch.note_number().unwrap().value())
            .collect();
        let voiced: Vec<u8> = plain_voicing(chord, c_major, 4)
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
            chord_pitches(chord, key(NoteLetter::C, Accidental::Natural), 3).unwrap();
        assert_eq!(names(&pitches), vec!["Bb3", "D4", "F4"]);
    }
}
