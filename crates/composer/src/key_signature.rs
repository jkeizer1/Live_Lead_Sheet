//! Key signatures, and the key a transposition reaches. A key signature is
//! counted in fifths around the circle: G major has one sharp (+1), F major
//! one flat (−1), C major none (0). A minor key has the signature of its
//! relative major, three fifths below its tonic (A minor 0, E minor +1).

use vocabulary::key::{Key, Mode};
use vocabulary::pitch::{Accidental, NoteLetter, NoteName};

pub const SEMITONES_PER_OCTAVE: i16 = 12;

/// The letters in staff order, C first.
pub const LETTERS: [NoteLetter; 7] = [
    NoteLetter::C,
    NoteLetter::D,
    NoteLetter::E,
    NoteLetter::F,
    NoteLetter::G,
    NoteLetter::A,
    NoteLetter::B,
];

/// The order sharps are added to a signature, F♯ first.
pub const SHARP_ORDER: [NoteLetter; 7] = [
    NoteLetter::F,
    NoteLetter::C,
    NoteLetter::G,
    NoteLetter::D,
    NoteLetter::A,
    NoteLetter::E,
    NoteLetter::B,
];

/// The order flats are added to a signature, B♭ first.
pub const FLAT_ORDER: [NoteLetter; 7] = [
    NoteLetter::B,
    NoteLetter::E,
    NoteLetter::A,
    NoteLetter::D,
    NoteLetter::G,
    NoteLetter::C,
    NoteLetter::F,
];

/// Fifths a minor key's signature lies below its tonic's major signature:
/// A minor shares C major's, and A major has 3 sharps.
const MINOR_FIFTHS_BELOW_MAJOR: i8 = 3;

/// A letter's place on the circle of fifths for its natural note: F −1,
/// C 0, G 1, D 2, A 3, E 4, B 5.
const fn letter_fifths(letter: NoteLetter) -> i8 {
    match letter {
        NoteLetter::F => -1,
        NoteLetter::C => 0,
        NoteLetter::G => 1,
        NoteLetter::D => 2,
        NoteLetter::A => 3,
        NoteLetter::E => 4,
        NoteLetter::B => 5,
    }
}

/// Fifths an accidental moves a note on the circle: a sharp is 7 fifths
/// up (F to F♯), a flat 7 down.
const FIFTHS_PER_ACCIDENTAL: i8 = 7;

/// A key's signature: sharps when positive, flats when negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySignature {
    fifths: i8,
}

impl KeySignature {
    #[must_use]
    pub fn of(key: Key) -> KeySignature {
        let accidental_fifths: i8 = match key.tonic().accidental() {
            Accidental::Flat => -FIFTHS_PER_ACCIDENTAL,
            Accidental::Natural => 0,
            Accidental::Sharp => FIFTHS_PER_ACCIDENTAL,
        };
        let tonic_fifths: i8 = letter_fifths(key.tonic().letter()) + accidental_fifths;
        let fifths: i8 = match key.mode() {
            Mode::Major => tonic_fifths,
            Mode::Minor => tonic_fifths - MINOR_FIFTHS_BELOW_MAJOR,
        };
        KeySignature { fifths }
    }

    /// Sharps (positive) or flats (negative).
    #[must_use]
    pub const fn fifths(self) -> i8 {
        self.fifths
    }

    /// The accidental the signature gives a letter. A signature beyond
    /// seven sharps or flats would need double sharps, which the notation
    /// does not write; such keys are never reached by transposing here.
    #[must_use]
    pub fn accidental_for(self, letter: NoteLetter) -> Accidental {
        let count: usize = usize::from(self.fifths.unsigned_abs()).min(SHARP_ORDER.len());
        if self.fifths > 0 && SHARP_ORDER[..count].contains(&letter) {
            Accidental::Sharp
        } else if self.fifths < 0 && FLAT_ORDER[..count].contains(&letter) {
            Accidental::Flat
        } else {
            Accidental::Natural
        }
    }

    /// The letters that carry the signature's sharps or flats, in the
    /// order they are drawn, and which of the two they are.
    #[must_use]
    pub fn drawn_accidentals(self) -> (Accidental, &'static [NoteLetter]) {
        let count: usize = usize::from(self.fifths.unsigned_abs()).min(SHARP_ORDER.len());
        if self.fifths >= 0 {
            (Accidental::Sharp, &SHARP_ORDER[..count])
        } else {
            (Accidental::Flat, &FLAT_ORDER[..count])
        }
    }

    /// Whether notes outside the key are better spelled with flats.
    #[must_use]
    pub const fn prefers_flats(self) -> bool {
        self.fifths < 0
    }
}

/// Semitones above C of a note name, 0 to 11.
#[must_use]
pub fn pitch_class(name: NoteName) -> i16 {
    (name.letter().semitones_above_c() + name.accidental().semitone_offset())
        .rem_euclid(SEMITONES_PER_OCTAVE)
}

