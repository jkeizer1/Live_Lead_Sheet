//! How the pulses of an irregular bar are grouped for accents and beaming:
//! `meter 7/8  grouping 2+2+3`. The beat and the tempo do not change; the
//! grouping says where the accents fall and how the notes are beamed
//! (Song Notation, section 5, "Beat grouping").

use std::fmt;

use crate::time_signature::TimeSignature;

/// The sizes of the groups of a bar, in pulses, in bar order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeatGrouping {
    group_pulses: Vec<u8>,
}

impl BeatGrouping {
    /// # Errors
    ///
    /// When there are no groups, a group has 0 pulses, or the groups do not
    /// add up to the pulses in the bar of `time_signature`.
    pub fn new(
        group_pulses: Vec<u8>,
        time_signature: TimeSignature,
    ) -> Result<BeatGrouping, BeatGroupingError> {
        let total_pulses: u32 = group_pulses
            .iter()
            .map(|&pulses: &u8| u32::from(pulses))
            .sum();
        if group_pulses.is_empty()
            || group_pulses.contains(&0)
            || total_pulses != u32::from(time_signature.pulses_per_bar())
        {
            return Err(BeatGroupingError::DoesNotFillTheBar {
                group_pulses,
                total_pulses,
                time_signature,
            });
        }
        Ok(BeatGrouping { group_pulses })
    }

    /// The group sizes, in pulses.
    #[must_use]
    pub fn group_pulses(&self) -> &[u8] {
        &self.group_pulses
    }

    /// Whether this grouping fills a bar of `time_signature`; a section's
    /// meter and its grouping must agree.
    #[must_use]
    pub fn fits(&self, time_signature: TimeSignature) -> bool {
        let total_pulses: u32 = self
            .group_pulses
            .iter()
            .map(|&pulses: &u8| u32::from(pulses))
            .sum();
        total_pulses == u32::from(time_signature.pulses_per_bar())
    }
}

impl fmt::Display for BeatGrouping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let words: Vec<String> = self
            .group_pulses
            .iter()
            .map(|pulses: &u8| pulses.to_string())
            .collect();
        write!(f, "{}", words.join("+"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeatGroupingError {
    DoesNotFillTheBar {
        group_pulses: Vec<u8>,
        total_pulses: u32,
        time_signature: TimeSignature,
    },
}

impl fmt::Display for BeatGroupingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BeatGroupingError::DoesNotFillTheBar {
                group_pulses,
                total_pulses,
                time_signature,
            } => {
                let written: Vec<String> = group_pulses
                    .iter()
                    .map(|pulses: &u8| pulses.to_string())
                    .collect();
                write!(
                    f,
                    "the grouping {} adds up to {total_pulses} pulses, but a bar of {}/{} has \
                     {}. Write groups of 1 or more that add up to {}, e.g. grouping 2+2+3 for 7/8",
                    written.join("+"),
                    time_signature.pulses_per_bar(),
                    time_signature.pulses_per_whole_note(),
                    time_signature.pulses_per_bar(),
                    time_signature.pulses_per_bar(),
                )
            }
        }
    }
}

impl std::error::Error for BeatGroupingError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn seven_eight() -> TimeSignature {
        TimeSignature::new(7, 8).unwrap()
    }

    #[test]
    fn groups_that_fill_the_bar_are_kept_in_order() {
        let grouping: BeatGrouping = BeatGrouping::new(vec![2, 2, 3], seven_eight()).unwrap();
        assert_eq!(grouping.group_pulses(), &[2, 2, 3]);
        assert_eq!(grouping.to_string(), "2+2+3");
        assert!(grouping.fits(seven_eight()));
        assert!(!grouping.fits(TimeSignature::new(5, 4).unwrap()));
    }

    #[test]
    fn groups_that_do_not_fill_the_bar_are_refused_with_what_to_write() {
        let error: BeatGroupingError = BeatGrouping::new(vec![2, 2, 2], seven_eight()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the grouping 2+2+2 adds up to 6 pulses, but a bar of 7/8 has 7. Write groups of 1 \
             or more that add up to 7, e.g. grouping 2+2+3 for 7/8"
        );
        assert!(BeatGrouping::new(vec![], seven_eight()).is_err());
        assert!(BeatGrouping::new(vec![7, 0], seven_eight()).is_err());
    }
}
