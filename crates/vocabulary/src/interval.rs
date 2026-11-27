//! An interval from a tonic, as a `notes pitched` line writes each note
//! (Song Notation, section 4, "Intervals"): a quality and a number, above
//! the tonic or, with a minus sign, below it. `P5`, `m3`, `A4`, `M9`, `-P4`.
//!
//! The name carries the spelling: above C, `A1` is C♯ and `m2` is D♭. So an
//! interval is counted in staff steps (its number) and in semitones (its
//! quality), never in semitones alone.

use std::fmt;

use crate::key::Mode;
use crate::pitch::{Accidental, NoteLetter, NoteName, Pitch};

const STEPS_PER_OCTAVE: i16 = 7;
const SEMITONES_PER_OCTAVE: i16 = 12;

/// Semitones of the major or perfect interval on each staff step above the
/// tonic within an octave: unison 0, second 2, third 4, fourth 5, fifth 7,
/// sixth 9, seventh 11.
const MAJOR_OR_PERFECT_SEMITONES: [i16; 7] = [0, 2, 4, 5, 7, 9, 11];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalQuality {
    Perfect,
    Major,
    Minor,
    Augmented,
    Diminished,
}

impl IntervalQuality {
    /// As written in the files: `P`, `M`, `m`, `A`, `d`.
    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            IntervalQuality::Perfect => 'P',
            IntervalQuality::Major => 'M',
            IntervalQuality::Minor => 'm',
            IntervalQuality::Augmented => 'A',
            IntervalQuality::Diminished => 'd',
        }
    }

    #[must_use]
    pub const fn from_letter(letter: char) -> Option<IntervalQuality> {
        match letter {
            'P' => Some(IntervalQuality::Perfect),
            'M' => Some(IntervalQuality::Major),
            'm' => Some(IntervalQuality::Minor),
            'A' => Some(IntervalQuality::Augmented),
            'd' => Some(IntervalQuality::Diminished),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalDirection {
    Up,
    Down,
}

/// Semitones above the tonic of each step of the key's scale: major, or
/// natural minor (the key signature's scale).
const fn scale_of(mode: Mode) -> [i16; 7] {
    match mode {
        Mode::Major => MAJOR_OR_PERFECT_SEMITONES,
        Mode::Minor => [0, 2, 3, 5, 7, 8, 10],
    }
}

/// Unisons, fourths, fifths and their octaves are perfect; the others are
/// major or minor. `steps_within_octave` is 0 to 6.
const fn is_perfect_kind(steps_within_octave: i16) -> bool {
    matches!(steps_within_octave, 0 | 3 | 4)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Interval {
    quality: IntervalQuality,
    /// 1 for a unison, 8 for an octave, 9 for a ninth.
    number: u8,
    direction: IntervalDirection,
}

impl Interval {
    /// # Errors
    ///
    /// When the number is 0, the quality does not suit the number (`M5`,
    /// `P3`), the interval is a diminished unison, or a unison is written
    /// below the tonic.
    pub fn new(
        quality: IntervalQuality,
        number: u8,
        direction: IntervalDirection,
    ) -> Result<Interval, IntervalError> {
        if number == 0 {
            return Err(IntervalError::ZeroNumber);
        }
        let steps_within_octave: i16 = (i16::from(number) - 1) % STEPS_PER_OCTAVE;
        let fits: bool = match quality {
            IntervalQuality::Perfect => is_perfect_kind(steps_within_octave),
            IntervalQuality::Major | IntervalQuality::Minor => {
                !is_perfect_kind(steps_within_octave)
            }
            IntervalQuality::Augmented | IntervalQuality::Diminished => true,
        };
        if !fits {
            return Err(IntervalError::QualityDoesNotFitNumber { quality, number });
        }
        if number == 1 && quality == IntervalQuality::Diminished {
            return Err(IntervalError::DiminishedUnison);
        }
        if number == 1
            && quality == IntervalQuality::Perfect
            && direction == IntervalDirection::Down
        {
            return Err(IntervalError::UnisonBelow);
        }
        Ok(Interval {
            quality,
            number,
            direction,
        })
    }

    #[must_use]
    pub const fn quality(self) -> IntervalQuality {
        self.quality
    }

    #[must_use]
    pub const fn number(self) -> u8 {
        self.number
    }

    #[must_use]
    pub const fn direction(self) -> IntervalDirection {
        self.direction
    }

    /// Staff steps from the tonic: `number − 1`, negative below it.
    fn signed_steps(self) -> i16 {
        let steps: i16 = i16::from(self.number) - 1;
        match self.direction {
            IntervalDirection::Up => steps,
            IntervalDirection::Down => -steps,
        }
    }

    /// The interval's size in semitones, negative below the tonic:
    /// `major or perfect semitones of the step + 12 semitones/octave ×
    /// octaves + the quality's semitones`.
    fn signed_semitones(self) -> i16 {
        let steps: i16 = i16::from(self.number) - 1;
        let steps_within_octave: i16 = steps % STEPS_PER_OCTAVE;
        let octaves: i16 = steps / STEPS_PER_OCTAVE;
        let quality_semitones: i16 = match (self.quality, is_perfect_kind(steps_within_octave)) {
            (IntervalQuality::Perfect | IntervalQuality::Major, _) => 0,
            (IntervalQuality::Minor, _) | (IntervalQuality::Diminished, true) => -1,
            (IntervalQuality::Diminished, false) => -2,
            (IntervalQuality::Augmented, _) => 1,
        };
        let size: i16 = MAJOR_OR_PERFECT_SEMITONES[usize::from(steps_within_octave.unsigned_abs())]
            + SEMITONES_PER_OCTAVE * octaves
            + quality_semitones;
        match self.direction {
            IntervalDirection::Up => size,
            IntervalDirection::Down => -size,
        }
    }

    /// The same scale step in the other mode's scale, with the same
    /// alteration (Editor Draft, "A key override in F1"): from major to
    /// minor `M3` becomes `m3` and `A4` stays `A4`; a step the two scales
    /// share keeps its interval.
    ///
    /// # Errors
    ///
    /// When the result is no interval the notation writes.
    pub fn in_mode(self, from: Mode, to: Mode) -> Result<Interval, IntervalError> {
        let steps: i16 = self.signed_steps();
        let step_in_octave: usize = usize::from(steps.rem_euclid(STEPS_PER_OCTAVE).unsigned_abs());
        let octaves: i16 = steps.div_euclid(STEPS_PER_OCTAVE);
        // semitones of the scale's step + 12 semitones/octave × octaves.
        let scale_step =
            |mode: Mode| -> i16 { scale_of(mode)[step_in_octave] + SEMITONES_PER_OCTAVE * octaves };
        let alteration: i16 = self.signed_semitones() - scale_step(from);
        Interval::from_size(steps, scale_step(to) + alteration)
    }

    /// Whether the interval is a note of the key's scale: in major P1 M2 M3
    /// P4 P5 M6 M7, in natural minor P1 M2 m3 P4 P5 m6 m7, and their
    /// octaves above and below (Song Notation, section 4).
    #[must_use]
    pub fn is_diatonic(self, mode: Mode) -> bool {
        // Below the tonic, the note's place in the scale is counted up from
        // the tonic an octave lower: −m3 is the M6 of the octave below.
        let semitones_in_octave: i16 = self.signed_semitones().rem_euclid(SEMITONES_PER_OCTAVE);
        let steps_in_octave: i16 = self.signed_steps().rem_euclid(STEPS_PER_OCTAVE);
        scale_of(mode)[usize::from(steps_in_octave.unsigned_abs())] == semitones_in_octave
    }

    /// The pitch this interval names from `tonic`: its letter is `number −
    /// 1` staff steps away, and its accidental makes up the semitones.
    ///
    /// # Errors
    ///
    /// When the pitch would need a double sharp or double flat, which the
    /// notation does not write.
    pub fn from_tonic(self, tonic: Pitch) -> Result<Pitch, IntervalError> {
        let target_steps: i16 = tonic.staff_steps_above_c0() + self.signed_steps();
        let octave: i16 = target_steps.div_euclid(STEPS_PER_OCTAVE);
        let letter: NoteLetter = NoteLetter::from_steps_above_c(
            target_steps.rem_euclid(STEPS_PER_OCTAVE),
        )
        .ok_or(IntervalError::OutOfRange {
            interval: self,
            tonic,
        })?;
        let octave_i8: i8 = i8::try_from(octave).map_err(|_| IntervalError::OutOfRange {
            interval: self,
            tonic,
        })?;
        let natural: Pitch = Pitch::new(NoteName::new(letter, Accidental::Natural), octave_i8);
        // semitones above C−1 of the target − of its letter unaltered →
        // semitones the accidental adds.
        let wanted_semitones: i16 = tonic.semitones_above_c_minus_1() + self.signed_semitones();
        let accidental: Accidental = Accidental::from_semitone_offset(
            wanted_semitones - natural.semitones_above_c_minus_1(),
        )
        .ok_or(IntervalError::NeedsDoubleAccidental {
            interval: self,
            tonic,
        })?;
        Ok(Pitch::new(NoteName::new(letter, accidental), octave_i8))
    }

    /// The interval from `tonic` to `pitch`, spelled by their letters.
    ///
    /// # Errors
    ///
    /// When the two spell no interval the notation writes (a diminished
    /// unison, or one more than once augmented or diminished).
    pub fn between(tonic: Pitch, pitch: Pitch) -> Result<Interval, IntervalError> {
        let signed_steps: i16 = pitch.staff_steps_above_c0() - tonic.staff_steps_above_c0();
        let signed_semitones: i16 =
            pitch.semitones_above_c_minus_1() - tonic.semitones_above_c_minus_1();
        Interval::from_size(signed_steps, signed_semitones)
            .map_err(|_| IntervalError::NotWritten { tonic, pitch })
    }

    /// The interval of `signed_steps` staff steps and `signed_semitones`,
    /// both negative below the tonic.
    ///
    /// # Errors
    ///
    /// When the two make no interval the notation writes (a diminished
    /// unison, or one more than once augmented or diminished).
    pub fn from_size(signed_steps: i16, signed_semitones: i16) -> Result<Interval, IntervalError> {
        let not_written = || IntervalError::SizeNotWritten {
            signed_steps,
            signed_semitones,
        };
        // A unison is up by definition; a lowered unison is a diminished
        // unison, which the notation does not write.
        let (direction, steps, semitones): (IntervalDirection, i16, i16) =
            if signed_steps > 0 || (signed_steps == 0 && signed_semitones >= 0) {
                (IntervalDirection::Up, signed_steps, signed_semitones)
            } else {
                (IntervalDirection::Down, -signed_steps, -signed_semitones)
            };
        let steps_within_octave: i16 = steps % STEPS_PER_OCTAVE;
        let base: i16 = MAJOR_OR_PERFECT_SEMITONES[usize::from(steps_within_octave.unsigned_abs())]
            + SEMITONES_PER_OCTAVE * (steps / STEPS_PER_OCTAVE);
        let quality: IntervalQuality =
            match (is_perfect_kind(steps_within_octave), semitones - base) {
                (true, 0) => IntervalQuality::Perfect,
                (false, 0) => IntervalQuality::Major,
                (false, -1) => IntervalQuality::Minor,
                (_, 1) => IntervalQuality::Augmented,
                (true, -1) | (false, -2) => IntervalQuality::Diminished,
                _ => return Err(not_written()),
            };
        let number: u8 = u8::try_from(steps + 1).map_err(|_| not_written())?;
        Interval::new(quality, number, direction).map_err(|_| not_written())
    }
}

/// The pitch of `tonic` nearest `own_tonic`, above or below it: where a
/// line measured from `own_tonic` is measured from under a key override
/// (Song Notation, section 4). A tritone, equally far either way, goes
/// below.
#[must_use]
pub fn nearest_tonic(own_tonic: Pitch, tonic: NoteName) -> Pitch {
    let own_semitones: i16 = own_tonic.semitones_above_c_minus_1();
    let octave: i8 = own_tonic.octave();
    [octave.saturating_sub(1), octave, octave.saturating_add(1)]
        .into_iter()
        .map(|candidate: i8| Pitch::new(tonic, candidate))
        // Nearest first; of two as near, the one below (false sorts first).
        .min_by_key(|candidate: &Pitch| {
            let distance_semitones: i16 = candidate.semitones_above_c_minus_1() - own_semitones;
            (distance_semitones.abs(), distance_semitones > 0)
        })
        .unwrap_or(Pitch::new(tonic, octave))
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign: &str = match self.direction {
            IntervalDirection::Up => "",
            IntervalDirection::Down => "-",
        };
        write!(f, "{sign}{}{}", self.quality.letter(), self.number)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalError {
    ZeroNumber,
    QualityDoesNotFitNumber {
        quality: IntervalQuality,
        number: u8,
    },
    DiminishedUnison,
    UnisonBelow,
    NeedsDoubleAccidental {
        interval: Interval,
        tonic: Pitch,
    },
    OutOfRange {
        interval: Interval,
        tonic: Pitch,
    },
    NotWritten {
        tonic: Pitch,
        pitch: Pitch,
    },
    SizeNotWritten {
        signed_steps: i16,
        signed_semitones: i16,
    },
}

impl fmt::Display for IntervalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntervalError::ZeroNumber => write!(
                f,
                "an interval is counted from 1, the unison. Write e.g. P1, m3 or P5"
            ),
            IntervalError::QualityDoesNotFitNumber { quality, number } => {
                let advice: &str = if *quality == IntervalQuality::Perfect {
                    "a second, third, sixth or seventh is major (M) or minor (m)"
                } else {
                    "a unison, fourth, fifth or octave is perfect (P)"
                };
                write!(
                    f,
                    "{}{number} is not an interval: {advice}, and any interval may be \
                     augmented (A) or diminished (d)",
                    quality.letter()
                )
            }
            IntervalError::DiminishedUnison => write!(
                f,
                "d1 is not an interval. Write -A1 for a note a semitone below the tonic on \
                 the same letter, or -m2"
            ),
            IntervalError::UnisonBelow => write!(f, "-P1 is the tonic itself. Write P1"),
            IntervalError::NeedsDoubleAccidental { interval, tonic } => write!(
                f,
                "{interval} from {tonic} needs a double sharp or double flat, which the \
                 notation does not write. Write the same note as the interval on the next \
                 letter, e.g. m2 instead of A1"
            ),
            IntervalError::OutOfRange { interval, tonic } => {
                write!(f, "{interval} from {tonic} is outside what can be written")
            }
            IntervalError::NotWritten { tonic, pitch } => write!(
                f,
                "{pitch} is no interval the notation writes from {tonic}. Respell the note"
            ),
            IntervalError::SizeNotWritten {
                signed_steps,
                signed_semitones,
            } => write!(
                f,
                "{signed_steps} staff steps of {signed_semitones} semitones is no interval the \
                 notation writes"
            ),
        }
    }
}

impl std::error::Error for IntervalError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pitch(letter: NoteLetter, accidental: Accidental, octave: i8) -> Pitch {
        Pitch::new(NoteName::new(letter, accidental), octave)
    }

    fn up(quality: IntervalQuality, number: u8) -> Interval {
        Interval::new(quality, number, IntervalDirection::Up).unwrap()
    }

    fn down(quality: IntervalQuality, number: u8) -> Interval {
        Interval::new(quality, number, IntervalDirection::Down).unwrap()
    }

    #[test]
    fn the_name_gives_the_spelling() {
        let c2: Pitch = pitch(NoteLetter::C, Accidental::Natural, 2);
        let a1: Pitch = up(IntervalQuality::Augmented, 1).from_tonic(c2).unwrap();
        let m2: Pitch = up(IntervalQuality::Minor, 2).from_tonic(c2).unwrap();
        assert_eq!(a1.to_string(), "C#2");
        assert_eq!(m2.to_string(), "Db2");
        assert_eq!(a1.note_number().unwrap(), m2.note_number().unwrap());
    }

    #[test]
    fn compound_and_downward_intervals() {
        let c2: Pitch = pitch(NoteLetter::C, Accidental::Natural, 2);
        let of = |interval: Interval| interval.from_tonic(c2).unwrap().to_string();
        assert_eq!(of(up(IntervalQuality::Major, 9)), "D3");
        assert_eq!(of(up(IntervalQuality::Perfect, 12)), "G3");
        assert_eq!(of(up(IntervalQuality::Major, 7)), "B2");
        assert_eq!(of(down(IntervalQuality::Perfect, 4)), "G1");
        assert_eq!(of(down(IntervalQuality::Perfect, 8)), "C1");
        assert_eq!(of(down(IntervalQuality::Minor, 3)), "A1");
    }

    #[test]
    fn a_transposition_keeps_the_interval_and_respells() {
        let eb2: Pitch = pitch(NoteLetter::E, Accidental::Flat, 2);
        // C# above C is E natural above Eb, not Fb.
        assert_eq!(
            up(IntervalQuality::Augmented, 1)
                .from_tonic(eb2)
                .unwrap()
                .to_string(),
            "E2"
        );
        assert_eq!(
            up(IntervalQuality::Perfect, 5)
                .from_tonic(eb2)
                .unwrap()
                .to_string(),
            "Bb2"
        );
    }

    #[test]
    fn a_double_accidental_is_refused() {
        let f_sharp: Pitch = pitch(NoteLetter::F, Accidental::Sharp, 2);
        assert!(matches!(
            up(IntervalQuality::Augmented, 1).from_tonic(f_sharp),
            Err(IntervalError::NeedsDoubleAccidental { .. })
        ));
    }

    #[test]
    fn between_is_the_inverse_of_from_tonic() {
        let tonics: [Pitch; 3] = [
            pitch(NoteLetter::C, Accidental::Natural, 2),
            pitch(NoteLetter::E, Accidental::Flat, 3),
            pitch(NoteLetter::A, Accidental::Natural, 1),
        ];
        let intervals: [Interval; 8] = [
            up(IntervalQuality::Perfect, 1),
            up(IntervalQuality::Minor, 3),
            up(IntervalQuality::Augmented, 4),
            up(IntervalQuality::Major, 6),
            up(IntervalQuality::Major, 9),
            down(IntervalQuality::Perfect, 4),
            down(IntervalQuality::Major, 6),
            up(IntervalQuality::Minor, 7),
        ];
        for tonic in tonics {
            for interval in intervals {
                let pitch: Pitch = interval.from_tonic(tonic).unwrap();
                assert_eq!(Interval::between(tonic, pitch).unwrap(), interval);
            }
        }
    }

    #[test]
    fn quality_must_suit_the_number() {
        assert!(Interval::new(IntervalQuality::Major, 5, IntervalDirection::Up).is_err());
        assert!(Interval::new(IntervalQuality::Perfect, 3, IntervalDirection::Up).is_err());
        assert!(Interval::new(IntervalQuality::Perfect, 11, IntervalDirection::Up).is_ok());
        assert!(Interval::new(IntervalQuality::Diminished, 1, IntervalDirection::Up).is_err());
        assert!(Interval::new(IntervalQuality::Perfect, 1, IntervalDirection::Down).is_err());
    }

    #[test]
    fn diatonic_follows_the_key_signature() {
        assert!(up(IntervalQuality::Major, 3).is_diatonic(Mode::Major));
        assert!(!up(IntervalQuality::Major, 3).is_diatonic(Mode::Minor));
        assert!(up(IntervalQuality::Minor, 7).is_diatonic(Mode::Minor));
        assert!(!up(IntervalQuality::Major, 7).is_diatonic(Mode::Minor));
        assert!(!up(IntervalQuality::Augmented, 4).is_diatonic(Mode::Major));
        // A minor third below the tonic is the major sixth of the octave below.
        assert!(down(IntervalQuality::Minor, 3).is_diatonic(Mode::Major));
        assert!(up(IntervalQuality::Major, 9).is_diatonic(Mode::Major));
    }

    #[test]
    fn an_override_takes_the_nearest_tonic() {
        let c2: Pitch = pitch(NoteLetter::C, Accidental::Natural, 2);
        let of = |letter: NoteLetter, accidental: Accidental| {
            nearest_tonic(c2, NoteName::new(letter, accidental)).to_string()
        };
        assert_eq!(of(NoteLetter::B, Accidental::Natural), "B1");
        assert_eq!(of(NoteLetter::A, Accidental::Natural), "A1");
        assert_eq!(of(NoteLetter::E, Accidental::Natural), "E2");
        assert_eq!(of(NoteLetter::D, Accidental::Natural), "D2");
        // A tritone either way: below.
        assert_eq!(of(NoteLetter::F, Accidental::Sharp), "F#1");
        assert_eq!(of(NoteLetter::C, Accidental::Natural), "C2");
    }

    #[test]
    fn a_step_keeps_its_place_in_the_other_mode() {
        let to_minor = |interval: Interval| {
            interval
                .in_mode(Mode::Major, Mode::Minor)
                .unwrap()
                .to_string()
        };
        assert_eq!(to_minor(up(IntervalQuality::Major, 3)), "m3");
        assert_eq!(to_minor(up(IntervalQuality::Major, 6)), "m6");
        assert_eq!(to_minor(up(IntervalQuality::Major, 7)), "m7");
        assert_eq!(to_minor(up(IntervalQuality::Perfect, 5)), "P5");
        assert_eq!(to_minor(up(IntervalQuality::Augmented, 4)), "A4");
        assert_eq!(to_minor(up(IntervalQuality::Major, 10)), "m10");
        // Below the tonic: -m3 is the sixth step of the octave below.
        assert_eq!(to_minor(down(IntervalQuality::Minor, 3)), "-M3");
        assert_eq!(
            up(IntervalQuality::Minor, 3)
                .in_mode(Mode::Minor, Mode::Major)
                .unwrap()
                .to_string(),
            "M3"
        );
    }

    #[test]
    fn written_as_in_the_files() {
        assert_eq!(up(IntervalQuality::Perfect, 5).to_string(), "P5");
        assert_eq!(down(IntervalQuality::Perfect, 4).to_string(), "-P4");
        assert_eq!(up(IntervalQuality::Minor, 3).to_string(), "m3");
    }
}
