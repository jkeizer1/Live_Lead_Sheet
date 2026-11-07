//! Time signature n/d: n pulses per bar, each pulse 1/d of a whole note.
//! The beat is 1 pulse in simple time and 3 pulses in compound time (upper
//! figure 6, 9 or 12). See docs/Architecture/Musical Time and Clock.md,
//! section 2 ("Pulse, beat and bar"). Beat grouping (5/4 as 3+2, 7/8 as 2+2+3) comes later, with the
//! rhythm line's beaming.

use std::fmt;

use crate::score_length::ScoreLength;
use crate::ticks::TICKS_PER_WHOLE_NOTE;

/// The lower numbers notation uses: a whole, half, quarter, eighth or
/// sixteenth note per pulse.
const ALLOWED_PULSE_NOTE_VALUES: [u8; 5] = [1, 2, 4, 8, 16];

/// The upper figures of compound time in the rudiments: duple, triple and
/// quadruple.
const COMPOUND_PULSES_PER_BAR: [u8; 3] = [6, 9, 12];

/// In compound time a beat is a dotted note: 3 pulses.
const PULSES_PER_COMPOUND_BEAT: u64 = 3;

/// The largest compound upper figure; a larger multiple of 3 (15/8, 18/8)
/// is outside the rudiments and refused until its beats are decided.
const LARGEST_COMPOUND_PULSES_PER_BAR: u8 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSignature {
    /// n, the upper number.
    pulses_per_bar: u8,
    /// d, the lower number: pulses per whole note.
    pulses_per_whole_note: u8,
}

impl TimeSignature {
    /// # Errors
    ///
    /// When the upper number is 0, or the lower number is not 1, 2, 4, 8
    /// or 16, or the upper number is a multiple of 3 above 12.
    pub fn new(
        pulses_per_bar: u8,
        pulses_per_whole_note: u8,
    ) -> Result<TimeSignature, TimeSignatureError> {
        if pulses_per_bar == 0 || !ALLOWED_PULSE_NOTE_VALUES.contains(&pulses_per_whole_note) {
            return Err(TimeSignatureError::NotAllowed {
                pulses_per_bar,
                pulses_per_whole_note,
            });
        }
        if pulses_per_bar > LARGEST_COMPOUND_PULSES_PER_BAR
            && u64::from(pulses_per_bar).is_multiple_of(PULSES_PER_COMPOUND_BEAT)
        {
            return Err(TimeSignatureError::CompoundAboveTwelve {
                pulses_per_bar,
                pulses_per_whole_note,
            });
        }
        Ok(TimeSignature {
            pulses_per_bar,
            pulses_per_whole_note,
        })
    }

    #[must_use]
    pub const fn pulses_per_bar(self) -> u8 {
        self.pulses_per_bar
    }

    #[must_use]
    pub const fn pulses_per_whole_note(self) -> u8 {
        self.pulses_per_whole_note
    }

    /// `L_pulse = 20160 ticks/w ÷ d pulses/w` → ticks/pulse. Exact: d is a
    /// power of 2 up to 16, and 20160 = 2⁶ × 315.
    #[must_use]
    pub const fn pulse_length(self) -> ScoreLength {
        ScoreLength::from_ticks(TICKS_PER_WHOLE_NOTE / self.pulses_per_whole_note as u64)
    }

    /// Compound time: upper figure 6, 9 or 12. Every other meter is simple,
    /// including irregular ones such as 5/8 and 7/8.
    #[must_use]
    pub fn is_compound(self) -> bool {
        COMPOUND_PULSES_PER_BAR.contains(&self.pulses_per_bar)
    }

    /// 1 pulse/beat in simple time, 3 in compound time.
    #[must_use]
    pub fn pulses_per_beat(self) -> u64 {
        if self.is_compound() {
            PULSES_PER_COMPOUND_BEAT
        } else {
            1
        }
    }

    /// `L_beat = L_pulse` in simple time, `3 pulses × L_pulse ticks/pulse`
    /// in compound time → ticks/beat. Tempo and the counter count these
    /// beats.
    #[must_use]
    pub fn beat_length(self) -> ScoreLength {
        // pulses/beat × ticks/pulse → ticks/beat.
        ScoreLength::from_ticks(self.pulses_per_beat() * self.pulse_length().ticks())
    }

