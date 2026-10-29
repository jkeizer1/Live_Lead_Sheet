//! Score position (P): a place in the song, a whole number of ticks from
//! P = 0 ticks, the first downbeat, counted in performance order.
//!
//! Whole numbers make adding and comparing positions exact, so no rounding
//! drift can build up. See docs/Architecture/Musical Time and Clock.md.

use std::fmt;

use crate::score_length::ScoreLength;

/// A place in the song, in ticks from the first downbeat.
///
/// Only position + length and position − position are defined. Adding two
/// positions is refused by the compiler:
///
/// ```compile_fail,E0369
/// use vocabulary::score_position::ScorePosition;
/// let first: ScorePosition = ScorePosition::from_ticks(0);
/// let second: ScorePosition = ScorePosition::from_ticks(5040);
/// let _sum = first + second;
/// ```
///
/// and so is passing a position where a length is expected:
///
/// ```compile_fail,E0308
/// use vocabulary::score_position::ScorePosition;
/// let first: ScorePosition = ScorePosition::from_ticks(0);
/// let second: ScorePosition = ScorePosition::from_ticks(5040);
/// let _sum = first.checked_add(second);
/// ```
///
/// or a length where a position is expected:
///
/// ```compile_fail,E0308
/// use vocabulary::score_length::ScoreLength;
/// use vocabulary::score_position::ScorePosition;
/// let position: ScorePosition = ScorePosition::from_ticks(5040);
/// let length: ScoreLength = ScoreLength::from_ticks(5040);
/// let _difference = position.checked_sub(length);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScorePosition {
    position_ticks: u64,
}

impl ScorePosition {
    /// P = 0 ticks, the first downbeat of the song.
    pub const ZERO: ScorePosition = ScorePosition { position_ticks: 0 };

    #[must_use]
    pub const fn from_ticks(position_ticks: u64) -> ScorePosition {
        ScorePosition { position_ticks }
    }

    #[must_use]
    pub const fn ticks(self) -> u64 {
        self.position_ticks
    }

    /// Position + length → position.
    ///
    /// # Errors
    ///
    /// When the result is past the last position a `u64` can count; never
    /// wraps around.
    pub fn checked_add(self, length: ScoreLength) -> Result<ScorePosition, ScorePositionError> {
        match self.position_ticks.checked_add(length.ticks()) {
            Some(position_ticks) => Ok(ScorePosition { position_ticks }),
            None => Err(ScorePositionError::PastLastPosition {
                position: self,
                length,
            }),
        }
    }

    /// Position − position → length: the ticks from `earlier` to `self`.
    ///
    /// # Errors
    ///
    /// When `earlier` is in fact later than `self`; a length is never
    /// negative.
    pub fn checked_sub(self, earlier: ScorePosition) -> Result<ScoreLength, ScorePositionError> {
        match self.position_ticks.checked_sub(earlier.position_ticks) {
            Some(length_ticks) => Ok(ScoreLength::from_ticks(length_ticks)),
            None => Err(ScorePositionError::SecondPositionLater {
                first: self,
                second: earlier,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScorePositionError {
    PastLastPosition {
        position: ScorePosition,
        length: ScoreLength,
    },
    SecondPositionLater {
        first: ScorePosition,
        second: ScorePosition,
    },
}

impl fmt::Display for ScorePositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScorePositionError::PastLastPosition { position, length } => write!(
                f,
                "adding a score length of {} ticks to score position {} ticks: \
                 the result is past the last position that can be counted",
                length.ticks(),
                position.position_ticks
            ),
            ScorePositionError::SecondPositionLater { first, second } => write!(
                f,
                "subtracting score position {} ticks from score position {} ticks: \
                 the second is later, and a score length is never negative",
                second.position_ticks, first.position_ticks
            ),
        }
    }
}

impl std::error::Error for ScorePositionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ticks_and_ticks_round_trip() {
        assert_eq!(ScorePosition::from_ticks(40_320).ticks(), 40_320);
        assert_eq!(ScorePosition::ZERO.ticks(), 0);
    }

    #[test]
    fn positions_are_ordered_by_ticks() {
        assert!(ScorePosition::from_ticks(20_160) < ScorePosition::from_ticks(40_320));
    }

    #[test]
    fn position_plus_length_is_exact() {
        let bar: ScoreLength = ScoreLength::from_note_value(1, 1).unwrap();
        let bar_2_start: ScorePosition = ScorePosition::ZERO.checked_add(bar).unwrap();
        let bar_3_start: ScorePosition = bar_2_start.checked_add(bar).unwrap();
        assert_eq!(bar_3_start.ticks(), 40_320);
    }

    #[test]
    fn position_plus_length_past_last_position_is_refused() {
        let last: ScorePosition = ScorePosition::from_ticks(u64::MAX);
        let one_tick: ScoreLength = ScoreLength::from_ticks(1);
        assert_eq!(
            last.checked_add(one_tick),
            Err(ScorePositionError::PastLastPosition {
                position: last,
                length: one_tick
            })
        );
    }

    #[test]
    fn position_minus_earlier_position_is_length() {
        let bar_3_start: ScorePosition = ScorePosition::from_ticks(40_320);
        let bar_4_start: ScorePosition = ScorePosition::from_ticks(60_480);
        assert_eq!(
            bar_4_start.checked_sub(bar_3_start),
            Ok(ScoreLength::from_ticks(20_160))
        );
    }

    #[test]
    fn position_minus_same_position_is_zero_length() {
        let bar_3_start: ScorePosition = ScorePosition::from_ticks(40_320);
        assert_eq!(bar_3_start.checked_sub(bar_3_start), Ok(ScoreLength::ZERO));
    }

    #[test]
    fn position_minus_later_position_is_refused() {
        let bar_3_start: ScorePosition = ScorePosition::from_ticks(40_320);
        let bar_4_start: ScorePosition = ScorePosition::from_ticks(60_480);
        assert_eq!(
            bar_3_start.checked_sub(bar_4_start),
            Err(ScorePositionError::SecondPositionLater {
                first: bar_3_start,
                second: bar_4_start
            })
        );
    }

    #[test]
    fn error_past_last_position_says_what_was_attempted() {
        assert_eq!(
            ScorePositionError::PastLastPosition {
                position: ScorePosition::from_ticks(40_320),
                length: ScoreLength::from_ticks(5040)
            }
            .to_string(),
            "adding a score length of 5040 ticks to score position 40320 ticks: \
             the result is past the last position that can be counted"
        );
    }

    #[test]
    fn error_second_position_later_says_what_was_attempted() {
        assert_eq!(
            ScorePositionError::SecondPositionLater {
                first: ScorePosition::from_ticks(40_320),
                second: ScorePosition::from_ticks(60_480)
            }
            .to_string(),
            "subtracting score position 60480 ticks from score position 40320 ticks: \
             the second is later, and a score length is never negative"
        );
    }
}
