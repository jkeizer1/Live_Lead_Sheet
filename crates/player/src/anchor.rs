//! The anchor (P₀, T₀, B): position P₀ sounded at instant T₀, and from
//! there on the tempo is B. Every conversion between score position and
//! instant starts from an anchor, never from the previous event, so no
//! rounding error can build up (EFS-2.4). See
//! docs/Architecture/Musical Time and Clock.md, sections 2 and 6.

use std::fmt;

use vocabulary::duration::Duration;
use vocabulary::instant::Instant;
use vocabulary::score_length::ScoreLength;
use vocabulary::score_position::ScorePosition;
use vocabulary::tempo::{MILLI_QUARTERS_PER_QUARTER_NOTE, Tempo};
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;

/// 60 s/min × 1000000000 ns/s.
const NANOSECONDS_PER_MINUTE: u128 = 60_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    position: ScorePosition,
    instant: Instant,
    tempo: Tempo,
}

impl Anchor {
    #[must_use]
    pub const fn new(position: ScorePosition, instant: Instant, tempo: Tempo) -> Anchor {
        Anchor {
            position,
            instant,
            tempo,
        }
    }

    #[must_use]
    pub const fn position(self) -> ScorePosition {
        self.position
    }

    #[must_use]
    pub const fn instant(self) -> Instant {
        self.instant
    }

    #[must_use]
    pub const fn tempo(self) -> Tempo {
        self.tempo
    }

    /// Position → instant:
    /// `T = T₀ + (P − P₀) × 60000000000 ns/min × 1000 mq/q ÷ (B × 5040 ticks/q)`.
    /// Units: ticks × ns/min × mq/q ÷ (mq/min × ticks/q) → ns. Multiplied
    /// in 128 bits, divided once, rounded to the nearest ns.
    ///
    /// # Errors
    ///
    /// When `position` is before the anchor, or the instant is past the last
    /// one a `u64` can count.
    pub fn instant_at(self, position: ScorePosition) -> Result<Instant, AnchorError> {
        let Ok(length): Result<ScoreLength, _> = position.checked_sub(self.position) else {
            return Err(AnchorError::PositionBeforeAnchor {
                position,
                anchor: self,
            });
        };
        let numerator: u128 = u128::from(length.ticks())
            * NANOSECONDS_PER_MINUTE
            * u128::from(MILLI_QUARTERS_PER_QUARTER_NOTE);
        let denominator: u128 =
            u128::from(self.tempo.milli_quarters_per_minute()) * u128::from(TICKS_PER_QUARTER_NOTE);
        // Adding half the denominator before dividing rounds to nearest.
        let offset_ns: u128 = (numerator + denominator / 2) / denominator;
        let instant_ns: Option<u64> = u64::try_from(offset_ns)
            .ok()
            .and_then(|offset_ns: u64| self.instant.ns().checked_add(offset_ns));
        match instant_ns {
            Some(instant_ns) => Ok(Instant::from_ns(instant_ns)),
            None => Err(AnchorError::InstantTooLate {
                position,
                anchor: self,
            }),
        }
    }

