//! A chord written as a Roman numeral against the key (EFS-1.5a): upper case
//! for a major-type chord, lower case for a minor-type one, with an optional
//! flat or sharp before it (`bVII7`). Turning it into notes in a key is the
//! voicing's job, not this type's.

use std::fmt;

use crate::pitch::Accidental;

/// The seven degrees of the scale, I to VII.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleDegree {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Sixth,
    Seventh,
}

impl ScaleDegree {
    /// 1 to 7.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            ScaleDegree::First => 1,
            ScaleDegree::Second => 2,
            ScaleDegree::Third => 3,
            ScaleDegree::Fourth => 4,
            ScaleDegree::Fifth => 5,
            ScaleDegree::Sixth => 6,
            ScaleDegree::Seventh => 7,
        }
    }

    /// The numeral in upper case.
    #[must_use]
    pub const fn numeral(self) -> &'static str {
        match self {
            ScaleDegree::First => "I",
            ScaleDegree::Second => "II",
            ScaleDegree::Third => "III",
            ScaleDegree::Fourth => "IV",
            ScaleDegree::Fifth => "V",
            ScaleDegree::Sixth => "VI",
            ScaleDegree::Seventh => "VII",
        }
    }
}

/// The chords V1 knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordKind {
    Major,
    Minor,
    Diminished,
    Augmented,
    DominantSeventh,
    MajorSeventh,
    MinorSeventh,
    DiminishedSeventh,
    HalfDiminishedSeventh,
    DominantSeventhFlatNinth,
}

impl ChordKind {
    /// Written in lower case numerals.
    #[must_use]
    pub const fn is_minor_type(self) -> bool {
        matches!(
            self,
            ChordKind::Minor
                | ChordKind::Diminished
                | ChordKind::MinorSeventh
                | ChordKind::DiminishedSeventh
                | ChordKind::HalfDiminishedSeventh
        )
    }

    /// What follows the numeral, as EFS-1.5a spells it.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            ChordKind::Major | ChordKind::Minor => "",
            ChordKind::Diminished => "°",
            ChordKind::Augmented => "+",
            ChordKind::DominantSeventh | ChordKind::MinorSeventh => "7",
            ChordKind::MajorSeventh => "M7",
            ChordKind::DiminishedSeventh => "°7",
            ChordKind::HalfDiminishedSeventh => "ø7",
            ChordKind::DominantSeventhFlatNinth => "7b9",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RomanChord {
    /// Flat or sharp before the numeral (`bVII`); natural otherwise.
    accidental: Accidental,
    degree: ScaleDegree,
    kind: ChordKind,
}

impl RomanChord {
    #[must_use]
    pub const fn new(accidental: Accidental, degree: ScaleDegree, kind: ChordKind) -> RomanChord {
        RomanChord {
            accidental,
            degree,
            kind,
        }
    }

    #[must_use]
    pub const fn accidental(self) -> Accidental {
        self.accidental
    }

    #[must_use]
    pub const fn degree(self) -> ScaleDegree {
        self.degree
    }

    #[must_use]
    pub const fn kind(self) -> ChordKind {
        self.kind
    }
}

impl fmt::Display for RomanChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let numeral: &str = self.degree.numeral();
        let numeral: String = if self.kind.is_minor_type() {
            numeral.to_lowercase()
        } else {
            numeral.to_string()
        };
        write!(
            f,
            "{}{}{}",
            self.accidental.text(),
            numeral,
            self.kind.suffix()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_as_in_the_specification() {
        let name = |accidental, degree, kind| RomanChord::new(accidental, degree, kind).to_string();
        assert_eq!(
            name(Accidental::Natural, ScaleDegree::First, ChordKind::Major),
            "I"
        );
        assert_eq!(
            name(
                Accidental::Natural,
                ScaleDegree::Second,
                ChordKind::MinorSeventh
            ),
            "ii7"
        );
        assert_eq!(
            name(
                Accidental::Flat,
                ScaleDegree::Seventh,
                ChordKind::DominantSeventh
            ),
            "bVII7"
        );
        assert_eq!(
            name(
                Accidental::Natural,
                ScaleDegree::Seventh,
                ChordKind::Diminished
            ),
            "vii°"
        );
    }
}
