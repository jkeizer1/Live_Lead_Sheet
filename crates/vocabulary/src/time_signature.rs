//! Time signature n/d: n counts per bar, each count 1/d of a whole note.
//! See docs/Architecture/Musical Time and Clock.md, section 2 ("Count
//! length and bar length"). Beat grouping (5/4 as 3+2, 7/8 as 2+2+3) comes
//! later, with the rhythm line's beaming.

use std::fmt;

use crate::score_length::ScoreLength;
use crate::ticks::TICKS_PER_WHOLE_NOTE;

/// The lower numbers notation uses: a whole, half, quarter, eighth or
/// sixteenth note per count.
const ALLOWED_COUNT_NOTE_VALUES: [u8; 5] = [1, 2, 4, 8, 16];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSignature {
    /// n, the upper number.
    counts_per_bar: u8,
    /// d, the lower number: counts per whole note.
    counts_per_whole_note: u8,
}

impl TimeSignature {
    /// # Errors
    ///
    /// When the upper number is 0, or the lower number is not 1, 2, 4, 8
    /// or 16.
    pub fn new(
        counts_per_bar: u8,
        counts_per_whole_note: u8,
    ) -> Result<TimeSignature, TimeSignatureError> {
        if counts_per_bar == 0 || !ALLOWED_COUNT_NOTE_VALUES.contains(&counts_per_whole_note) {
            return Err(TimeSignatureError::NotAllowed {
                counts_per_bar,
                counts_per_whole_note,
            });
        }
        Ok(TimeSignature {
            counts_per_bar,
            counts_per_whole_note,
        })
    }

    #[must_use]
    pub const fn counts_per_bar(self) -> u8 {
        self.counts_per_bar
    }

    #[must_use]
    pub const fn counts_per_whole_note(self) -> u8 {
        self.counts_per_whole_note
    }

    /// `L_count = 20160 ticks/w ÷ d counts/w` → ticks/count. Exact: d is a
    /// power of 2 up to 16, and 20160 = 2⁶ × 315.
    #[must_use]
    pub const fn count_length(self) -> ScoreLength {
        ScoreLength::from_ticks(TICKS_PER_WHOLE_NOTE / self.counts_per_whole_note as u64)
    }

    /// `L_bar = n counts × L_count ticks/count` → ticks. At most
    /// 255 counts × 20160 ticks/count = 5140800 ticks, so it cannot
    /// overflow a `u64`.
    #[must_use]
    pub const fn bar_length(self) -> ScoreLength {
        ScoreLength::from_ticks(self.counts_per_bar as u64 * self.count_length().ticks())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSignatureError {
    NotAllowed {
        counts_per_bar: u8,
        counts_per_whole_note: u8,
    },
}

impl fmt::Display for TimeSignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeSignatureError::NotAllowed {
                counts_per_bar,
                counts_per_whole_note,
            } => write!(
                f,
                "{counts_per_bar}/{counts_per_whole_note} is not a time signature. \
                 Use 1 or more on top, and 1, 2, 4, 8 or 16 below: e.g. 4/4, 6/8"
            ),
        }
    }
}

impl std::error::Error for TimeSignatureError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn lengths_ticks(counts_per_bar: u8, counts_per_whole_note: u8) -> (u64, u64) {
        let signature: TimeSignature =
            TimeSignature::new(counts_per_bar, counts_per_whole_note).unwrap();
        (
            signature.count_length().ticks(),
            signature.bar_length().ticks(),
        )
    }

    #[test]
    fn count_and_bar_lengths_match_the_reference_table() {
        assert_eq!(lengths_ticks(4, 4), (5040, 20_160));
        assert_eq!(lengths_ticks(3, 4), (5040, 15_120));
        assert_eq!(lengths_ticks(6, 8), (2520, 15_120));
        assert_eq!(lengths_ticks(5, 4), (5040, 25_200));
        assert_eq!(lengths_ticks(7, 8), (2520, 17_640));
        assert_eq!(lengths_ticks(2, 2), (10_080, 20_160));
    }

    #[test]
    fn impossible_signatures_are_refused() {
        assert!(TimeSignature::new(0, 4).is_err());
        assert!(TimeSignature::new(4, 3).is_err());
        assert!(TimeSignature::new(4, 32).is_err());
    }
}
