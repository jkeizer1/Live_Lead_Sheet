//! Tempo (B): quarter notes per minute, held as a whole number of
//! milli-quarters per minute (mq/min), so slider settings such as
//! 112.44 q/min (112440 mq/min) stay whole numbers. See
//! docs/Architecture/Musical Time and Clock.md, sections 0 and 2.

use std::fmt;

/// 1 q = 1000 mq.
pub const MILLI_QUARTERS_PER_QUARTER_NOTE: u64 = 1000;

/// The conversion rate between score positions and instants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tempo {
    milli_quarters_per_minute: u64,
}

impl Tempo {
    /// # Errors
    ///
    /// When the tempo is zero: no score position would ever be reached.
    pub fn from_milli_quarters_per_minute(
        milli_quarters_per_minute: u64,
    ) -> Result<Tempo, TempoError> {
        if milli_quarters_per_minute == 0 {
            return Err(TempoError::Zero);
        }
        Ok(Tempo {
            milli_quarters_per_minute,
        })
    }

    /// q/min × 1000 mq/q → mq/min.
    ///
    /// # Errors
    ///
    /// When the tempo is zero, or too large to count in mq/min.
    pub fn from_quarter_notes_per_minute(
        quarter_notes_per_minute: u64,
    ) -> Result<Tempo, TempoError> {
        let Some(milli_quarters_per_minute): Option<u64> =
            quarter_notes_per_minute.checked_mul(MILLI_QUARTERS_PER_QUARTER_NOTE)
        else {
            return Err(TempoError::TooFast {
                quarter_notes_per_minute,
            });
        };
        Tempo::from_milli_quarters_per_minute(milli_quarters_per_minute)
    }

    #[must_use]
    pub const fn milli_quarters_per_minute(self) -> u64 {
        self.milli_quarters_per_minute
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempoError {
    Zero,
    TooFast { quarter_notes_per_minute: u64 },
}

impl fmt::Display for TempoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TempoError::Zero => write!(f, "a tempo of 0 q/min: nothing would ever play"),
            TempoError::TooFast {
                quarter_notes_per_minute,
            } => write!(
                f,
                "a tempo of {quarter_notes_per_minute} q/min is too large to count in mq/min"
            ),
        }
    }
}

impl std::error::Error for TempoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_notes_per_minute_become_milli_quarters_per_minute() {
        assert_eq!(
            Tempo::from_quarter_notes_per_minute(120)
                .unwrap()
                .milli_quarters_per_minute(),
            120_000
        );
    }

    #[test]
    fn zero_tempo_is_refused() {
        assert_eq!(
            Tempo::from_quarter_notes_per_minute(0),
            Err(TempoError::Zero)
        );
        assert_eq!(
            Tempo::from_milli_quarters_per_minute(0),
            Err(TempoError::Zero)
        );
    }

    #[test]
    fn tempo_too_large_is_refused() {
        assert_eq!(
            Tempo::from_quarter_notes_per_minute(u64::MAX),
            Err(TempoError::TooFast {
                quarter_notes_per_minute: u64::MAX
            })
        );
    }
}