    /// Instant → position:
    /// `P = P₀ + (T − T₀) × B × 5040 ticks/q ÷ (60000000000 ns/min × 1000 mq/q)`.
    /// Units: ns × mq/min × ticks/q ÷ (ns/min × mq/q) → ticks. Rounded
    /// down, so the playhead never shows a position not yet reached.
    ///
    /// # Errors
    ///
    /// When `instant` is before the anchor, or the position is past the
    /// last one a `u64` can count.
    pub fn position_at(self, instant: Instant) -> Result<ScorePosition, AnchorError> {
        let Ok(duration): Result<Duration, _> = instant.checked_sub(self.instant) else {
            return Err(AnchorError::InstantBeforeAnchor {
                instant,
                anchor: self,
            });
        };
        let numerator: u128 = u128::from(duration.ns())
            * u128::from(self.tempo.milli_quarters_per_minute())
            * u128::from(TICKS_PER_QUARTER_NOTE);
        let denominator: u128 =
            NANOSECONDS_PER_MINUTE * u128::from(MILLI_QUARTERS_PER_QUARTER_NOTE);
        let offset_ticks: u128 = numerator / denominator;
        let position_ticks: Option<u64> = u64::try_from(offset_ticks)
            .ok()
            .and_then(|offset_ticks: u64| self.position.ticks().checked_add(offset_ticks));
        match position_ticks {
            Some(position_ticks) => Ok(ScorePosition::from_ticks(position_ticks)),
            None => Err(AnchorError::PositionTooLate {
                instant,
                anchor: self,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorError {
    PositionBeforeAnchor {
        position: ScorePosition,
        anchor: Anchor,
    },
    InstantBeforeAnchor {
        instant: Instant,
        anchor: Anchor,
    },
    InstantTooLate {
        position: ScorePosition,
        anchor: Anchor,
    },
    PositionTooLate {
        instant: Instant,
        anchor: Anchor,
    },
}

impl fmt::Display for AnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnchorError::PositionBeforeAnchor { position, anchor } => write!(
                f,
                "finding the instant of score position {} ticks: it is before the anchor \
                 at {} ticks",
                position.ticks(),
                anchor.position.ticks()
            ),
            AnchorError::InstantBeforeAnchor { instant, anchor } => write!(
                f,
                "finding the score position at instant {} ns: it is before the anchor \
                 at {} ns",
                instant.ns(),
                anchor.instant.ns()
            ),
            AnchorError::InstantTooLate { position, anchor } => write!(
                f,
                "finding the instant of score position {} ticks from the anchor at {} ticks, \
                 {} ns: past the last instant that can be counted",
                position.ticks(),
                anchor.position.ticks(),
                anchor.instant.ns()
            ),
            AnchorError::PositionTooLate { instant, anchor } => write!(
                f,
                "finding the score position at instant {} ns from the anchor at {} ticks, \
                 {} ns: past the last position that can be counted",
                instant.ns(),
                anchor.position.ticks(),
                anchor.instant.ns()
            ),
        }
    }
}

impl std::error::Error for AnchorError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempo(quarter_notes_per_minute: u64) -> Tempo {
        Tempo::from_quarter_notes_per_minute(quarter_notes_per_minute).unwrap()
    }

    /// The worked examples start at T₀ = 10000000000 ns.
    fn song_start(quarter_notes_per_minute: u64) -> Anchor {
        Anchor::new(
            ScorePosition::ZERO,
            Instant::from_ns(10_000_000_000),
            tempo(quarter_notes_per_minute),
        )
    }

    #[test]
    fn example_1_bar_2_at_120_quarter_notes_per_minute() {
        assert_eq!(
            song_start(120).instant_at(ScorePosition::from_ticks(20_160)),
            Ok(Instant::from_ns(12_000_000_000))
        );
    }

    #[test]
    fn example_2_tempo_change_rounds_once_to_the_nearest_ns() {
        let bar_3: Anchor = Anchor::new(
            ScorePosition::from_ticks(40_320),
            Instant::from_ns(14_000_000_000),
            tempo(140),
        );
        assert_eq!(
            bar_3.instant_at(ScorePosition::from_ticks(60_480)),
            Ok(Instant::from_ns(15_714_285_714))
        );
    }

    #[test]
    fn example_4_slider_at_90_percent() {
        let anchor: Anchor = Anchor::new(
            ScorePosition::ZERO,
            Instant::from_ns(0),
            Tempo::from_milli_quarters_per_minute(108_000).unwrap(),
        );
        assert_eq!(
            anchor.instant_at(ScorePosition::from_ticks(20_160)),
            Ok(Instant::from_ns(2_222_222_222))
        );
    }

    #[test]
    fn example_5_one_clock_pulse() {
        let anchor: Anchor = Anchor::new(ScorePosition::ZERO, Instant::from_ns(0), tempo(120));
        assert_eq!(
            anchor.instant_at(ScorePosition::from_ticks(210)),
            Ok(Instant::from_ns(20_833_333))
        );
    }

    #[test]
    fn position_at_rounds_down() {
        let anchor: Anchor = song_start(120);
        // 1 ns before bar 2 is still in bar 1.
        assert_eq!(
            anchor.position_at(Instant::from_ns(11_999_999_999)),
            Ok(ScorePosition::from_ticks(20_159))
        );
        assert_eq!(
            anchor.position_at(Instant::from_ns(12_000_000_000)),
            Ok(ScorePosition::from_ticks(20_160))
        );
    }

    #[test]
    fn before_the_anchor_is_refused() {
        let anchor: Anchor = Anchor::new(
            ScorePosition::from_ticks(5040),
            Instant::from_ns(1_000),
            tempo(120),
        );
        assert!(anchor.instant_at(ScorePosition::ZERO).is_err());
        assert!(anchor.position_at(Instant::from_ns(0)).is_err());
    }
}
