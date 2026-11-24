//! Moving a chord numeral by an interval, thinking in harmony rather than
//! notes: a diatonic step moves the root to the next degree of the key and
//! takes that degree's own chord (in C major, `I` up one step is `ii`,
//! `IM7` is `ii7`); a chromatic step moves the root a semitone and keeps the
//! chord's kind (`V7` down one semitone is `bV7`).

use vocabulary::key::{Key, Mode};
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::pitch::Accidental;
use vocabulary::roman_chord::{ChordKind, RomanChord, ScaleDegree};

use crate::key_signature::SEMITONES_PER_OCTAVE;

const DEGREES: [ScaleDegree; 7] = [
    ScaleDegree::First,
    ScaleDegree::Second,
    ScaleDegree::Third,
    ScaleDegree::Fourth,
    ScaleDegree::Fifth,
    ScaleDegree::Sixth,
    ScaleDegree::Seventh,
];

/// Semitones above the tonic of each degree of the major scale.
const MAJOR_SCALE_SEMITONES: [i16; 7] = [0, 2, 4, 5, 7, 9, 11];
/// And of the natural minor scale.
const NATURAL_MINOR_SCALE_SEMITONES: [i16; 7] = [0, 2, 3, 5, 7, 8, 10];

/// The triads and seventh chords built on each degree of the major scale.
const MAJOR_TRIADS: [ChordKind; 7] = [
    ChordKind::Major,
    ChordKind::Minor,
    ChordKind::Minor,
    ChordKind::Major,
    ChordKind::Major,
    ChordKind::Minor,
    ChordKind::Diminished,
];
const MAJOR_SEVENTHS: [ChordKind; 7] = [
    ChordKind::MajorSeventh,
    ChordKind::MinorSeventh,
    ChordKind::MinorSeventh,
    ChordKind::MajorSeventh,
    ChordKind::DominantSeventh,
    ChordKind::MinorSeventh,
    ChordKind::HalfDiminishedSeventh,
];
/// And of the natural minor scale.
const MINOR_TRIADS: [ChordKind; 7] = [
    ChordKind::Minor,
    ChordKind::Diminished,
    ChordKind::Major,
    ChordKind::Minor,
    ChordKind::Minor,
    ChordKind::Major,
    ChordKind::Major,
];
const MINOR_SEVENTHS: [ChordKind; 7] = [
    ChordKind::MinorSeventh,
    ChordKind::HalfDiminishedSeventh,
    ChordKind::MajorSeventh,
    ChordKind::MinorSeventh,
    ChordKind::MinorSeventh,
    ChordKind::MajorSeventh,
    ChordKind::DominantSeventh,
];

/// How many notes a chord stacks, which a diatonic step keeps: a triad
/// stays a triad, a seventh chord a seventh chord, a ninth a ninth. Colour
/// chords (sus, sixth, add9, power) keep their own kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChordSize {
    Triad,
    Seventh,
    Ninth,
    Colour,
}

const fn size_of(kind: ChordKind) -> ChordSize {
    match kind {
        ChordKind::Major | ChordKind::Minor | ChordKind::Diminished | ChordKind::Augmented => {
            ChordSize::Triad
        }
        ChordKind::DominantSeventh
        | ChordKind::MajorSeventh
        | ChordKind::MinorSeventh
        | ChordKind::DiminishedSeventh
        | ChordKind::HalfDiminishedSeventh
        | ChordKind::AugmentedSeventh
        | ChordKind::MinorMajorSeventh => ChordSize::Seventh,
        ChordKind::DominantSeventhFlatNinth
        | ChordKind::DominantNinth
        | ChordKind::MajorNinth
        | ChordKind::MinorNinth
        | ChordKind::DominantSeventhSharpNinth
        | ChordKind::DominantThirteenth => ChordSize::Ninth,
        ChordKind::SuspendedSecond
        | ChordKind::SuspendedFourth
        | ChordKind::MajorSixth
        | ChordKind::MinorSixth
        | ChordKind::AddedNinth
        | ChordKind::DominantSeventhSuspendedFourth
        | ChordKind::PowerChord => ChordSize::Colour,
    }
}

/// The ninth chord of a seventh chord's quality; the half-diminished
/// seventh has no ninth chord in the notation and stays as it is.
const fn ninth_of(seventh: ChordKind) -> ChordKind {
    match seventh {
        ChordKind::MajorSeventh => ChordKind::MajorNinth,
        ChordKind::MinorSeventh => ChordKind::MinorNinth,
        ChordKind::DominantSeventh => ChordKind::DominantNinth,
        other => other,
    }
}

fn degree_index(degree: ScaleDegree) -> usize {
    usize::from(degree.number()) - 1
}

/// Whether the song counts minor-key numerals in the major scale, writing
/// a flat for the lowered steps (`bIII`, `bVI`, `bVII`).
fn counts_in_major_scale(mode: Mode, numeral_standard: Option<NumeralStandard>) -> bool {
    match mode {
        Mode::Major => true,
        Mode::Minor => numeral_standard == Some(NumeralStandard::MajorReferential),
    }
}

/// Semitones above the tonic of each degree, as the song's numerals count.
fn numeral_scale(mode: Mode, numeral_standard: Option<NumeralStandard>) -> [i16; 7] {
    if counts_in_major_scale(mode, numeral_standard) {
        MAJOR_SCALE_SEMITONES
    } else {
        NATURAL_MINOR_SCALE_SEMITONES
    }
}

