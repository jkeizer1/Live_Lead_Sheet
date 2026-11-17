//! The meter of each bar of a section (Song Notation, section 5, "Meter
//! Control"): one meter for every bar, or a pattern of meters that repeats
//! to the end of the section. Every meter of a pattern has the same beat, so
//! the tempo in beats per minute is the same in every bar.

use std::fmt;

use crate::time_signature::TimeSignature;

/// The meters of the bars of a section. The pattern repeats; one meter is a
/// pattern of one bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarMeters {
    /// Never empty.
    pattern: Vec<TimeSignature>,
}

impl BarMeters {
    /// Every bar in `time_signature`.
    #[must_use]
    pub fn uniform(time_signature: TimeSignature) -> BarMeters {
        BarMeters {
            pattern: vec![time_signature],
        }
    }

    /// A pattern of one or more bars that repeats.
    ///
    /// # Errors
    ///
    /// When the pattern is empty, or its meters do not all have the same
    /// beat: the same lower figure, and a compound meter (6/8, 9/8, 12/8) is
    /// not mixed with one that is not.
    pub fn pattern(pattern: Vec<TimeSignature>) -> Result<BarMeters, BarMetersError> {
        let Some(first) = pattern.first().copied() else {
            return Err(BarMetersError::Empty);
        };
        for &other in &pattern[1..] {
            let same_beat: bool = other.pulses_per_whole_note() == first.pulses_per_whole_note()
                && other.beat_length() == first.beat_length();
            if !same_beat {
                return Err(BarMetersError::DifferentBeat { first, other });
            }
        }
        Ok(BarMeters { pattern })
    }

    /// The meter of the first bar, which gives the beat of the whole pattern.
    #[must_use]
    pub fn first(&self) -> TimeSignature {
        self.pattern[0]
    }

    /// The number of bars in one play of the pattern.
    #[must_use]
    pub fn pattern_bar_count(&self) -> usize {
        self.pattern.len()
    }

    /// The pattern as written, one meter per bar.
    #[must_use]
    pub fn pattern_meters(&self) -> &[TimeSignature] {
        &self.pattern
    }

    #[must_use]
    pub fn is_uniform(&self) -> bool {
        self.pattern.len() == 1
    }

    /// The meter of the bar at `bar_index`, counted from 0 along the section.
    #[must_use]
    pub fn meter_of_bar(&self, bar_index: usize) -> TimeSignature {
        self.pattern[bar_index % self.pattern.len()]
    }

    /// Ticks from the start of the section to the start of the bar at
    /// `bar_index`.
    #[must_use]
    pub fn bar_start_ticks(&self, bar_index: usize) -> u64 {
        let plays: u64 = (bar_index / self.pattern.len()) as u64;
        let into_pattern: usize = bar_index % self.pattern.len();
        // plays × ticks/play + the ticks of the bars played so far in this play.
        plays * self.pattern_length_ticks() + self.ticks_of_first_bars(into_pattern)
    }

    /// The length of the bar at `bar_index`, in ticks.
    #[must_use]
    pub fn bar_length_ticks(&self, bar_index: usize) -> u64 {
        self.meter_of_bar(bar_index).bar_length().ticks()
    }

    /// The length of the first `bar_count` bars of the section, in ticks.
    #[must_use]
    pub fn length_ticks(&self, bar_count: u32) -> u64 {
        self.bar_start_ticks(bar_count as usize)
    }

    /// The bar a place falls in, counted from 0, and the ticks into that
    /// bar. `place_ticks` counts from the start of the section.
    #[must_use]
    pub fn bar_at(&self, place_ticks: u64) -> (usize, u64) {
        let pattern_ticks: u64 = self.pattern_length_ticks();
        let plays: u64 = place_ticks / pattern_ticks;
        let mut remaining_ticks: u64 = place_ticks % pattern_ticks;
        let mut bar_in_pattern: usize = 0;
        for meter in &self.pattern {
            let bar_ticks: u64 = meter.bar_length().ticks();
            if remaining_ticks < bar_ticks {
                break;
            }
            remaining_ticks -= bar_ticks;
            bar_in_pattern += 1;
        }
        let bar_index: u64 = plays * self.pattern.len() as u64 + bar_in_pattern as u64;
        (
            usize::try_from(bar_index).unwrap_or(usize::MAX),
            remaining_ticks,
        )
    }

