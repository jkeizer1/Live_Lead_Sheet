//! The counter (EFS-1.3): where a score position is, in the musician's
//! terms — bar and beat, with the count-in section named (EFS-2.7). The
//! beat is the meter's: one lower-figure note in simple time, a dotted note
//! (3 of them) in compound time, so 6/8 counts beats 1–2 (Jim, 7 Nov 2026).
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

/// Bars and beats are numbered from 1, as musicians count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counter {
    pub section: CounterSection,
    pub bar: u64,
    pub beat: u64,
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
        let beat_length_ticks: u64 = time_signature.beat_length().ticks();
        // ticks ÷ ticks/bar → bars since 0 ticks; the rest is into the bar.
        let bars_since_start: u64 = position.ticks() / bar_length_ticks;
        let into_bar_ticks: u64 = position.ticks() % bar_length_ticks;
        // ticks ÷ ticks/beat → whole beats into the bar; beat 1 is the first.
        let beat: u64 = into_bar_ticks / beat_length_ticks + 1;
        if bars_since_start < count_in_bars {
            Counter {
                section: CounterSection::CountIn,
                bar: bars_since_start + 1,
                beat,
            }
        } else {
            Counter {
                section: CounterSection::Music,
                bar: bars_since_start - count_in_bars + 1,
                beat,
            }
        }
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.section {
            CounterSection::CountIn => {
                write!(f, "Count-in   Bar: {}   Beat: {}", self.bar, self.beat)
            }
            CounterSection::Music => {
                write!(f, "Bar: {}   Beat: {}", self.bar, self.beat)
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
            "Count-in   Bar: 1   Beat: 4"
        );
        assert_eq!(
            Counter::at(ScorePosition::from_ticks(20_160), four_four(), 1).to_string(),
            "Bar: 1   Beat: 1"
        );
    }

    fn beat_at(position_ticks: u64, pulses_per_bar: u8, pulses_per_whole_note: u8) -> u64 {
        let time_signature: TimeSignature =
            TimeSignature::new(pulses_per_bar, pulses_per_whole_note).unwrap();
        Counter::at(ScorePosition::from_ticks(position_ticks), time_signature, 0).beat
    }

    #[test]
    fn six_eight_counts_two_dotted_quarter_beats() {
        // A 6/8 beat is a dotted quarter: 3 eighths × 2520 ticks = 7560 ticks.
        // 2 eighths in (5040 ticks) is still beat 1; 3 eighths in, beat 2.
        assert_eq!(beat_at(5040, 6, 8), 1);
        assert_eq!(beat_at(7560, 6, 8), 2);
        // 5 eighths in (12600 ticks): beat 2, in the same bar.
        assert_eq!(
            Counter::at(
                ScorePosition::from_ticks(12_600),
                TimeSignature::new(6, 8).unwrap(),
                0
            ),
            Counter {
                section: CounterSection::Music,
                bar: 1,
                beat: 2
            }
        );
    }

    #[test]
    fn compound_meters_count_their_beats_and_simple_meters_their_lower_figure() {
        // The last eighth of a 9/8 and a 12/8 bar: 8 and 11 eighths in.
        assert_eq!(beat_at(8 * 2520, 9, 8), 3);
        assert_eq!(beat_at(11 * 2520, 12, 8), 4);
        // 3/8 and 5/8 are simple: one beat per eighth.
        assert_eq!(beat_at(2 * 2520, 3, 8), 3);
        assert_eq!(beat_at(4 * 2520, 5, 8), 5);
        // 2/2: one beat per half note (10080 ticks).
        assert_eq!(beat_at(10_080, 2, 2), 2);
    }
}
