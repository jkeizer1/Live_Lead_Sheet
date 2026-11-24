//! A chord numeral named as a lead sheet names it, in a key (EFS-1.5a):
//! `ii7` in C major is Dm7, `V7` in E♭ major is B♭7, `I` in its first
//! inversion is C/E. The root's letter comes from the same spelling the
//! staff uses (`note_spelling`), so the symbol and the noteheads agree.

use note_spelling::chord_pitches::{chord_pitches, inverted_chord_pitches};
use player::voicing::VoicingError;
use vocabulary::key::Key;
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::pitch::{Accidental, NoteName, Pitch};
use vocabulary::roman_chord::{ChordKind, Inversion, RomanChord};

/// Any octave does for naming: the letters are the same in every octave.
const NAMING_OCTAVE: i8 = 4;

/// A note name with the music's own sharp and flat signs: F♯, B♭.
#[must_use]
pub fn note_name_text(name: NoteName) -> String {
    let sign: &str = match name.accidental() {
        Accidental::Flat => "♭",
        Accidental::Natural => "",
        Accidental::Sharp => "♯",
    };
    format!("{}{sign}", name.letter().letter())
}

/// What follows the root in a chord symbol (EFS-1.5a).
#[must_use]
pub const fn symbol_suffix(kind: ChordKind) -> &'static str {
    match kind {
        ChordKind::Major => "",
        ChordKind::Minor => "m",
        ChordKind::Diminished => "°",
        ChordKind::Augmented => "+",
        ChordKind::DominantSeventh => "7",
        ChordKind::MajorSeventh => "M7",
        ChordKind::MinorSeventh => "m7",
        ChordKind::DiminishedSeventh => "°7",
        ChordKind::HalfDiminishedSeventh => "ø7",
        ChordKind::DominantSeventhFlatNinth => "7♭9",
        ChordKind::SuspendedSecond => "sus2",
        ChordKind::SuspendedFourth => "sus4",
        ChordKind::MajorSixth => "6",
        ChordKind::MinorSixth => "m6",
        ChordKind::DominantNinth => "9",
        ChordKind::MajorNinth => "M9",
        ChordKind::MinorNinth => "m9",
        ChordKind::AddedNinth => "add9",
        ChordKind::DominantSeventhSuspendedFourth => "7sus4",
        ChordKind::DominantSeventhSharpNinth => "7♯9",
        ChordKind::AugmentedSeventh => "+7",
        ChordKind::MinorMajorSeventh => "mM7",
        ChordKind::PowerChord => "5",
        ChordKind::DominantThirteenth => "13",
    }
}

/// The chord's symbol in `key`.
///
/// # Errors
///
/// When the chord cannot be voiced (its notes would lie outside MIDI).
pub fn chord_symbol(
    chord: RomanChord,
    key: Key,
    numeral_standard: Option<NumeralStandard>,
) -> Result<String, VoicingError> {
    let pitches: Vec<Pitch> = chord_pitches(chord, key, numeral_standard, NAMING_OCTAVE)?;
    Ok(match pitches.first() {
        Some(root) => format!(
            "{}{}",
            note_name_text(root.name()),
            symbol_suffix(chord.kind())
        ),
        None => chord.to_string(),
    })
}

/// The symbol of a chord in an inversion: the chord, a slash, and the note
/// in the bass (C/E).
///
/// # Errors
///
/// When the chord cannot be voiced.
pub fn inverted_chord_symbol(
    chord: RomanChord,
    inversion: Inversion,
    key: Key,
    numeral_standard: Option<NumeralStandard>,
) -> Result<String, VoicingError> {
    let name: String = chord_symbol(chord, key, numeral_standard)?;
    let pitches: Vec<Pitch> =
        inverted_chord_pitches(chord, key, numeral_standard, NAMING_OCTAVE, inversion)?;
    Ok(match pitches.first() {
        Some(bass) => format!("{name}/{}", note_name_text(bass.name())),
        None => name,
    })
}

/// The numeral as the screen shows it, with the music's sharp and flat
/// signs in place of the file's `#` and `b`: `♭VII7`.
#[must_use]
pub fn numeral_text(chord: RomanChord) -> String {
    let written: String = chord.to_string();
    let accidental_text: &str = chord.accidental().text();
    let rest: &str = written.strip_prefix(accidental_text).unwrap_or(&written);
    let sign: &str = match chord.accidental() {
        Accidental::Flat => "♭",
        Accidental::Natural => "",
        Accidental::Sharp => "♯",
    };
    let rest: String = rest.replace("#9", "♯9").replace("b9", "♭9");
    format!("{sign}{rest}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::key::Mode;
    use vocabulary::pitch::NoteLetter;
    use vocabulary::roman_chord::ScaleDegree;

    #[test]
    fn symbols_follow_the_lead_sheet_convention() {
        let c_major: Key = Key::new(
            NoteName::new(NoteLetter::C, Accidental::Natural),
            Mode::Major,
        );
        let e_flat: Key = Key::new(NoteName::new(NoteLetter::E, Accidental::Flat), Mode::Major);
        let ii7: RomanChord = RomanChord::new(
            Accidental::Natural,
            ScaleDegree::Second,
            ChordKind::MinorSeventh,
        );
        let v7: RomanChord = RomanChord::new(
            Accidental::Natural,
            ScaleDegree::Fifth,
            ChordKind::DominantSeventh,
        );
        let flat_seven: RomanChord = RomanChord::new(
            Accidental::Flat,
            ScaleDegree::Seventh,
            ChordKind::DominantSeventh,
        );
        assert_eq!(chord_symbol(ii7, c_major, None).unwrap(), "Dm7");
        assert_eq!(chord_symbol(v7, e_flat, None).unwrap(), "B♭7");
        assert_eq!(chord_symbol(flat_seven, c_major, None).unwrap(), "B♭7");
        assert_eq!(numeral_text(flat_seven), "♭VII7");
        let first: Inversion = Inversion::new(1, ChordKind::Major).unwrap();
        let i: RomanChord =
            RomanChord::new(Accidental::Natural, ScaleDegree::First, ChordKind::Major);
        assert_eq!(
            inverted_chord_symbol(i, first, c_major, None).unwrap(),
            "C/E"
        );
    }
}
