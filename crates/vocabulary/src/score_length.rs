//! Score length (ΔP): a distance along the song, counted in ticks.
//!
//! A note value is a length, so note values are made here. Lengths are
//! forward-only: never negative. See section 10 of
//! docs/Architecture/Musical Time and Clock.md for the allowed operations.

use std::fmt;

use crate::ticks::{TICKS_PER_QUARTER_NOTE, TICKS_PER_WHOLE_NOTE};

/// A distance along the song, in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScoreLength {
    length_ticks: u64,
}

impl ScoreLength {
    pub const ZERO: ScoreLength = ScoreLength { length_ticks: 0 };

    #[must_use]
    pub const fn from_ticks(length_ticks: u64) -> ScoreLength {
        ScoreLength { length_ticks }
    }

    #[must_use]
    pub const fn ticks(self) -> u64 {
        self.length_ticks
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
    ) -> Result<ScoreLength, ScoreLengthError> {
        if denominator == 0 {
            return Err(ScoreLengthError::ZeroDenominator { numerator });
        }
        // numerator/denominator w × 20160 ticks/w → ticks; multiply first so
        // the one division can be checked for a remainder.
        let Some(scaled_ticks): Option<u64> = numerator.checked_mul(TICKS_PER_WHOLE_NOTE) else {
            return Err(ScoreLengthError::NoteValueTooLong {
                numerator,
                denominator,
            });
        };
        if scaled_ticks % denominator != 0 {
            return Err(ScoreLengthError::NotWholeTicks {
                numerator,
                denominator,
            });
        }
        Ok(ScoreLength {
            length_ticks: scaled_ticks / denominator,
        })
    }

    /// Length + length → length.
    ///
    /// # Errors
    ///
    /// When the sum is too long to count; never wraps around.
    pub fn checked_add(self, other: ScoreLength) -> Result<ScoreLength, ScoreLengthError> {
        match self.length_ticks.checked_add(other.length_ticks) {
            Some(length_ticks) => Ok(ScoreLength { length_ticks }),
            None => Err(ScoreLengthError::SumTooLong {
                left: self,
                right: other,
            }),
        }
    }

    /// Whole number × length → length, e.g. 4 × one bar. The factor is a
    /// plain count of repetitions, so it carries no unit.
    ///
    /// # Errors
    ///
    /// When the product is too long to count; never wraps around.
    pub fn checked_mul(self, factor: u64) -> Result<ScoreLength, ScoreLengthError> {
        match self.length_ticks.checked_mul(factor) {
            Some(length_ticks) => Ok(ScoreLength { length_ticks }),
            None => Err(ScoreLengthError::ProductTooLong {
                factor,
                length: self,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreLengthError {
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
        left: ScoreLength,
        right: ScoreLength,
    },
    ProductTooLong {
        factor: u64,
        length: ScoreLength,
    },
}

impl fmt::Display for ScoreLengthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScoreLengthError::ZeroDenominator { numerator } => write!(
                f,
                "making a score length from note value {numerator}/0 of a whole note: \
                 the denominator is zero"
            ),
            ScoreLengthError::NotWholeTicks {
                numerator,
                denominator,
            } => write!(
                f,
                "making a score length from note value {numerator}/{denominator} of a whole note: \
                 not a whole number of ticks ({TICKS_PER_QUARTER_NOTE} ticks/q)"
            ),
            ScoreLengthError::NoteValueTooLong {
                numerator,
                denominator,
            } => write!(
                f,
                "making a score length from note value {numerator}/{denominator} of a whole note: \
                 too long to count in ticks"
            ),
            ScoreLengthError::SumTooLong { left, right } => write!(
                f,
                "adding a score length of {} ticks to a score length of {} ticks: \
                 the sum is too long to count",
                right.length_ticks, left.length_ticks
            ),
            ScoreLengthError::ProductTooLong { factor, length } => write!(
                f,
                "multiplying a score length of {} ticks by {factor}: \
                 the product is too long to count",
                length.length_ticks
            ),
        }
    }
}

impl std::error::Error for ScoreLengthError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_value_ticks(numerator: u64, denominator: u64) -> u64 {
        ScoreLength::from_note_value(numerator, denominator)
            .unwrap()
            .ticks()
    }

    #[test]
    fn from_ticks_and_ticks_round_trip() {
        assert_eq!(ScoreLength::from_ticks(2520).ticks(), 2520);
        assert_eq!(ScoreLength::ZERO.ticks(), 0);
    }

    #[test]
    fn plain_note_values() {
        assert_eq!(note_value_ticks(1, 1), 20_160);
        assert_eq!(note_value_ticks(1, 4), 5040);
        assert_eq!(note_value_ticks(1, 8), 2520);
        assert_eq!(note_value_ticks(1, 16), 1260);
        assert_eq!(note_value_ticks(1, 64), 315);
    }

