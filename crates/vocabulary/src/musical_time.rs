//! Musical time: a whole number of ticks, 5040 per quarter note (D-7).
//!
//! Whole numbers make adding and comparing positions exact, so no rounding
//! drift can build up. "Tick" means only this unit; MIDI's 24-per-quarter
//! messages are clock pulses and live with the clock, not here.

use std::fmt;

/// 5040 = 2⁴ · 3² · 5 · 7, so triplets, quintuplets, septuplets, dotted
/// values and sixty-fourth notes are all whole numbers of ticks.
pub const TICKS_PER_QUARTER_NOTE: u64 = 5040;

pub const TICKS_PER_WHOLE_NOTE: u64 = 4 * TICKS_PER_QUARTER_NOTE;

/// A position or a length in musical time, counted in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MusicalTime {
    ticks: u64,
}

impl MusicalTime {
    pub const ZERO: MusicalTime = MusicalTime { ticks: 0 };

    #[must_use]
    pub const fn from_ticks(ticks: u64) -> MusicalTime {
        MusicalTime { ticks }
    }

    #[must_use]
    pub const fn ticks(self) -> u64 {
        self.ticks
    }

    /// A note value as a fraction of a whole note: 1/4 is a quarter note,
    /// 3/8 a dotted quarter, 1/12 a triplet eighth.
    ///
    /// # Errors
    ///
    /// When the denominator is zero, when the value is not a whole number
    /// of ticks (for example 1/11), or when it is too long to count.
    pub fn from_note_value(
        numerator: u64,
        denominator: u64,
    ) -> Result<MusicalTime, MusicalTimeError> {
        if denominator == 0 {
            return Err(MusicalTimeError::ZeroDenominator { numerator });
        }
        let Some(scaled): Option<u64> = numerator.checked_mul(TICKS_PER_WHOLE_NOTE) else {
            return Err(MusicalTimeError::NoteValueTooLong {
                numerator,
                denominator,
            });
        };
        if scaled % denominator != 0 {
            return Err(MusicalTimeError::NotWholeTicks {
                numerator,
                denominator,
            });
        }
        Ok(MusicalTime {
            ticks: scaled / denominator,
        })
    }

    /// # Errors
    ///
    /// When the sum is too large to count; never wraps around.
    pub fn checked_add(self, other: MusicalTime) -> Result<MusicalTime, MusicalTimeError> {
        match self.ticks.checked_add(other.ticks) {
            Some(ticks) => Ok(MusicalTime { ticks }),
            None => Err(MusicalTimeError::SumTooLong {
                left: self,
                right: other,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicalTimeError {
    ZeroDenominator {
        numerator: u64,
    },
    NotWholeTicks {
        numerator: u64,
        denominator: u64,
    },
    NoteValueTooLong {
        numerator: u64,
        denominator: u64,
    },
    SumTooLong {
        left: MusicalTime,
        right: MusicalTime,
    },
}

impl fmt::Display for MusicalTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MusicalTimeError::ZeroDenominator { numerator } => {
                write!(f, "note value {numerator}/0 has a zero denominator")
            }
            MusicalTimeError::NotWholeTicks {
                numerator,
                denominator,
            } => write!(
                f,
                "note value {numerator}/{denominator} of a whole note is not a whole number of ticks \
                 ({TICKS_PER_QUARTER_NOTE} per quarter note)"
            ),
            MusicalTimeError::NoteValueTooLong {
                numerator,
                denominator,
            } => write!(
                f,
                "note value {numerator}/{denominator} of a whole note is too long to count in ticks"
            ),
            MusicalTimeError::SumTooLong { left, right } => write!(
                f,
                "adding {} ticks to {} ticks is too long to count",
                right.ticks, left.ticks
            ),
        }
    }
}

impl std::error::Error for MusicalTimeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(numerator: u64, denominator: u64) -> u64 {
        MusicalTime::from_note_value(numerator, denominator)
            .unwrap()
            .ticks()
    }

    #[test]
    fn plain_note_values() {
        assert_eq!(note(1, 1), 20160);
        assert_eq!(note(1, 4), 5040);
        assert_eq!(note(1, 8), 2520);
        assert_eq!(note(1, 16), 1260);
        assert_eq!(note(1, 64), 315);
    }

    #[test]
    fn dotted_values() {
        assert_eq!(note(3, 8), 7560); // dotted quarter
        assert_eq!(note(3, 16), 3780); // dotted eighth
        assert_eq!(note(7, 16), 8820); // double-dotted quarter
    }

    #[test]
    fn tuplets_are_exact() {
        assert_eq!(note(1, 12), 1680); // triplet eighth
        assert_eq!(note(1, 20), 1008); // quintuplet sixteenth
        assert_eq!(note(1, 28), 720); // septuplet sixteenth
        assert_eq!(note(1, 6) * 3, note(1, 2)); // three triplet quarters fill a half
    }

    #[test]
    fn values_that_are_not_whole_ticks_are_refused() {
        assert_eq!(
            MusicalTime::from_note_value(1, 11),
            Err(MusicalTimeError::NotWholeTicks {
                numerator: 1,
                denominator: 11
            })
        );
        assert_eq!(
            MusicalTime::from_note_value(1, 128),
            Err(MusicalTimeError::NotWholeTicks {
                numerator: 1,
                denominator: 128
            })
        );
    }

    #[test]
    fn zero_denominator_is_refused() {
        assert_eq!(
            MusicalTime::from_note_value(1, 0),
            Err(MusicalTimeError::ZeroDenominator { numerator: 1 })
        );
    }

    #[test]
    fn huge_note_value_is_refused() {
        assert_eq!(
            MusicalTime::from_note_value(u64::MAX, 1),
            Err(MusicalTimeError::NoteValueTooLong {
                numerator: u64::MAX,
                denominator: 1
            })
        );
    }

    #[test]
    fn adding_is_exact_and_overflow_is_reported() {
        let bar: MusicalTime = MusicalTime::from_note_value(1, 1).unwrap();
        let two_bars: MusicalTime = bar.checked_add(bar).unwrap();
        assert_eq!(two_bars.ticks(), 2 * TICKS_PER_WHOLE_NOTE);
        assert!(bar < two_bars);

        let last: MusicalTime = MusicalTime::from_ticks(u64::MAX);
        assert_eq!(
            last.checked_add(bar),
            Err(MusicalTimeError::SumTooLong {
                left: last,
                right: bar
            })
        );
    }

    #[test]
    fn errors_say_what_was_attempted() {
        let message: String = MusicalTimeError::NotWholeTicks {
            numerator: 1,
            denominator: 11,
        }
        .to_string();
        assert_eq!(
            message,
            "note value 1/11 of a whole note is not a whole number of ticks (5040 per quarter note)"
        );
    }
}
