//! Tempo (B): quarter notes per minute, held as a whole number of
//! milli-quarters per minute (mq/min), so slider settings such as
//! 112.44 q/min (112440 mq/min) stay whole numbers. A song writes its tempo
//! in beats per minute or per second; the meter says how long a beat is.
//! See docs/Architecture/Musical Time and Clock.md, sections 0, 2 and 13.

use std::fmt;

use crate::score_length::ScoreLength;
use crate::ticks::TICKS_PER_QUARTER_NOTE;

/// 1 q = 1000 mq.
pub const MILLI_QUARTERS_PER_QUARTER_NOTE: u64 = 1000;

const SECONDS_PER_MINUTE: u64 = 60;

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

    /// `B = beats/min × L_beat ticks/beat × 1000 mq/q ÷ 5040 ticks/q`
    /// → mq/min. Exact for every meter: the shortest beat, a sixteenth
    /// (1260 ticks), gives 250 mq/min per beat/min.
    ///
    /// # Errors
    ///
    /// When the tempo is zero, too large to count in mq/min, or the beat is
    /// not a whole number of mq/min per beat/min.
    pub fn from_beats_per_minute(
        beats_per_minute: u64,
        beat_length: ScoreLength,
    ) -> Result<Tempo, TempoError> {
        let too_fast = || TempoError::BeatsTooFast { beats_per_minute };
        // beats/min × ticks/beat × mq/q → milli-quarter ticks per minute.
        let milli_quarter_ticks_per_minute: u64 = beats_per_minute
            .checked_mul(beat_length.ticks())
            .ok_or_else(too_fast)?
            .checked_mul(MILLI_QUARTERS_PER_QUARTER_NOTE)
            .ok_or_else(too_fast)?;
        if !milli_quarter_ticks_per_minute.is_multiple_of(TICKS_PER_QUARTER_NOTE) {
            return Err(TempoError::BeatNotWholeMilliQuarters {
                beat_length_ticks: beat_length.ticks(),
            });
        }
        // (mq × ticks/min) ÷ ticks/q, with 1 q per quarter: mq/min.
        Tempo::from_milli_quarters_per_minute(
            milli_quarter_ticks_per_minute / TICKS_PER_QUARTER_NOTE,
        )
    }

    /// `beats/s × 60 s/min` → beats/min, then as `from_beats_per_minute`.
    ///
    /// # Errors
    ///
    /// As `from_beats_per_minute`.
    pub fn from_beats_per_second(
        beats_per_second: u64,
        beat_length: ScoreLength,
    ) -> Result<Tempo, TempoError> {
        let Some(beats_per_minute): Option<u64> = beats_per_second.checked_mul(SECONDS_PER_MINUTE)
        else {
            return Err(TempoError::BeatsPerSecondTooFast { beats_per_second });
        };
        Tempo::from_beats_per_minute(beats_per_minute, beat_length)
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
    BeatsTooFast { beats_per_minute: u64 },
    BeatsPerSecondTooFast { beats_per_second: u64 },
    BeatNotWholeMilliQuarters { beat_length_ticks: u64 },
}

impl fmt::Display for TempoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TempoError::Zero => write!(f, "a tempo of 0: nothing would ever play"),
            TempoError::TooFast {
                quarter_notes_per_minute,
            } => write!(
                f,
                "a tempo of {quarter_notes_per_minute} q/min is too large to count in mq/min"
            ),
            TempoError::BeatsTooFast { beats_per_minute } => write!(
                f,
                "a tempo of {beats_per_minute} beats/min is too large to count in mq/min"
            ),
            TempoError::BeatsPerSecondTooFast { beats_per_second } => write!(
                f,
                "a tempo of {beats_per_second} beats/s is too large to count in mq/min"
            ),
            TempoError::BeatNotWholeMilliQuarters { beat_length_ticks } => write!(
                f,
                "a beat of {beat_length_ticks} ticks is not a whole number of mq/min per \
                 beat/min"
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

    fn beat(length_ticks: u64) -> ScoreLength {
        ScoreLength::from_ticks(length_ticks)
    }

    fn per_minute(beats_per_minute: u64, beat_length_ticks: u64) -> u64 {
        Tempo::from_beats_per_minute(beats_per_minute, beat(beat_length_ticks))
            .unwrap()
            .milli_quarters_per_minute()
    }

    /// The table in Musical Time and Clock, section 13.
    #[test]
    fn beats_per_minute_match_the_reference_table() {
        assert_eq!(per_minute(120, 5040), 120_000); // 4/4
        assert_eq!(per_minute(30, 2520), 15_000); // 3/8
        assert_eq!(per_minute(60, 10_080), 120_000); // 2/2
        assert_eq!(per_minute(100, 5040), 100_000); // 5/4
        assert_eq!(per_minute(60, 7560), 90_000); // 6/8, 9/8, 12/8
        assert_eq!(per_minute(80, 3780), 60_000); // 9/16
        // The shortest beat, a sixteenth: 250 mq/min per beat/min.
        assert_eq!(per_minute(1, 1260), 250);
    }

    #[test]
    fn beats_per_second_are_sixty_times_beats_per_minute() {
        assert_eq!(
            Tempo::from_beats_per_second(2, beat(7560))
                .unwrap()
                .milli_quarters_per_minute(),
            180_000
        );
    }

    #[test]
    fn beat_tempos_that_cannot_be_counted_are_refused() {
        assert_eq!(
            Tempo::from_beats_per_minute(0, beat(5040)),
            Err(TempoError::Zero)
        );
        assert_eq!(
            Tempo::from_beats_per_minute(u64::MAX, beat(5040)),
            Err(TempoError::BeatsTooFast {
                beats_per_minute: u64::MAX
            })
        );
        assert_eq!(
            Tempo::from_beats_per_second(u64::MAX, beat(5040)),
            Err(TempoError::BeatsPerSecondTooFast {
                beats_per_second: u64::MAX
            })
        );
        // 1 tick: 1 beat/min × 1 tick × 1000 mq/q ÷ 5040 ticks/q is not whole.
        assert_eq!(
            Tempo::from_beats_per_minute(1, beat(1)),
            Err(TempoError::BeatNotWholeMilliQuarters {
                beat_length_ticks: 1
            })
        );
    }
}
