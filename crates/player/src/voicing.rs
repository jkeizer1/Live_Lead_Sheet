//! The plain voicing of a chord (EFS-5.5): close position, root position,
//! the root at or above the track's home octave.
//!
//! This is score logic, so it is the player's and not the notation's: the
//! text says `ii7` and a key, and this says which notes sound.

use std::fmt;

use vocabulary::key::{Key, Mode};
use vocabulary::midi_values::NoteNumber;
use vocabulary::roman_chord::{ChordKind, RomanChord, ScaleDegree};

const SEMITONES_PER_OCTAVE: i16 = 12;

/// The notes of a chord, lowest first.
///
/// `home_octave` is the octave whose C is the lowest the root may be:
/// with home octave 4, a C chord has its root on middle C and a G chord
/// has its root on the G above it.
///
/// # Errors
///
/// When a note would lie outside the MIDI range 0 to 127.
pub fn plain_voicing(
    chord: RomanChord,
    key: Key,
    home_octave: i8,
) -> Result<Vec<NoteNumber>, VoicingError> {
    let Mode::Major = key.mode();
    let tonic_semitones: i16 =
        key.tonic().letter().semitones_above_c() + key.tonic().accidental().semitone_offset();
    let root_semitones_above_c: i16 = (tonic_semitones
        + semitones_above_tonic(chord.degree())
        + chord.accidental().semitone_offset())
    .rem_euclid(SEMITONES_PER_OCTAVE);
    // C of the home octave: MIDI note 60 is C4, so C of octave n is
    // 12 notes/octave × (n + 1) octaves.
    let home_c: i16 = SEMITONES_PER_OCTAVE * (i16::from(home_octave) + 1);
    let root: i16 = home_c + root_semitones_above_c;
    intervals(chord.kind())
        .iter()
        .map(|interval: &i16| {
            let value: i16 = root + interval;
            u8::try_from(value)
                .ok()
                .and_then(|byte: u8| NoteNumber::new(byte).ok())
                .ok_or(VoicingError::OutOfRange { chord, value })
        })
        .collect()
}

/// The major scale: I 0, II 2, III 4, IV 5, V 7, VI 9, VII 11 semitones
/// above the tonic.
fn semitones_above_tonic(degree: ScaleDegree) -> i16 {
    match degree {
        ScaleDegree::First => 0,
        ScaleDegree::Second => 2,
        ScaleDegree::Third => 4,
        ScaleDegree::Fourth => 5,
        ScaleDegree::Fifth => 7,
        ScaleDegree::Sixth => 9,
        ScaleDegree::Seventh => 11,
    }
}

/// Semitones above the root, in close position.
fn intervals(kind: ChordKind) -> &'static [i16] {
    match kind {
        ChordKind::Major => &[0, 4, 7],
        ChordKind::Minor => &[0, 3, 7],
        ChordKind::Diminished => &[0, 3, 6],
        ChordKind::Augmented => &[0, 4, 8],
        ChordKind::DominantSeventh => &[0, 4, 7, 10],
        ChordKind::MajorSeventh => &[0, 4, 7, 11],
        ChordKind::MinorSeventh => &[0, 3, 7, 10],
        ChordKind::DiminishedSeventh => &[0, 3, 6, 9],
        ChordKind::HalfDiminishedSeventh => &[0, 3, 6, 10],
        ChordKind::DominantSeventhFlatNinth => &[0, 4, 7, 10, 13],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicingError {
    OutOfRange { chord: RomanChord, value: i16 },
}

impl fmt::Display for VoicingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoicingError::OutOfRange { chord, value } => write!(
                f,
                "the chord {chord} would need MIDI note {value}, outside 0 to 127. Lower or \
                 raise the track's home_octave"
            ),
        }
    }
}

impl std::error::Error for VoicingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::pitch::{Accidental, NoteLetter, NoteName};

    fn key(letter: NoteLetter, accidental: Accidental) -> Key {
        Key::new(NoteName::new(letter, accidental), Mode::Major)
    }

    fn chord(accidental: Accidental, degree: ScaleDegree, kind: ChordKind) -> RomanChord {
        RomanChord::new(accidental, degree, kind)
    }

    fn values(notes: &[NoteNumber]) -> Vec<u8> {
        notes.iter().map(|note: &NoteNumber| note.value()).collect()
    }

    #[test]
    fn i_in_c_major_is_middle_c_e_g() {
        let notes = plain_voicing(
            chord(Accidental::Natural, ScaleDegree::First, ChordKind::Major),
            key(NoteLetter::C, Accidental::Natural),
            4,
        )
        .unwrap();
        assert_eq!(values(&notes), vec![60, 64, 67]);
    }

    #[test]
    fn ii7_in_c_major_is_d_f_a_c() {
        let notes = plain_voicing(
            chord(
                Accidental::Natural,
                ScaleDegree::Second,
                ChordKind::MinorSeventh,
            ),
            key(NoteLetter::C, Accidental::Natural),
            4,
        )
        .unwrap();
        assert_eq!(values(&notes), vec![62, 65, 69, 72]);
    }

    #[test]
    fn v7_in_c_major_keeps_its_root_in_the_home_octave() {
        let notes = plain_voicing(
            chord(
                Accidental::Natural,
                ScaleDegree::Fifth,
                ChordKind::DominantSeventh,
            ),
            key(NoteLetter::C, Accidental::Natural),
            4,
        )
        .unwrap();
        assert_eq!(values(&notes), vec![67, 71, 74, 77]);
    }

    #[test]
    fn a_flat_seventh_chord_in_f_major_uses_the_key_and_the_accidental() {
        // bVII in F major is Eb.
        let notes = plain_voicing(
            chord(Accidental::Flat, ScaleDegree::Seventh, ChordKind::Major),
            key(NoteLetter::F, Accidental::Natural),
            4,
        )
        .unwrap();
        assert_eq!(values(&notes), vec![63, 67, 70]);
    }

    #[test]
    fn the_home_octave_moves_the_chord() {
        let notes = plain_voicing(
            chord(Accidental::Natural, ScaleDegree::First, ChordKind::Major),
            key(NoteLetter::C, Accidental::Natural),
            3,
        )
        .unwrap();
        assert_eq!(values(&notes), vec![48, 52, 55]);
    }

    #[test]
    fn a_chord_outside_the_midi_range_is_refused() {
        let error = plain_voicing(
            chord(
                Accidental::Natural,
                ScaleDegree::Seventh,
                ChordKind::DominantSeventhFlatNinth,
            ),
            key(NoteLetter::B, Accidental::Natural),
            9,
        )
        .unwrap_err();
        assert!(error.to_string().contains("outside 0 to 127"));
    }
}
