//! Lengths in the musician's words, for messages ("an eighth note") and for
//! the spelling that would write them ("e").

use vocabulary::ticks::{TICKS_PER_QUARTER_NOTE, TICKS_PER_WHOLE_NOTE};
use vocabulary::time_signature::TimeSignature;

/// Letter, name and length of each plain note value.
const NOTE_VALUES: [(&str, &str, u64); 6] = [
    ("w", "whole", TICKS_PER_WHOLE_NOTE),
    ("h", "half", TICKS_PER_WHOLE_NOTE / 2),
    ("q", "quarter", TICKS_PER_QUARTER_NOTE),
    ("e", "eighth", TICKS_PER_QUARTER_NOTE / 2),
    ("sx", "sixteenth", TICKS_PER_QUARTER_NOTE / 4),
    ("tx", "thirty-second", TICKS_PER_QUARTER_NOTE / 8),
];

/// Most dots the notation accepts: `q...`.
pub const MAX_DOTS: u32 = 3;

/// The length in ticks of a note value with dots: each dot adds half of
/// what the one before added, so n dots give (2ⁿ⁺¹ − 1) ÷ 2ⁿ times the
/// plain value. Returned in eighths of a tick, which keeps a double-dotted
/// thirty-second note (1102.5 ticks) exact.
///
/// `base_ticks × 8 subticks/tick × (2^(n+1) − 1) ÷ 2^n` → subticks.
#[must_use]
pub fn dotted_length_subticks(base_ticks: u64, dots: u32, subticks_per_tick: u64) -> u64 {
    let dotted_numerator: u64 = (1_u64 << (dots + 1)) - 1;
    base_ticks * subticks_per_tick * dotted_numerator / (1_u64 << dots)
}

/// How to say and write a length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthWords {
    /// "an eighth note", "a dotted quarter note", or "7/12 of a quarter note".
    pub description: String,
    /// The notation that writes it (`e`, `q.`), when it is one note value.
    pub letters: Option<String>,
}

/// Names a length given in ticks.
#[must_use]
pub fn describe_length(length_ticks: u64) -> LengthWords {
    for (letter, name, base_ticks) in NOTE_VALUES {
        for dots in 0..=2_u32 {
            let numerator: u64 = base_ticks * ((1_u64 << (dots + 1)) - 1);
            let denominator: u64 = 1_u64 << dots;
            if !numerator.is_multiple_of(denominator) || numerator / denominator != length_ticks {
                continue;
            }
            let dotted_name: String = match dots {
                0 => name.to_string(),
                1 => format!("dotted {name}"),
                _ => format!("double-dotted {name}"),
            };
            let article: &str = if dotted_name.starts_with(['a', 'e', 'i', 'o', 'u']) {
                "an"
            } else {
                "a"
            };
            return LengthWords {
                description: format!("{article} {dotted_name} note"),
                letters: Some(format!("{letter}{}", ".".repeat(dots as usize))),
            };
        }
    }
    // A triplet value: two thirds of a plain note value.
    // base ticks × 2 ÷ 3 → ticks.
    for (_, name, base_ticks) in NOTE_VALUES {
        if (base_ticks * 2).is_multiple_of(3) && base_ticks * 2 / 3 == length_ticks {
            return LengthWords {
                description: format!("a triplet {name} note"),
                letters: None,
            };
        }
    }
    // Not one note value: say it as a fraction of a quarter note.
    // length_ticks ÷ 5040 ticks/q → quarter notes, reduced.
    let common: u64 = greatest_common_divisor(length_ticks, TICKS_PER_QUARTER_NOTE);
    let quarters_numerator: u64 = length_ticks / common;
    let quarters_denominator: u64 = TICKS_PER_QUARTER_NOTE / common;
    let description: String = if quarters_denominator == 1 {
        format!("{quarters_numerator} quarter notes")
    } else {
        format!("{quarters_numerator}/{quarters_denominator} of a quarter note")
    };
    LengthWords {
        description,
        letters: None,
    }
}

/// Where a place is in the bar, in the musician's words: "bar 2 count 3" or
/// "bar 1 count 4 plus an eighth note".
#[must_use]
pub fn describe_place(start_ticks: u64, time_signature: TimeSignature) -> String {
    let bar_length_ticks: u64 = time_signature.bar_length().ticks();
    let count_length_ticks: u64 = time_signature.count_length().ticks();
    // ticks ÷ ticks/bar → bars; ticks within the bar ÷ ticks/count → counts.
    let bar: u64 = start_ticks / bar_length_ticks + 1;
    let within_bar_ticks: u64 = start_ticks % bar_length_ticks;
    let count: u64 = within_bar_ticks / count_length_ticks + 1;
    let past_count_ticks: u64 = within_bar_ticks % count_length_ticks;
    if past_count_ticks == 0 {
        format!("bar {bar} count {count}")
    } else {
        format!(
            "bar {bar} count {count} plus {}",
            describe_length(past_count_ticks).description
        )
    }
}

fn greatest_common_divisor(first: u64, second: u64) -> u64 {
    let (mut larger, mut smaller): (u64, u64) = (first.max(second), first.min(second));
    while smaller != 0 {
        (larger, smaller) = (smaller, larger % smaller);
    }
    larger
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_plain_and_dotted_values() {
        assert_eq!(describe_length(2520).description, "an eighth note");
        assert_eq!(describe_length(2520).letters.as_deref(), Some("e"));
        assert_eq!(describe_length(7560).description, "a dotted quarter note");
        assert_eq!(describe_length(7560).letters.as_deref(), Some("q."));
        assert_eq!(
            describe_length(8820).description,
            "a double-dotted quarter note"
        );
    }

    #[test]
    fn says_other_lengths_as_a_fraction_of_a_quarter_note() {
        // 5040 ticks/q ÷ 7 = 720 ticks: a septuplet sixteenth, no plain name.
        assert_eq!(describe_length(720).description, "1/7 of a quarter note");
        assert_eq!(describe_length(5040 * 5).description, "5 quarter notes");
    }

    #[test]
    fn names_triplet_values() {
        assert_eq!(describe_length(3360).description, "a triplet quarter note");
        assert_eq!(describe_length(1680).description, "a triplet eighth note");
    }

    #[test]
    fn says_a_place_in_counts() {
        let four_four: TimeSignature = TimeSignature::new(4, 4).unwrap();
        assert_eq!(describe_place(0, four_four), "bar 1 count 1");
        assert_eq!(
            describe_place(17_640, four_four),
            "bar 1 count 4 plus an eighth note"
        );
        assert_eq!(describe_place(20_160 + 5040, four_four), "bar 2 count 2");
    }

    #[test]
    fn double_dotted_thirty_second_is_a_half_tick() {
        // 630 ticks × 8 subticks/tick × 7 ÷ 4 = 8820 subticks = 1102.5 ticks.
        assert_eq!(dotted_length_subticks(630, 2, 8), 8820);
    }
}
