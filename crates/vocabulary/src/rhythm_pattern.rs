//! Rhythm patterns: one to eight whole bars of one time signature, as
//! notes and rests (docs/Architecture/Song Notation.md, section 3).
//!
//! A pattern holds elements as written. A tie is a flag on the element it
//! starts from, so the rhythm line can draw it across a barline; whoever
//! plays the pattern joins the tied elements into one note.

use std::fmt;

use crate::score_length::ScoreLength;
use crate::time_signature::TimeSignature;

pub const MAX_BARS_PER_RHYTHM_PATTERN: u8 = 8;

/// How hard a note is played, as an accent mark; the velocity each level
/// plays at is the receiving device's setting (the studio configuration).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentLevel {
    /// No accent mark.
    Normal,
    /// `>`
    Accent,
    /// `>>`
    Strong,
}

/// The marks an element carries (Song Notation, section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementMarks {
    accent: AccentLevel,
    staccato: bool,
    legato: bool,
    fermata: bool,
}

impl ElementMarks {
    /// An element with no marks.
    pub const NONE: ElementMarks = ElementMarks {
        accent: AccentLevel::Normal,
        staccato: false,
        legato: false,
        fermata: false,
    };

    #[must_use]
    pub const fn new(
        accent: AccentLevel,
        staccato: bool,
        legato: bool,
        fermata: bool,
    ) -> ElementMarks {
        ElementMarks {
            accent,
            staccato,
            legato,
            fermata,
        }
    }

    #[must_use]
    pub const fn accent(self) -> AccentLevel {
        self.accent
    }

    #[must_use]
    pub const fn staccato(self) -> bool {
        self.staccato
    }

    #[must_use]
    pub const fn legato(self) -> bool {
        self.legato
    }

    #[must_use]
    pub const fn fermata(self) -> bool {
        self.fermata
    }

    /// A rest may carry only a fermata.
    const fn only_fermata(self) -> bool {
        matches!(self.accent, AccentLevel::Normal) && !self.staccato && !self.legato
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    Sounding,
    Rest,
}

/// One note or rest of a rhythm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RhythmElement {
    /// Distance from the start of the pattern.
    start: ScoreLength,
    length: ScoreLength,
    kind: ElementKind,
    marks: ElementMarks,
    /// Joined to the next element into one note.
    tied_to_next: bool,
}

impl RhythmElement {
    /// # Errors
    ///
    /// When the length is zero, when a rest carries a mark other than a
    /// fermata, or when a rest is tied.
    pub fn new(
        start: ScoreLength,
        length: ScoreLength,
        kind: ElementKind,
        marks: ElementMarks,
        tied_to_next: bool,
    ) -> Result<RhythmElement, RhythmPatternError> {
        if length == ScoreLength::ZERO {
            return Err(RhythmPatternError::ElementWithoutLength {
                start_ticks: start.ticks(),
            });
        }
        if kind == ElementKind::Rest && (!marks.only_fermata() || tied_to_next) {
            return Err(RhythmPatternError::RestWithNoteMarks {
                start_ticks: start.ticks(),
            });
        }
        Ok(RhythmElement {
            start,
            length,
            kind,
            marks,
            tied_to_next,
        })
    }

    #[must_use]
    pub const fn start(&self) -> ScoreLength {
        self.start
    }

    #[must_use]
    pub const fn length(&self) -> ScoreLength {
        self.length
    }

    #[must_use]
    pub const fn kind(&self) -> ElementKind {
        self.kind
    }

    #[must_use]
    pub const fn marks(&self) -> ElementMarks {
        self.marks
    }

    #[must_use]
    pub const fn tied_to_next(&self) -> bool {
        self.tied_to_next
    }

    /// Where the element ends. Cannot overflow for an element of a
    /// pattern, which is checked to lie within 8 bars.
    #[must_use]
    pub const fn end_ticks(&self) -> u64 {
        self.start.ticks() + self.length.ticks()
    }
}

/// A named rhythm of whole bars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhythmPattern {
    name: String,
    time_signature: TimeSignature,
    bar_count: u8,
    elements: Vec<RhythmElement>,
}

