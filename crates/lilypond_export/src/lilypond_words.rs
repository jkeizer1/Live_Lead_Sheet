//! LilyPond's words for one thing at a time: a pitch, a note value, a key,
//! the marks on a note, a quoted string. LilyPond's default note names are
//! Dutch: `c d e f g a b`, `is` for a sharp and `es` for a flat (`fis`,
//! `bes`; E flat and A flat are `es` and `as`). Octaves are absolute: `c'`
//! is middle C (C4), each `'` an octave up and each `,` an octave down.

use note_spelling::note_value::NoteValue;
use vocabulary::key::{Key, Mode};
use vocabulary::pitch::{Accidental, NoteLetter, NoteName, Pitch};
use vocabulary::rhythm_pattern::{AccentLevel, ElementMarks};

/// The octave LilyPond writes with no mark: `c` is C3.
const UNMARKED_OCTAVE: i8 = 3;

/// `Eb` → `es`, `F#` → `fis`, `B` → `b`.
#[must_use]
pub fn note_name(name: NoteName) -> String {
    let letter: &str = match name.letter() {
        NoteLetter::C => "c",
        NoteLetter::D => "d",
        NoteLetter::E => "e",
        NoteLetter::F => "f",
        NoteLetter::G => "g",
        NoteLetter::A => "a",
        NoteLetter::B => "b",
    };
    match (name.letter(), name.accidental()) {
        (_, Accidental::Natural) => letter.to_string(),
        (_, Accidental::Sharp) => format!("{letter}is"),
        // The two vowels take a plain s: es, as.
        (NoteLetter::E | NoteLetter::A, Accidental::Flat) => format!("{letter}s"),
        (_, Accidental::Flat) => format!("{letter}es"),
    }
}

/// `C4` → `c'`, `Bb1` → `bes,,`, `G3` → `g`.
#[must_use]
pub fn pitch(written: Pitch) -> String {
    let octave_steps: i8 = written.octave() - UNMARKED_OCTAVE;
    let marks: String = if octave_steps >= 0 {
        "'".repeat(usize::from(octave_steps.unsigned_abs()))
    } else {
        ",".repeat(usize::from(octave_steps.unsigned_abs()))
    };
    format!("{}{marks}", note_name(written.name()))
}

/// A quarter → `4`, a dotted eighth → `8.`.
#[must_use]
pub fn duration(value: NoteValue) -> String {
    format!(
        "{}{}",
        value.base().per_whole_note(),
        ".".repeat(usize::from(value.dots()))
    )
}

/// `C major` → `\key c \major`.
#[must_use]
pub fn key(written: Key) -> String {
    let mode: &str = match written.mode() {
        Mode::Major => "\\major",
    };
    format!("\\key {} {mode}", note_name(written.tonic()))
}

/// Accent `->`, strong accent (marcato) `-^`, staccato `-.`, fermata
/// `\fermata`. Legato is a slur, written by the caller across notes.
#[must_use]
pub fn marks(written: ElementMarks) -> String {
    let mut text: String = String::new();
    match written.accent() {
        AccentLevel::Normal => {}
        AccentLevel::Accent => text.push_str("->"),
        AccentLevel::Strong => text.push_str("-^"),
    }
    if written.staccato() {
        text.push_str("-.");
    }
    if written.fermata() {
        text.push_str("\\fermata");
    }
    text
}

/// A LilyPond string in quotes, with `\` and `"` escaped.
#[must_use]
pub fn quoted(text: &str) -> String {
    let escaped: String = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use note_spelling::note_value::BaseValue;

    fn at(letter: NoteLetter, accidental: Accidental, octave: i8) -> Pitch {
        Pitch::new(NoteName::new(letter, accidental), octave)
    }

    #[test]
    fn pitches_are_dutch_names_with_absolute_octaves() {
        assert_eq!(pitch(at(NoteLetter::C, Accidental::Natural, 4)), "c'");
        assert_eq!(pitch(at(NoteLetter::B, Accidental::Flat, 1)), "bes,,");
        assert_eq!(pitch(at(NoteLetter::E, Accidental::Flat, 5)), "es''");
        assert_eq!(pitch(at(NoteLetter::A, Accidental::Flat, 3)), "as");
        assert_eq!(pitch(at(NoteLetter::F, Accidental::Sharp, 2)), "fis,");
    }

    #[test]
    fn durations_and_keys() {
        assert_eq!(duration(NoteValue::new(BaseValue::Eighth, 1)), "8.");
        assert_eq!(duration(NoteValue::new(BaseValue::Whole, 0)), "1");
        assert_eq!(
            key(Key::new(
                NoteName::new(NoteLetter::E, Accidental::Flat),
                Mode::Major
            )),
            "\\key es \\major"
        );
    }

    #[test]
    fn strings_are_escaped() {
        assert_eq!(quoted("Start \"SQ-1\""), "\"Start \\\"SQ-1\\\"\"");
    }
}
