//! A pitch as written on a staff: letter, accidental and octave, with C4 the
//! middle C of the piano (Song Notation, section 2). It is a place on the
//! staff, not a MIDI note number; `note_number` makes the number the
//! standard way (C4 = 60), before any octave offset a device needs.

use std::fmt;

use crate::midi_values::NoteNumber;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteLetter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl NoteLetter {
    /// Semitones above C within the octave: C 0, D 2, E 4, F 5, G 7, A 9, B 11.
    #[must_use]
    pub const fn semitones_above_c(self) -> i16 {
        match self {
            NoteLetter::C => 0,
            NoteLetter::D => 2,
            NoteLetter::E => 4,
            NoteLetter::F => 5,
            NoteLetter::G => 7,
            NoteLetter::A => 9,
            NoteLetter::B => 11,
        }
    }

    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            NoteLetter::C => 'C',
            NoteLetter::D => 'D',
            NoteLetter::E => 'E',
            NoteLetter::F => 'F',
            NoteLetter::G => 'G',
            NoteLetter::A => 'A',
            NoteLetter::B => 'B',
        }
    }

    #[must_use]
    pub const fn from_letter(letter: char) -> Option<NoteLetter> {
        match letter {
            'C' => Some(NoteLetter::C),
            'D' => Some(NoteLetter::D),
            'E' => Some(NoteLetter::E),
            'F' => Some(NoteLetter::F),
            'G' => Some(NoteLetter::G),
            'A' => Some(NoteLetter::A),
            'B' => Some(NoteLetter::B),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Accidental {
    Flat,
    Natural,
    Sharp,
}

impl Accidental {
    /// Semitones added to the letter's pitch: flat −1, natural 0, sharp +1.
    #[must_use]
    pub const fn semitone_offset(self) -> i16 {
        match self {
            Accidental::Flat => -1,
            Accidental::Natural => 0,
            Accidental::Sharp => 1,
        }
    }

    /// As written in the files: `b`, nothing, `#`.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Accidental::Flat => "b",
            Accidental::Natural => "",
            Accidental::Sharp => "#",
        }
    }
}

/// A letter with its accidental: a pitch class as the music spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteName {
    letter: NoteLetter,
    accidental: Accidental,
}

impl NoteName {
    #[must_use]
    pub const fn new(letter: NoteLetter, accidental: Accidental) -> NoteName {
        NoteName { letter, accidental }
    }

    #[must_use]
    pub const fn letter(self) -> NoteLetter {
        self.letter
    }

    #[must_use]
    pub const fn accidental(self) -> Accidental {
        self.accidental
    }
}

impl fmt::Display for NoteName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.letter.letter(), self.accidental.text())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pitch {
    name: NoteName,
    octave: i8,
}

impl Pitch {
    #[must_use]
    pub const fn new(name: NoteName, octave: i8) -> Pitch {
        Pitch { name, octave }
    }

    #[must_use]
    pub const fn name(self) -> NoteName {
        self.name
    }

    #[must_use]
    pub const fn octave(self) -> i8 {
        self.octave
    }

    /// `(octave + 1) × 12 semitones/octave + letter semitones + accidental
    /// semitones` → MIDI note number; C4 is 5 × 12 = 60.
    ///
    /// # Errors
    ///
    /// When the pitch lies outside the MIDI range 0 to 127 (C−1 to G9).
    pub fn note_number(self) -> Result<NoteNumber, PitchError> {
        let number: i16 = (i16::from(self.octave) + 1) * 12
            + self.name.letter.semitones_above_c()
            + self.name.accidental.semitone_offset();
        u8::try_from(number)
            .ok()
            .and_then(|value: u8| NoteNumber::new(value).ok())
            .ok_or(PitchError::OutsideMidiRange { pitch: self })
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.name, self.octave)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchError {
    OutsideMidiRange { pitch: Pitch },
}

impl fmt::Display for PitchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PitchError::OutsideMidiRange { pitch } => write!(
                f,
                "{pitch} is outside what MIDI can play (C-1 to G9). Use a nearer octave"
            ),
        }
    }
}

impl std::error::Error for PitchError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pitch(letter: NoteLetter, accidental: Accidental, octave: i8) -> Pitch {
        Pitch::new(NoteName::new(letter, accidental), octave)
    }

    #[test]
    fn middle_c_is_60() {
        let c4: Pitch = pitch(NoteLetter::C, Accidental::Natural, 4);
        assert_eq!(c4.note_number().unwrap().value(), 60);
    }

    #[test]
    fn bass_notes_and_accidentals() {
        assert_eq!(
            pitch(NoteLetter::C, Accidental::Natural, 2)
                .note_number()
                .unwrap()
                .value(),
            36
        );
        assert_eq!(
            pitch(NoteLetter::F, Accidental::Sharp, 2)
                .note_number()
                .unwrap()
                .value(),
            42
        );
        assert_eq!(
            pitch(NoteLetter::B, Accidental::Flat, 1)
                .note_number()
                .unwrap()
                .value(),
            34
        );
    }

    #[test]
    fn the_ends_of_the_midi_range() {
        assert_eq!(
            pitch(NoteLetter::C, Accidental::Natural, -1)
                .note_number()
                .unwrap()
                .value(),
            0
        );
        assert_eq!(
            pitch(NoteLetter::G, Accidental::Natural, 9)
                .note_number()
                .unwrap()
                .value(),
            127
        );
        assert!(
            pitch(NoteLetter::G, Accidental::Sharp, 9)
                .note_number()
                .is_err()
        );
        assert!(
            pitch(NoteLetter::C, Accidental::Flat, -1)
                .note_number()
                .is_err()
        );
    }

    #[test]
    fn written_as_in_the_files() {
        assert_eq!(pitch(NoteLetter::B, Accidental::Flat, 1).to_string(), "Bb1");
        assert_eq!(
            pitch(NoteLetter::F, Accidental::Sharp, 2).to_string(),
            "F#2"
        );
    }
}