    fn pattern_length_ticks(&self) -> u64 {
        self.ticks_of_first_bars(self.pattern.len())
    }

    fn ticks_of_first_bars(&self, bar_count: usize) -> u64 {
        self.pattern[..bar_count]
            .iter()
            .map(|meter: &TimeSignature| meter.bar_length().ticks())
            .sum()
    }
}

/// One meter for every bar.
impl From<TimeSignature> for BarMeters {
    fn from(time_signature: TimeSignature) -> BarMeters {
        BarMeters::uniform(time_signature)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarMetersError {
    Empty,
    DifferentBeat {
        first: TimeSignature,
        other: TimeSignature,
    },
}

impl fmt::Display for BarMetersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BarMetersError::Empty => {
                write!(
                    f,
                    "a meter pattern needs at least one meter, e.g. mp [3_4 5_4]"
                )
            }
            BarMetersError::DifferentBeat { first, other } => write!(
                f,
                "{}/{} and {}/{} do not have the same beat. Every meter of a pattern must have \
                 the same lower figure, and a compound meter (6_8, 9_8, 12_8) is not mixed with \
                 one that is not",
                first.pulses_per_bar(),
                first.pulses_per_whole_note(),
                other.pulses_per_bar(),
                other.pulses_per_whole_note()
            ),
        }
    }
}

impl std::error::Error for BarMetersError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn meter(pulses_per_bar: u8, pulses_per_whole_note: u8) -> TimeSignature {
        TimeSignature::new(pulses_per_bar, pulses_per_whole_note).unwrap()
    }

    #[test]
    fn one_meter_is_the_same_in_every_bar() {
        let meters: BarMeters = BarMeters::uniform(meter(4, 4));
        assert!(meters.is_uniform());
        assert_eq!(meters.meter_of_bar(7), meter(4, 4));
        assert_eq!(meters.bar_start_ticks(3), 3 * 20_160);
    }

    #[test]
    fn a_pattern_repeats_along_the_section() {
        let meters: BarMeters = BarMeters::pattern(vec![meter(3, 4), meter(5, 4)]).unwrap();
        assert_eq!(meters.meter_of_bar(0), meter(3, 4));
        assert_eq!(meters.meter_of_bar(1), meter(5, 4));
        assert_eq!(meters.meter_of_bar(2), meter(3, 4));
        // 3/4 is 15120 ticks and 5/4 is 25200 ticks.
        assert_eq!(meters.bar_start_ticks(1), 15_120);
        assert_eq!(meters.bar_start_ticks(2), 40_320);
        assert_eq!(meters.bar_start_ticks(3), 55_440);
        assert_eq!(meters.length_ticks(4), 80_640);
    }

    #[test]
    fn a_place_is_found_in_its_bar() {
        let meters: BarMeters = BarMeters::pattern(vec![meter(3, 4), meter(5, 4)]).unwrap();
        assert_eq!(meters.bar_at(0), (0, 0));
        assert_eq!(meters.bar_at(15_119), (0, 15_119));
        assert_eq!(meters.bar_at(15_120), (1, 0));
        assert_eq!(meters.bar_at(40_320), (2, 0));
        assert_eq!(meters.bar_at(40_320 + 15_120 + 5040), (3, 5040));
    }

    #[test]
    fn meters_with_one_beat_make_a_pattern() {
        assert!(BarMeters::pattern(vec![meter(6, 8), meter(9, 8), meter(12, 8)]).is_ok());
        assert!(BarMeters::pattern(vec![meter(3, 4), meter(5, 4), meter(4, 4)]).is_ok());
    }

    #[test]
    fn meters_with_different_beats_are_refused() {
        // 3/4 has a quarter-note beat and 6/8 a dotted-quarter beat.
        assert!(matches!(
            BarMeters::pattern(vec![meter(3, 4), meter(6, 8)]).unwrap_err(),
            BarMetersError::DifferentBeat { .. }
        ));
        // 6/8 is compound and 5/8 is not.
        assert!(matches!(
            BarMeters::pattern(vec![meter(6, 8), meter(5, 8)]).unwrap_err(),
            BarMetersError::DifferentBeat { .. }
        ));
    }

    #[test]
    fn an_empty_pattern_is_refused() {
        assert_eq!(
            BarMeters::pattern(Vec::new()).unwrap_err(),
            BarMetersError::Empty
        );
    }
}
