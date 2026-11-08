//! A written note value: a whole, half, quarter, eighth, sixteenth or
//! thirty-second note, with up to three dots (Song Notation, section 3).

use vocabulary::ticks::TICKS_PER_WHOLE_NOTE;

/// Most dots a written value may have, as in the song notation.
const MAX_DOTS: u8 = 3;

/// The undotted note values, longest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseValue {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
}

impl BaseValue {
    const ALL: [BaseValue; 6] = [
        BaseValue::Whole,
        BaseValue::Half,
        BaseValue::Quarter,
        BaseValue::Eighth,
        BaseValue::Sixteenth,
        BaseValue::ThirtySecond,
    ];

    /// How many of this value fill a whole note: 1, 2, 4, 8, 16 or 32, the
    /// number notation programs write for it.
    #[must_use]
    pub const fn per_whole_note(self) -> u64 {
        match self {
            BaseValue::Whole => 1,
            BaseValue::Half => 2,
            BaseValue::Quarter => 4,
            BaseValue::Eighth => 8,
            BaseValue::Sixteenth => 16,
            BaseValue::ThirtySecond => 32,
        }
    }

    /// The length in ticks: 20160 ticks per whole note ÷ (values per whole note).
    #[must_use]
    pub const fn ticks(self) -> u64 {
        TICKS_PER_WHOLE_NOTE / self.per_whole_note()
    }
}

/// A note value with its dots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteValue {
    base: BaseValue,
    dots: u8,
}

impl NoteValue {
    #[must_use]
    pub const fn new(base: BaseValue, dots: u8) -> NoteValue {
        NoteValue { base, dots }
    }

    #[must_use]
    pub const fn base(self) -> BaseValue {
        self.base
    }

    #[must_use]
    pub const fn dots(self) -> u8 {
        self.dots
    }

    /// The length in ticks: each dot adds half of what the one before added.
    #[must_use]
    pub fn ticks(self) -> u64 {
        let mut length_ticks: u64 = self.base.ticks();
        let mut added_ticks: u64 = self.base.ticks();
        for _ in 0..self.dots {
            added_ticks /= 2;
            length_ticks += added_ticks;
        }
        length_ticks
    }

    /// The value whose length is exactly `length_ticks`, if there is one.
    /// A triple-dotted thirty-second is not whole ticks and is never found.
    #[must_use]
    pub fn from_ticks(length_ticks: u64) -> Option<NoteValue> {
        for base in BaseValue::ALL {
            for dots in 0..=MAX_DOTS {
                // Exact only: a value whose dots fall on half ticks is
                // checked by multiplying, not by the rounded `ticks()`.
                if exact_dotted_ticks(base, dots) == Some(length_ticks) {
                    return Some(NoteValue::new(base, dots));
                }
            }
        }
        None
    }
}

/// The dotted length in ticks when it is a whole number of ticks:
/// base × (2 − 1/2^dots) = base × (2^(dots+1) − 1) / 2^dots.
fn exact_dotted_ticks(base: BaseValue, dots: u8) -> Option<u64> {
    let denominator: u64 = 1_u64 << dots;
    let numerator: u64 = base.ticks() * ((denominator << 1) - 1);
    numerator
        .is_multiple_of(denominator)
        .then_some(numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_dotted_values_are_found() {
        assert_eq!(
            NoteValue::from_ticks(5040),
            Some(NoteValue::new(BaseValue::Quarter, 0))
        );
        assert_eq!(
            NoteValue::from_ticks(7560),
            Some(NoteValue::new(BaseValue::Quarter, 1))
        );
        assert_eq!(
            NoteValue::from_ticks(8820),
            Some(NoteValue::new(BaseValue::Quarter, 2))
        );
        assert_eq!(
            NoteValue::from_ticks(1260),
            Some(NoteValue::new(BaseValue::Sixteenth, 0))
        );
        assert_eq!(NoteValue::new(BaseValue::Half, 1).ticks(), 15_120);
    }

    #[test]
    fn a_triplet_length_is_no_note_value() {
        assert_eq!(NoteValue::from_ticks(3360), None);
        assert_eq!(NoteValue::from_ticks(1008), None);
    }
}