    /// `L_bar = n pulses × L_pulse ticks/pulse` → ticks. At most
    /// 255 pulses × 20160 ticks/pulse = 5140800 ticks, so it cannot
    /// overflow a `u64`.
    #[must_use]
    pub const fn bar_length(self) -> ScoreLength {
        ScoreLength::from_ticks(self.pulses_per_bar as u64 * self.pulse_length().ticks())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSignatureError {
    NotAllowed {
        pulses_per_bar: u8,
        pulses_per_whole_note: u8,
    },
    CompoundAboveTwelve {
        pulses_per_bar: u8,
        pulses_per_whole_note: u8,
    },
}

impl fmt::Display for TimeSignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeSignatureError::NotAllowed {
                pulses_per_bar,
                pulses_per_whole_note,
            } => write!(
                f,
                "{pulses_per_bar}/{pulses_per_whole_note} is not a time signature. \
                 Use 1 or more on top, and 1, 2, 4, 8 or 16 below: e.g. 4/4, 6/8"
            ),
            TimeSignatureError::CompoundAboveTwelve {
                pulses_per_bar,
                pulses_per_whole_note,
            } => write!(
                f,
                "{pulses_per_bar}/{pulses_per_whole_note} is not read yet: compound meters stop \
                 at 12 on top. Write it as shorter bars, e.g. 6/{pulses_per_whole_note}, \
                 9/{pulses_per_whole_note} or 12/{pulses_per_whole_note}"
            ),
        }
    }
}

impl std::error::Error for TimeSignatureError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn lengths_ticks(pulses_per_bar: u8, pulses_per_whole_note: u8) -> (u64, u64) {
        let signature: TimeSignature =
            TimeSignature::new(pulses_per_bar, pulses_per_whole_note).unwrap();
        (
            signature.pulse_length().ticks(),
            signature.bar_length().ticks(),
        )
    }

    #[test]
    fn pulse_and_bar_lengths_match_the_reference_table() {
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

    fn beat_ticks(pulses_per_bar: u8, pulses_per_whole_note: u8) -> u64 {
        TimeSignature::new(pulses_per_bar, pulses_per_whole_note)
            .unwrap()
            .beat_length()
            .ticks()
    }

    #[test]
    fn beats_match_the_reference_table() {
        // Simple time: the beat is one pulse.
        assert_eq!(beat_ticks(4, 4), 5040);
        assert_eq!(beat_ticks(3, 8), 2520);
        assert_eq!(beat_ticks(2, 2), 10_080);
        assert_eq!(beat_ticks(5, 4), 5040);
        assert_eq!(beat_ticks(7, 8), 2520);
        assert_eq!(beat_ticks(3, 4), 5040);
        // Compound time: the beat is three pulses, a dotted note.
        assert_eq!(beat_ticks(6, 8), 7560);
        assert_eq!(beat_ticks(9, 8), 7560);
        assert_eq!(beat_ticks(12, 8), 7560);
        assert_eq!(beat_ticks(9, 4), 15_120);
        assert_eq!(beat_ticks(12, 16), 3780);
        assert_eq!(beat_ticks(9, 16), 3780);
    }

    #[test]
    fn compound_meters_above_twelve_are_refused_with_a_way_out() {
        for pulses_per_bar in [15, 18, 21, 24] {
            let error: TimeSignatureError = TimeSignature::new(pulses_per_bar, 8).unwrap_err();
            assert_eq!(
                error,
                TimeSignatureError::CompoundAboveTwelve {
                    pulses_per_bar,
                    pulses_per_whole_note: 8
                }
            );
        }
        assert_eq!(
            TimeSignature::new(15, 8).unwrap_err().to_string(),
            "15/8 is not read yet: compound meters stop at 12 on top. Write it as shorter \
             bars, e.g. 6/8, 9/8 or 12/8"
        );
        // Other upper figures above 12 stay simple time.
        assert!(TimeSignature::new(13, 8).is_ok());
    }
}