    #[test]
    fn dotted_note_values() {
        assert_eq!(note_value_ticks(3, 8), 7560); // dotted quarter
        assert_eq!(note_value_ticks(3, 16), 3780); // dotted eighth
        assert_eq!(note_value_ticks(7, 16), 8820); // double-dotted quarter
    }

    #[test]
    fn tuplet_note_values_are_exact() {
        assert_eq!(note_value_ticks(1, 12), 1680); // triplet eighth
        assert_eq!(note_value_ticks(1, 20), 1008); // quintuplet sixteenth
        assert_eq!(note_value_ticks(1, 28), 720); // septuplet sixteenth
        // Three triplet quarters fill a half note.
        assert_eq!(note_value_ticks(1, 6) * 3, note_value_ticks(1, 2));
    }

    #[test]
    fn note_value_not_whole_ticks_is_refused() {
        assert_eq!(
            ScoreLength::from_note_value(1, 11),
            Err(ScoreLengthError::NotWholeTicks {
                numerator: 1,
                denominator: 11
            })
        );
        assert_eq!(
            ScoreLength::from_note_value(1, 128),
            Err(ScoreLengthError::NotWholeTicks {
                numerator: 1,
                denominator: 128
            })
        );
    }

    #[test]
    fn note_value_zero_denominator_is_refused() {
        assert_eq!(
            ScoreLength::from_note_value(1, 0),
            Err(ScoreLengthError::ZeroDenominator { numerator: 1 })
        );
    }

    #[test]
    fn note_value_too_long_is_refused() {
        assert_eq!(
            ScoreLength::from_note_value(u64::MAX, 1),
            Err(ScoreLengthError::NoteValueTooLong {
                numerator: u64::MAX,
                denominator: 1
            })
        );
    }

    #[test]
    fn length_plus_length_is_exact() {
        let bar: ScoreLength = ScoreLength::from_note_value(1, 1).unwrap();
        let quarter: ScoreLength = ScoreLength::from_note_value(1, 4).unwrap();
        assert_eq!(bar.checked_add(quarter).unwrap().ticks(), 25_200);
    }

    #[test]
    fn length_plus_length_too_long_is_refused() {
        let longest: ScoreLength = ScoreLength::from_ticks(u64::MAX);
        let one_tick: ScoreLength = ScoreLength::from_ticks(1);
        assert_eq!(
            longest.checked_add(one_tick),
            Err(ScoreLengthError::SumTooLong {
                left: longest,
                right: one_tick
            })
        );
    }

    #[test]
    fn whole_number_times_length_is_exact() {
        let bar: ScoreLength = ScoreLength::from_note_value(1, 1).unwrap();
        assert_eq!(bar.checked_mul(4).unwrap().ticks(), 4 * TICKS_PER_WHOLE_NOTE);
        assert_eq!(bar.checked_mul(0).unwrap(), ScoreLength::ZERO);
    }

    #[test]
    fn whole_number_times_length_too_long_is_refused() {
        let bar: ScoreLength = ScoreLength::from_note_value(1, 1).unwrap();
        assert_eq!(
            bar.checked_mul(u64::MAX),
            Err(ScoreLengthError::ProductTooLong {
                factor: u64::MAX,
                length: bar
            })
        );
    }

    #[test]
    fn error_zero_denominator_says_what_was_attempted() {
        assert_eq!(
            ScoreLengthError::ZeroDenominator { numerator: 1 }.to_string(),
            "making a score length from note value 1/0 of a whole note: the denominator is zero"
        );
    }

    #[test]
    fn error_not_whole_ticks_says_what_was_attempted() {
        assert_eq!(
            ScoreLengthError::NotWholeTicks {
                numerator: 1,
                denominator: 11
            }
            .to_string(),
            "making a score length from note value 1/11 of a whole note: \
             not a whole number of ticks (5040 ticks/q)"
        );
    }

    #[test]
    fn error_note_value_too_long_says_what_was_attempted() {
        assert_eq!(
            ScoreLengthError::NoteValueTooLong {
                numerator: 5,
                denominator: 1
            }
            .to_string(),
            "making a score length from note value 5/1 of a whole note: too long to count in ticks"
        );
    }

    #[test]
    fn error_sum_too_long_says_what_was_attempted() {
        assert_eq!(
            ScoreLengthError::SumTooLong {
                left: ScoreLength::from_ticks(7),
                right: ScoreLength::from_ticks(3)
            }
            .to_string(),
            "adding a score length of 3 ticks to a score length of 7 ticks: \
             the sum is too long to count"
        );
    }

    #[test]
    fn error_product_too_long_says_what_was_attempted() {
        assert_eq!(
            ScoreLengthError::ProductTooLong {
                factor: 4,
                length: ScoreLength::from_ticks(20_160)
            }
            .to_string(),
            "multiplying a score length of 20160 ticks by 4: the product is too long to count"
        );
    }
}
