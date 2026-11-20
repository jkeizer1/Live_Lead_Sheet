//! The plain voicing of a chord (EFS-5.5): close position, root position,
//! the root at or above the track's home octave.
//!
//! This is score logic, so it is the player's and not the notation's: the
//! text says `ii7` and a key, and this says which notes sound.

use std::fmt;

use vocabulary::key::{Key, Mode};
use vocabulary::midi_values::NoteNumber;
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::roman_chord::{ChordKind, Inversion, RomanChord, ScaleDegree};

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
    numeral_standard: Option<NumeralStandard>,
    home_octave: i8,
) -> Result<Vec<NoteNumber>, VoicingError> {
    let step_semitones: i16 = step_semitones_above_tonic(chord, key.mode(), numeral_standard)?;
    let tonic_semitones: i16 =
        key.tonic().letter().semitones_above_c() + key.tonic().accidental().semitone_offset();
    let root_semitones_above_c: i16 =
        (tonic_semitones + step_semitones + chord.accidental().semitone_offset())
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

/// The notes of a chord in an inversion, lowest first: the notes that were
/// below the new lowest note move up one octave (Song Notation, section 4).
/// `[60, 64, 67]` in the first inversion is `[64, 67, 72]`.
///
/// # Errors
///
/// When a note would lie outside the MIDI range 0 to 127.
pub fn inverted_voicing(
    chord: RomanChord,
    key: Key,
    numeral_standard: Option<NumeralStandard>,
    home_octave: i8,
    inversion: Inversion,
) -> Result<Vec<NoteNumber>, VoicingError> {
    let notes: Vec<NoteNumber> = plain_voicing(chord, key, numeral_standard, home_octave)?;
    let moved: usize = usize::from(inversion.number());
    let (below, rest): (&[NoteNumber], &[NoteNumber]) = notes.split_at(moved);
    let mut inverted: Vec<NoteNumber> = rest.to_vec();
    for note in below {
        let value: i16 = i16::from(note.value()) + SEMITONES_PER_OCTAVE;
        inverted.push(
            u8::try_from(value)
                .ok()
                .and_then(|byte: u8| NoteNumber::new(byte).ok())
                .ok_or(VoicingError::OutOfRange { chord, value })?,
        );
    }
    Ok(inverted)
}

/// The semitones from the tonic to the step a numeral names, before its
/// flat or sharp. A major key counts in the major scale. A minor key counts
/// as the song's `chord_numeral_standard` says: `major_referential` in the
/// major scale on the tonic (`bIII` is E♭ in C minor), `key_dependent` in
/// the natural minor scale (`III` is E♭), except that a diminished chord on
/// step 7 is the leading-tone chord and takes the raised 7th (`vii°` is B in
/// C minor, as in classical harmony).
fn step_semitones_above_tonic(
    chord: RomanChord,
    mode: Mode,
    numeral_standard: Option<NumeralStandard>,
) -> Result<i16, VoicingError> {
    if mode == Mode::Major {
        return Ok(semitones_above_tonic(chord.degree()));
    }
    match numeral_standard {
        None => Err(VoicingError::MinorKeyNeedsNumeralStandard { chord }),
        Some(NumeralStandard::MajorReferential) => Ok(semitones_above_tonic(chord.degree())),
        Some(NumeralStandard::KeyDependent) => {
            let leading_tone_chord: bool = chord.degree() == ScaleDegree::Seventh
                && matches!(
                    chord.kind(),
                    ChordKind::Diminished
                        | ChordKind::DiminishedSeventh
                        | ChordKind::HalfDiminishedSeventh
                );
            if leading_tone_chord {
                return Ok(semitones_above_tonic(chord.degree()));
            }
            Ok(natural_minor_semitones_above_tonic(chord.degree()))
        }
    }
}

/// The natural minor scale: I 0, II 2, III 3, IV 5, V 7, VI 8, VII 10
/// semitones above the tonic.
fn natural_minor_semitones_above_tonic(degree: ScaleDegree) -> i16 {
    match degree {
        ScaleDegree::First => 0,
        ScaleDegree::Second => 2,
        ScaleDegree::Third => 3,
        ScaleDegree::Fourth => 5,
        ScaleDegree::Fifth => 7,
        ScaleDegree::Sixth => 8,
        ScaleDegree::Seventh => 10,
    }
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
        ChordKind::SuspendedSecond => &[0, 2, 7],
        ChordKind::SuspendedFourth => &[0, 5, 7],
        ChordKind::MajorSixth => &[0, 4, 7, 9],
        ChordKind::MinorSixth => &[0, 3, 7, 9],
        ChordKind::DominantNinth => &[0, 4, 7, 10, 14],
        ChordKind::MajorNinth => &[0, 4, 7, 11, 14],
        ChordKind::MinorNinth => &[0, 3, 7, 10, 14],
        ChordKind::AddedNinth => &[0, 4, 7, 14],
        ChordKind::DominantSeventhSuspendedFourth => &[0, 5, 7, 10],
        ChordKind::DominantSeventhSharpNinth => &[0, 4, 7, 10, 15],
        ChordKind::AugmentedSeventh => &[0, 4, 8, 10],
        ChordKind::MinorMajorSeventh => &[0, 3, 7, 11],
        ChordKind::PowerChord => &[0, 7],
        ChordKind::DominantThirteenth => &[0, 4, 10, 14, 21],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicingError {
    OutOfRange { chord: RomanChord, value: i16 },
    MinorKeyNeedsNumeralStandard { chord: RomanChord },
}

impl fmt::Display for VoicingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoicingError::OutOfRange { chord, value } => write!(
                f,
                "the chord {chord} would need MIDI note {value}, outside 0 to 127. Lower or \
                 raise the track's home_octave"
            ),
            VoicingError::MinorKeyNeedsNumeralStandard { chord } => write!(
                f,
                "the chord {chord} is in a minor key and the song does not say how its numerals \
                 are counted. Add chord_numeral_standard key_dependent or \
                 chord_numeral_standard major_referential on the line after song"
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
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
            None,
            9,
        )
        .unwrap_err();
        assert!(error.to_string().contains("outside 0 to 127"));
    }
}