/// The chord `steps` degrees above (below when negative) `chord`, with the
/// new degree's own chord of the same size, in the key's scale (the natural
/// minor in a minor key).
#[must_use]
pub fn diatonic_chord_step(
    chord: RomanChord,
    key: Key,
    numeral_standard: Option<NumeralStandard>,
    steps: i16,
) -> RomanChord {
    let from: i16 = i16::from(chord.degree().number()) - 1;
    let to_index: usize = usize::try_from((from + steps).rem_euclid(7)).unwrap_or(0);
    let degree: ScaleDegree = DEGREES[to_index];
    let (triads, sevenths): ([ChordKind; 7], [ChordKind; 7]) = match key.mode() {
        Mode::Major => (MAJOR_TRIADS, MAJOR_SEVENTHS),
        Mode::Minor => (MINOR_TRIADS, MINOR_SEVENTHS),
    };
    let kind: ChordKind = match size_of(chord.kind()) {
        ChordSize::Triad => triads[to_index],
        ChordSize::Seventh => sevenths[to_index],
        ChordSize::Ninth => ninth_of(sevenths[to_index]),
        ChordSize::Colour => chord.kind(),
    };
    // A minor key whose numerals count in the major scale writes its
    // lowered third, sixth and seventh degrees with a flat.
    let lowered: bool = key.mode() == Mode::Minor
        && counts_in_major_scale(key.mode(), numeral_standard)
        && MAJOR_SCALE_SEMITONES[to_index] != NATURAL_MINOR_SCALE_SEMITONES[to_index];
    let accidental: Accidental = if lowered {
        Accidental::Flat
    } else {
        Accidental::Natural
    };
    RomanChord::new(accidental, degree, kind)
}

/// The chord with its root `semitones` higher (lower when negative), the
/// same kind of chord. The root is written on the degree that names it
/// plainly when there is one, otherwise with a sharp going up and a flat
/// going down (`I` up a semitone is `#I`, `II` down a semitone is `bII`).
#[must_use]
pub fn chromatic_chord_step(
    chord: RomanChord,
    key: Key,
    numeral_standard: Option<NumeralStandard>,
    semitones: i16,
) -> RomanChord {
    let scale: [i16; 7] = numeral_scale(key.mode(), numeral_standard);
    let root: i16 = scale[degree_index(chord.degree())] + chord.accidental().semitone_offset();
    let target: i16 = (root + semitones).rem_euclid(SEMITONES_PER_OCTAVE);
    let plain: Option<usize> = scale.iter().position(|step: &i16| *step == target);
    let (index, accidental): (usize, Accidental) = match plain {
        Some(index) => (index, Accidental::Natural),
        None if semitones >= 0 => (
            scale
                .iter()
                .position(|step: &i16| (step + 1).rem_euclid(SEMITONES_PER_OCTAVE) == target)
                .unwrap_or(0),
            Accidental::Sharp,
        ),
        None => (
            scale
                .iter()
                .position(|step: &i16| (step - 1).rem_euclid(SEMITONES_PER_OCTAVE) == target)
                .unwrap_or(0),
            Accidental::Flat,
        ),
    };
    RomanChord::new(accidental, DEGREES[index], chord.kind())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::pitch::{NoteLetter, NoteName};

    fn c_major() -> Key {
        Key::new(
            NoteName::new(NoteLetter::C, Accidental::Natural),
            Mode::Major,
        )
    }

    fn numeral(degree: ScaleDegree, kind: ChordKind) -> RomanChord {
        RomanChord::new(Accidental::Natural, degree, kind)
    }

    #[test]
    fn a_diatonic_step_takes_the_next_degrees_own_chord() {
        let i: RomanChord = numeral(ScaleDegree::First, ChordKind::Major);
        assert_eq!(diatonic_chord_step(i, c_major(), None, 1).to_string(), "ii");
        assert_eq!(
            diatonic_chord_step(i, c_major(), None, -1).to_string(),
            "vii°"
        );
        let imaj7: RomanChord = numeral(ScaleDegree::First, ChordKind::MajorSeventh);
        assert_eq!(
            diatonic_chord_step(imaj7, c_major(), None, 4).to_string(),
            "V7"
        );
        let a_minor: Key = Key::new(
            NoteName::new(NoteLetter::A, Accidental::Natural),
            Mode::Minor,
        );
        let i_minor: RomanChord = numeral(ScaleDegree::First, ChordKind::Minor);
        assert_eq!(
            diatonic_chord_step(i_minor, a_minor, Some(NumeralStandard::KeyDependent), 2)
                .to_string(),
            "III"
        );
        assert_eq!(
            diatonic_chord_step(i_minor, a_minor, Some(NumeralStandard::MajorReferential), 2)
                .to_string(),
            "bIII"
        );
    }

    #[test]
    fn a_chromatic_step_keeps_the_kind() {
        let v7: RomanChord = numeral(ScaleDegree::Fifth, ChordKind::DominantSeventh);
        assert_eq!(
            chromatic_chord_step(v7, c_major(), None, -1).to_string(),
            "bV7"
        );
        assert_eq!(
            chromatic_chord_step(v7, c_major(), None, 2).to_string(),
            "VI7"
        );
        let i: RomanChord = numeral(ScaleDegree::First, ChordKind::Major);
        assert_eq!(
            chromatic_chord_step(i, c_major(), None, 1).to_string(),
            "#I"
        );
        assert_eq!(
            chromatic_chord_step(i, c_major(), None, -2).to_string(),
            "bVII"
        );
    }
}
