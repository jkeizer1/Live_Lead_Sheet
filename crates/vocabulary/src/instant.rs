//! Instant (T): a reading of the clock, a whole number of nanoseconds.
//!
//! The clock is the ALSA queue's real time (D-8); its absolute value has no
//! meaning, so instants are only compared and subtracted. See
//! docs/Architecture/Musical Time and Clock.md, sections 2 and 5.

use std::fmt;

use crate::duration::Duration;

/// A clock reading, in nanoseconds.
///
/// Only instant + duration and instant − instant are defined; adding two
/// instants is refused by the compiler:
///
/// ```compile_fail,E0369
/// use vocabulary::instant::Instant;
/// let first: Instant = Instant::from_ns(1);
/// let second: Instant = Instant::from_ns(2);
/// let _sum = first + second;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    instant_ns: u64,
}

impl Instant {
    #[must_use]
    pub const fn from_ns(instant_ns: u64) -> Instant {
        Instant { instant_ns }
    }

    #[must_use]
    pub const fn ns(self) -> u64 {
        self.instant_ns
    }

    /// Instant + duration → instant.
    ///
    /// # Errors
    ///
    /// When the result is past the last instant a `u64` can count; never
    /// wraps around.
    pub fn checked_add(self, duration: Duration) -> Result<Instant, InstantError> {
        match self.instant_ns.checked_add(duration.ns()) {
            Some(instant_ns) => Ok(Instant { instant_ns }),
            None => Err(InstantError::PastLastInstant {
                instant: self,
                duration,
            }),
        }
    }

    /// Instant − instant → duration: the time from `earlier` to `self`.
    ///
    /// # Errors
    ///
    /// When `earlier` is in fact later than `self`; a duration is never
    /// negative.
    pub fn checked_sub(self, earlier: Instant) -> Result<Duration, InstantError> {
        match self.instant_ns.checked_sub(earlier.instant_ns) {
            Some(duration_ns) => Ok(Duration::from_ns(duration_ns)),
            None => Err(InstantError::SecondInstantLater {
                first: self,
                second: earlier,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstantError {
    PastLastInstant { instant: Instant, duration: Duration },
    SecondInstantLater { first: Instant, second: Instant },
}

impl fmt::Display for InstantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstantError::PastLastInstant { instant, duration } => write!(
                f,
                "adding a duration of {} ns to instant {} ns: \
                 the result is past the last instant that can be counted",
                duration.ns(),
                instant.instant_ns
            ),
            InstantError::SecondInstantLater { first, second } => write!(
                f,
                "subtracting instant {} ns from instant {} ns: \
                 the second is later, and a duration is never negative",
                second.instant_ns, first.instant_ns
            ),
        }
    }
}

impl std::error::Error for InstantError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_plus_duration_is_exact() {
        let start: Instant = Instant::from_ns(10_000_000_000);
        assert_eq!(
            start.checked_add(Duration::from_ns(2_000_000_000)),
            Ok(Instant::from_ns(12_000_000_000))
        );
    }

    #[test]
    fn instant_plus_duration_past_last_instant_is_refused() {
        let last: Instant = Instant::from_ns(u64::MAX);
        let one_ns: Duration = Duration::from_ns(1);
        assert_eq!(
            last.checked_add(one_ns),
            Err(InstantError::PastLastInstant {
                instant: last,
                duration: one_ns
            })
        );
    }

    #[test]
    fn instant_minus_earlier_instant_is_duration() {
        assert_eq!(
            Instant::from_ns(12_000_000_000).checked_sub(Instant::from_ns(10_000_000_000)),
            Ok(Duration::from_ns(2_000_000_000))
        );
    }

    #[test]
    fn instant_minus_later_instant_is_refused() {
        let early: Instant = Instant::from_ns(1);
        let late: Instant = Instant::from_ns(2);
        assert_eq!(
            early.checked_sub(late),
            Err(InstantError::SecondInstantLater {
                first: early,
                second: late
            })
        );
    }

    #[test]
    fn error_second_instant_later_says_what_was_attempted() {
        assert_eq!(
            InstantError::SecondInstantLater {
                first: Instant::from_ns(1),
                second: Instant::from_ns(2)
            }
            .to_string(),
            "subtracting instant 2 ns from instant 1 ns: \
             the second is later, and a duration is never negative"
        );
    }
}
