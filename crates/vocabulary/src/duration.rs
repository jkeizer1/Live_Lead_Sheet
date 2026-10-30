//! Duration (ΔT): how long something takes, a whole number of nanoseconds.
//!
//! A clock quantity, the difference of two instants. Score lengths (ticks)
//! are a different kind of quantity and do not mix with it. See
//! docs/Architecture/Musical Time and Clock.md, sections 1 and 2.

/// A span of real time, in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    duration_ns: u64,
}

impl Duration {
    pub const ZERO: Duration = Duration { duration_ns: 0 };

    #[must_use]
    pub const fn from_ns(duration_ns: u64) -> Duration {
        Duration { duration_ns }
    }

    /// 1 ms = 1000000 ns. Saturates rather than wraps; a duration this long
    /// (over 584 years) is not a musical one, and the caller's value is a
    /// literal, not computed.
    #[must_use]
    pub const fn from_ms(duration_ms: u64) -> Duration {
        Duration {
            duration_ns: duration_ms.saturating_mul(NANOSECONDS_PER_MILLISECOND),
        }
    }

    #[must_use]
    pub const fn ns(self) -> u64 {
        self.duration_ns
    }
}

pub const NANOSECONDS_PER_MILLISECOND: u64 = 1_000_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ms_is_whole_nanoseconds() {
        assert_eq!(Duration::from_ms(50).ns(), 50_000_000);
    }

    #[test]
    fn from_ns_and_ns_round_trip() {
        assert_eq!(Duration::from_ns(20_833_333).ns(), 20_833_333);
    }
}
