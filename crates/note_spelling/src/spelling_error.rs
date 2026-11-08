//! What note spelling cannot do, said in musical terms.

use std::fmt;

use player::voicing::VoicingError;
use vocabulary::ticks::TICKS_PER_QUARTER_NOTE;

/// A place in the song that cannot be spelled, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellingError {
    pub section_name: String,
    pub track_name: String,
    /// Counted from 1 in the section, as the song file counts them.
    pub bar_number: usize,
    pub problem: SpellingProblem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellingProblem {
    /// A length that is neither a plain or dotted value nor part of a
    /// triplet, e.g. one of five notes in the time of a quarter.
    LengthNotSpelled {
        length_ticks: u64,
    },
    /// A bar ends inside a triplet, or a plain note comes before the
    /// triplet is complete.
    TripletNotComplete,
    /// A chord with no key in effect; the song file refuses this, so it is
    /// reported rather than guessed if it ever arrives.
    ChordWithoutKey,
    /// A sounding element that plays nothing; the song file refuses this too.
    NothingWritten,
    Voicing(VoicingError),
}

impl fmt::Display for SpellingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "section \"{}\", track \"{}\", bar {}: ",
            self.section_name, self.track_name, self.bar_number
        )?;
        match &self.problem {
            SpellingProblem::LengthNotSpelled { length_ticks } => write!(
                f,
                "a note {} long cannot be drawn yet. The view draws plain and dotted values \
                 and triplets; the song still plays",
                fraction_of_a_quarter(*length_ticks)
            ),
            SpellingProblem::TripletNotComplete => write!(
                f,
                "a triplet is not complete in the bar, so it cannot be drawn. Check the trp \
                 group; the song still plays"
            ),
            SpellingProblem::ChordWithoutKey => {
                write!(f, "a chord with no key in effect. Add a key to the section")
            }
            SpellingProblem::NothingWritten => {
                write!(f, "a note that plays nothing. Write what it plays")
            }
            SpellingProblem::Voicing(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SpellingError {}

/// `1008 ticks` → "1/5 of a quarter note": the length as a reduced fraction
/// of a quarter note (5040 ticks), so no ticks reach the musician.
fn fraction_of_a_quarter(length_ticks: u64) -> String {
    let divisor: u64 = greatest_common_divisor(length_ticks, TICKS_PER_QUARTER_NOTE);
    let numerator: u64 = length_ticks / divisor;
    let denominator: u64 = TICKS_PER_QUARTER_NOTE / divisor;
    format!("{numerator}/{denominator} of a quarter note")
}

fn greatest_common_divisor(first: u64, second: u64) -> u64 {
    let (mut larger, mut smaller): (u64, u64) = (first.max(second), first.min(second));
    while smaller != 0 {
        let remainder: u64 = larger % smaller;
        larger = smaller;
        smaller = remainder;
    }
    larger
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_length_is_told_as_a_fraction_of_a_quarter_note() {
        assert_eq!(fraction_of_a_quarter(1008), "1/5 of a quarter note");
        assert_eq!(fraction_of_a_quarter(2880), "4/7 of a quarter note");
    }
}
