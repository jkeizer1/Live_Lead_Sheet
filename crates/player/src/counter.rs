//! The counter (EFS-1.3): where a score position is, in the musician's
//! terms — bar and count, with the count-in section named (EFS-2.7).
//!
//! This first version takes one time signature and a count-in length in
//! whole bars; sections and repeats come with the song.

use std::fmt;

use vocabulary::score_position::ScorePosition;
use vocabulary::time_signature::TimeSignature;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterSection {
    CountIn,
    Music,
}

/// Bars and counts are numbered from 1, as musicians count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counter {
    pub section: CounterSection,
    pub bar: u64,
    pub count: u64,
}

impl Counter {
    /// The counter at `position`, when the song opens with
    /// `count_in_bars` bars of count-in.
    #[must_use]
    pub fn at(
        position: ScorePosition,
        time_signature: TimeSignature,
        count_in_bars: u64,
    ) -> Counter {
        let bar_length_ticks: u64 = time_signature.bar_length().ticks();
        let count_length_ticks: u64 = time_signature.count_length().ticks();
        // ticks ÷ ticks/bar → bars since 0 ticks; the rest is into the bar.
        let bars_since_start: u64 = position.ticks() / bar_length_ticks;
        let into_bar_ticks: u64 = position.ticks() % bar_length_ticks;
        let count: u64 = into_bar_ticks / count_length_ticks + 1;
        if bars_since_start < count_in_bars {
            Counter {
                section: CounterSection::CountIn,
                bar: bars_since_start + 1,
                count,
            }
        } else {
            Counter {
                section: CounterSection::Music,
                bar: bars_since_start - count_in_bars + 1,
                count,
            }
        }
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.section {
            CounterSection::CountIn => {
                write!(f, "Count-in   Bar: {}   Count: {}", self.bar, self.count)
            }
            CounterSection::Music => {
                write!(f, "Bar: {}   Count: {}", self.bar, self.count)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn four_four() -> TimeSignature {
        TimeSignature::new(4, 4).unwrap()
    }

    #[test]
    fn count_in_bars_are_named_and_the_music_starts_at_bar_1() {
        assert_eq!(
            Counter::at(ScorePosition::from_ticks(15_120), four_four(), 1).to_string(),
            "Count-in   Bar: 1   Count: 4"
        );
        assert_eq!(
            Counter::at(ScorePosition::from_ticks(20_160), four_four(), 1).to_string(),
            "Bar: 1   Count: 1"
        );
    }

    #[test]
    fn six_eight_counts_eighths() {
        let six_eight: TimeSignature = TimeSignature::new(6, 8).unwrap();
        // 5 eighths into the first bar: 5 × 2520 ticks.
        assert_eq!(
            Counter::at(ScorePosition::from_ticks(12_600), six_eight, 0),
            Counter {
                section: CounterSection::Music,
                bar: 1,
                count: 6
            }
        );
    }
}