impl RhythmPattern {
    /// # Errors
    ///
    /// When the name is empty; when the bar count is not 1 to 8; when the
    /// elements do not follow one another from the pattern's start to the
    /// end of its last bar; when an element crosses a barline (only a tie
    /// does that); or when a tie has no sounding element after it.
    pub fn new(
        name: &str,
        time_signature: TimeSignature,
        bar_count: u8,
        elements: Vec<RhythmElement>,
    ) -> Result<RhythmPattern, RhythmPatternError> {
        if name.trim().is_empty() {
            return Err(RhythmPatternError::EmptyName);
        }
        if bar_count == 0 || bar_count > MAX_BARS_PER_RHYTHM_PATTERN {
            return Err(RhythmPatternError::BarCountNotAllowed {
                name: name.to_string(),
                bar_count,
            });
        }
        check_elements_fill_bars(name, time_signature, u32::from(bar_count), &elements)?;
        Ok(RhythmPattern {
            name: name.to_string(),
            time_signature,
            bar_count,
            elements,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn time_signature(&self) -> TimeSignature {
        self.time_signature
    }

    #[must_use]
    pub const fn bar_count(&self) -> u8 {
        self.bar_count
    }

    #[must_use]
    pub fn elements(&self) -> &[RhythmElement] {
        &self.elements
    }
}

/// Checks that `elements` follow one another from the start to the end of
/// `bar_count` whole bars, that none crosses a barline (only a tie does
/// that), and that every tie has a sounding element after it. Shared by
/// rhythm patterns and the lines of a song. `name` is only for messages.
///
/// # Errors
///
/// The first of those rules that is broken.
pub fn check_elements_fill_bars(
    name: &str,
    time_signature: TimeSignature,
    bar_count: u32,
    elements: &[RhythmElement],
) -> Result<(), RhythmPatternError> {
    let bar_length_ticks: u64 = time_signature.bar_length().ticks();
    let total_length_ticks: u64 = u64::from(bar_count) * bar_length_ticks;

    let mut expected_start_ticks: u64 = 0;
    for (index, element) in elements.iter().enumerate() {
        if element.start().ticks() != expected_start_ticks {
            return Err(RhythmPatternError::ElementsNotContiguous {
                name: name.to_string(),
                expected_start_ticks,
                found_start_ticks: element.start().ticks(),
            });
        }
        let end_ticks: u64 = element.end_ticks();
        // The last tick of the element lies in the same bar as its start.
        if element.start().ticks() / bar_length_ticks != (end_ticks - 1) / bar_length_ticks {
            return Err(RhythmPatternError::ElementCrossesBarline {
                name: name.to_string(),
                start_ticks: element.start().ticks(),
            });
        }
        if element.tied_to_next() {
            let next_is_sounding: bool = elements
                .get(index + 1)
                .is_some_and(|next| next.kind() == ElementKind::Sounding);
            if !next_is_sounding {
                return Err(RhythmPatternError::TieWithoutNote {
                    name: name.to_string(),
                    start_ticks: element.start().ticks(),
                });
            }
        }
        expected_start_ticks = end_ticks;
    }
    if expected_start_ticks != total_length_ticks {
        return Err(RhythmPatternError::LengthNotWholeBars {
            name: name.to_string(),
            found_ticks: expected_start_ticks,
            expected_ticks: total_length_ticks,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RhythmPatternError {
    EmptyName,
    ElementWithoutLength {
        start_ticks: u64,
    },
    RestWithNoteMarks {
        start_ticks: u64,
    },
    BarCountNotAllowed {
        name: String,
        bar_count: u8,
    },
    ElementsNotContiguous {
        name: String,
        expected_start_ticks: u64,
        found_start_ticks: u64,
    },
    ElementCrossesBarline {
        name: String,
        start_ticks: u64,
    },
    TieWithoutNote {
        name: String,
        start_ticks: u64,
    },
    LengthNotWholeBars {
        name: String,
        found_ticks: u64,
        expected_ticks: u64,
    },
}

impl fmt::Display for RhythmPatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RhythmPatternError::EmptyName => {
                write!(
                    f,
                    "a rhythm pattern has no name. Give it one, e.g. Charleston"
                )
            }
            RhythmPatternError::ElementWithoutLength { start_ticks } => write!(
                f,
                "the rhythm element at {start_ticks} ticks from the pattern's start has length 0"
            ),
            RhythmPatternError::RestWithNoteMarks { start_ticks } => write!(
                f,
                "the rest at {start_ticks} ticks from the pattern's start has a mark that \
                 only a note can have, or a tie. Only a fermata may stand on a rest"
            ),
            RhythmPatternError::BarCountNotAllowed { name, bar_count } => write!(
                f,
                "rhythm \"{name}\" has {bar_count} bars. A rhythm has 1 to \
                 {MAX_BARS_PER_RHYTHM_PATTERN} bars"
            ),
            RhythmPatternError::ElementsNotContiguous {
                name,
                expected_start_ticks,
                found_start_ticks,
            } => write!(
                f,
                "rhythm \"{name}\": an element should start at {expected_start_ticks} ticks \
                 from the pattern's start but starts at {found_start_ticks} ticks"
            ),
            RhythmPatternError::ElementCrossesBarline { name, start_ticks } => write!(
                f,
                "rhythm \"{name}\": the element at {start_ticks} ticks crosses a barline. \
                 Split it and join the parts with a tie (~)"
            ),
            RhythmPatternError::TieWithoutNote { name, start_ticks } => write!(
                f,
                "rhythm \"{name}\": the tie at {start_ticks} ticks has no note after it \
                 to join"
            ),
            RhythmPatternError::LengthNotWholeBars {
                name,
                found_ticks,
                expected_ticks,
            } => write!(
                f,
                "rhythm \"{name}\" is {found_ticks} ticks long but its bars add up to \
                 {expected_ticks} ticks"
            ),
        }
    }
}

impl std::error::Error for RhythmPatternError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(
        start_ticks: u64,
        length_ticks: u64,
        kind: ElementKind,
        tied_to_next: bool,
    ) -> RhythmElement {
        RhythmElement::new(
            ScoreLength::from_ticks(start_ticks),
            ScoreLength::from_ticks(length_ticks),
            kind,
            ElementMarks::NONE,
            tied_to_next,
        )
        .unwrap()
    }

    fn four_four() -> TimeSignature {
        TimeSignature::new(4, 4).unwrap()
    }

    #[test]
    fn charleston_is_one_full_bar() {
        // Dotted quarter, eighth, half rest: 7560 + 2520 + 10080 = 20160 ticks.
        let elements: Vec<RhythmElement> = vec![
            element(0, 7560, ElementKind::Sounding, false),
            element(7560, 2520, ElementKind::Sounding, false),
            element(10_080, 10_080, ElementKind::Rest, false),
        ];
        let pattern: RhythmPattern =
            RhythmPattern::new("Charleston", four_four(), 1, elements).unwrap();
        assert_eq!(pattern.bar_count(), 1);
        assert_eq!(pattern.elements().len(), 3);
    }

    #[test]
    fn a_short_pattern_is_refused() {
        let elements: Vec<RhythmElement> = vec![element(0, 15_120, ElementKind::Sounding, false)];
        let error: RhythmPatternError =
            RhythmPattern::new("Short", four_four(), 1, elements).unwrap_err();
        assert!(matches!(
            error,
            RhythmPatternError::LengthNotWholeBars {
                found_ticks: 15_120,
                expected_ticks: 20_160,
                ..
            }
        ));
    }

    #[test]
    fn an_element_may_not_cross_a_barline() {
        // A half note starting on beat 4 of bar 1.
        let elements: Vec<RhythmElement> = vec![
            element(0, 15_120, ElementKind::Rest, false),
            element(15_120, 10_080, ElementKind::Sounding, false),
            element(25_200, 15_120, ElementKind::Rest, false),
        ];
        let error: RhythmPatternError =
            RhythmPattern::new("Crossing", four_four(), 2, elements).unwrap_err();
        assert!(matches!(
            error,
            RhythmPatternError::ElementCrossesBarline { .. }
        ));
    }

    #[test]
    fn a_tie_across_a_barline_is_allowed_and_a_tie_to_a_rest_is_not() {
        let tied: Vec<RhythmElement> = vec![
            element(0, 17_640, ElementKind::Rest, false),
            element(17_640, 2520, ElementKind::Sounding, true),
            element(20_160, 5040, ElementKind::Sounding, false),
            element(25_200, 15_120, ElementKind::Rest, false),
        ];
        assert!(RhythmPattern::new("Anticipation", four_four(), 2, tied).is_ok());

        let to_rest: Vec<RhythmElement> = vec![
            element(0, 15_120, ElementKind::Rest, false),
            element(15_120, 5040, ElementKind::Sounding, true),
            element(20_160, 20_160, ElementKind::Rest, false),
        ];
        assert!(matches!(
            RhythmPattern::new("Bad tie", four_four(), 2, to_rest).unwrap_err(),
            RhythmPatternError::TieWithoutNote { .. }
        ));
    }

    #[test]
    fn a_rest_takes_only_a_fermata() {
        let accented_rest = RhythmElement::new(
            ScoreLength::ZERO,
            ScoreLength::from_ticks(5040),
            ElementKind::Rest,
            ElementMarks::new(AccentLevel::Accent, false, false, false),
            false,
        );
        assert!(accented_rest.is_err());
        let fermata_rest = RhythmElement::new(
            ScoreLength::ZERO,
            ScoreLength::from_ticks(5040),
            ElementKind::Rest,
            ElementMarks::new(AccentLevel::Normal, false, false, true),
            false,
        );
        assert!(fermata_rest.is_ok());
    }

    #[test]
    fn bar_counts_outside_one_to_eight_are_refused() {
        assert!(RhythmPattern::new("None", four_four(), 0, vec![]).is_err());
        assert!(RhythmPattern::new("Nine", four_four(), 9, vec![]).is_err());
    }
}