/// The usual spelling of each tonic, by semitones above C, for major and
/// for minor keys: the spelling with fewer accidentals in its signature
/// (D♭ major, 5 flats, rather than C♯ major, 7 sharps; F♯ major and minor
/// are both written with sharps, as most lead sheets do).
const MAJOR_TONICS: [(NoteLetter, Accidental); 12] = [
    (NoteLetter::C, Accidental::Natural),
    (NoteLetter::D, Accidental::Flat),
    (NoteLetter::D, Accidental::Natural),
    (NoteLetter::E, Accidental::Flat),
    (NoteLetter::E, Accidental::Natural),
    (NoteLetter::F, Accidental::Natural),
    (NoteLetter::F, Accidental::Sharp),
    (NoteLetter::G, Accidental::Natural),
    (NoteLetter::A, Accidental::Flat),
    (NoteLetter::A, Accidental::Natural),
    (NoteLetter::B, Accidental::Flat),
    (NoteLetter::B, Accidental::Natural),
];

const MINOR_TONICS: [(NoteLetter, Accidental); 12] = [
    (NoteLetter::C, Accidental::Natural),
    (NoteLetter::C, Accidental::Sharp),
    (NoteLetter::D, Accidental::Natural),
    (NoteLetter::E, Accidental::Flat),
    (NoteLetter::E, Accidental::Natural),
    (NoteLetter::F, Accidental::Natural),
    (NoteLetter::F, Accidental::Sharp),
    (NoteLetter::G, Accidental::Natural),
    (NoteLetter::G, Accidental::Sharp),
    (NoteLetter::A, Accidental::Natural),
    (NoteLetter::B, Accidental::Flat),
    (NoteLetter::B, Accidental::Natural),
];

/// Semitones a fifth spans.
pub const SEMITONES_PER_FIFTH: i16 = 7;

/// The key `semitones` above `key` (below when negative), in the same
/// mode, spelled as lead sheets usually spell it.
#[must_use]
pub fn transposed_key(key: Key, semitones: i16) -> Key {
    let tonic_class: i16 = (pitch_class(key.tonic()) + semitones).rem_euclid(SEMITONES_PER_OCTAVE);
    // rem_euclid by 12 gives 0 to 11, which fits an index.
    let index: usize = usize::try_from(tonic_class).unwrap_or(0);
    let (letter, accidental): (NoteLetter, Accidental) = match key.mode() {
        Mode::Major => MAJOR_TONICS[index],
        Mode::Minor => MINOR_TONICS[index],
    };
    Key::new(NoteName::new(letter, accidental), key.mode())
}

/// The key `fifths` steps round the circle of fifths from `key`: one step
/// up is a fifth above (C to G), one down a fifth below (C to F).
#[must_use]
pub fn key_by_fifths(key: Key, fifths: i16) -> Key {
    transposed_key(key, fifths * SEMITONES_PER_FIFTH)
}

/// Semitones from one key's tonic up to another's, −5 to +6: the smallest
/// move, so C to G is −5 (down a fourth) and C to F♯ is +6.
#[must_use]
pub fn semitones_between(from: Key, to: Key) -> i16 {
    let up: i16 =
        (pitch_class(to.tonic()) - pitch_class(from.tonic())).rem_euclid(SEMITONES_PER_OCTAVE);
    if up > SEMITONES_PER_OCTAVE / 2 {
        up - SEMITONES_PER_OCTAVE
    } else {
        up
    }
}

/// The key as the song file writes it after `key`: `C major`, `Bb minor`.
#[must_use]
pub fn key_text(key: Key) -> String {
    key.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(letter: NoteLetter, accidental: Accidental, mode: Mode) -> Key {
        Key::new(NoteName::new(letter, accidental), mode)
    }

    #[test]
    fn signatures_count_sharps_and_flats() {
        assert_eq!(
            KeySignature::of(key(NoteLetter::C, Accidental::Natural, Mode::Major)).fifths(),
            0
        );
        assert_eq!(
            KeySignature::of(key(NoteLetter::D, Accidental::Natural, Mode::Major)).fifths(),
            2
        );
        assert_eq!(
            KeySignature::of(key(NoteLetter::E, Accidental::Flat, Mode::Major)).fifths(),
            -3
        );
        assert_eq!(
            KeySignature::of(key(NoteLetter::A, Accidental::Natural, Mode::Minor)).fifths(),
            0
        );
        assert_eq!(
            KeySignature::of(key(NoteLetter::C, Accidental::Natural, Mode::Minor)).fifths(),
            -3
        );
        let d_major: KeySignature =
            KeySignature::of(key(NoteLetter::D, Accidental::Natural, Mode::Major));
        assert_eq!(d_major.accidental_for(NoteLetter::F), Accidental::Sharp);
        assert_eq!(d_major.accidental_for(NoteLetter::C), Accidental::Sharp);
        assert_eq!(d_major.accidental_for(NoteLetter::G), Accidental::Natural);
    }

    #[test]
    fn transposing_spells_keys_as_lead_sheets_do() {
        let c_major: Key = key(NoteLetter::C, Accidental::Natural, Mode::Major);
        assert_eq!(transposed_key(c_major, 1).to_string(), "Db major");
        assert_eq!(transposed_key(c_major, -2).to_string(), "Bb major");
        assert_eq!(key_by_fifths(c_major, 1).to_string(), "G major");
        assert_eq!(key_by_fifths(c_major, -1).to_string(), "F major");
        let a_minor: Key = key(NoteLetter::A, Accidental::Natural, Mode::Minor);
        assert_eq!(transposed_key(a_minor, -1).to_string(), "G# minor");
        assert_eq!(semitones_between(c_major, key_by_fifths(c_major, 1)), -5);
    }
}
