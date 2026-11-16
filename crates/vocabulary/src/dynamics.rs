//! The marks of a dynamics line (Song Notation, section 5, "Dynamics, in
//! rhythm notation"): the levels `pp`, `p`, `mp`, `mf`, `f`, `ff`, and the
//! crescendo `<` and diminuendo `>` between them.

use std::fmt;

use crate::midi_values::{Velocity, VelocityLimit};

/// A level of loudness, relative to the track's normal velocity, which is
/// `mf`. The softer levels move the normal velocity a share of the room left
/// toward 1, the louder ones toward 127, so no level reaches the limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicLevel {
    Pianissimo,
    Piano,
    MezzoPiano,
    MezzoForte,
    Forte,
    Fortissimo,
}

impl DynamicLevel {
    /// The level as the song writes it.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            DynamicLevel::Pianissimo => "pp",
            DynamicLevel::Piano => "p",
            DynamicLevel::MezzoPiano => "mp",
            DynamicLevel::MezzoForte => "mf",
            DynamicLevel::Forte => "f",
            DynamicLevel::Fortissimo => "ff",
        }
    }

    /// `pp` 3/4, `p` 3/8, `mp` 3/16 of the way from `normal` to 1; `mf` is
    /// `normal`; `f` 3/8 and `ff` 3/4 of the way to 127 (Song Notation,
    /// section 6), rounded to the nearest whole velocity.
    #[must_use]
    pub fn velocity(self, normal: Velocity) -> Velocity {
        match self {
            DynamicLevel::Pianissimo => normal.sixteenths_of_the_way_to(VelocityLimit::Softest, 12),
            DynamicLevel::Piano => normal.sixteenths_of_the_way_to(VelocityLimit::Softest, 6),
            DynamicLevel::MezzoPiano => normal.sixteenths_of_the_way_to(VelocityLimit::Softest, 3),
            DynamicLevel::MezzoForte => normal,
            DynamicLevel::Forte => normal.sixteenths_of_the_way_to(VelocityLimit::Loudest, 6),
            DynamicLevel::Fortissimo => normal.sixteenths_of_the_way_to(VelocityLimit::Loudest, 12),
        }
    }
}

impl fmt::Display for DynamicLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text())
    }
}

/// What a sounding element of a dynamics line says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicMark {
    /// The level holds from here until the next mark.
    Level(DynamicLevel),
    /// Growing, in a straight line, to the next level written.
    Crescendo,
    /// Falling, in a straight line, to the next level written.
    Diminuendo,
}

impl DynamicMark {
    /// The mark a word of a dynamics line names, if it names one.
    #[must_use]
    pub fn from_text(text: &str) -> Option<DynamicMark> {
        let level: Option<DynamicLevel> = match text {
            "pp" => Some(DynamicLevel::Pianissimo),
            "p" => Some(DynamicLevel::Piano),
            "mp" => Some(DynamicLevel::MezzoPiano),
            "mf" => Some(DynamicLevel::MezzoForte),
            "f" => Some(DynamicLevel::Forte),
            "ff" => Some(DynamicLevel::Fortissimo),
            _ => None,
        };
        match (level, text) {
            (Some(level), _) => Some(DynamicMark::Level(level)),
            (None, "<") => Some(DynamicMark::Crescendo),
            (None, ">") => Some(DynamicMark::Diminuendo),
            (None, _) => None,
        }
    }

    /// Whether the mark is a crescendo or a diminuendo, which need a level
    /// written after them to say where they end.
    #[must_use]
    pub const fn is_ramp(self) -> bool {
        matches!(self, DynamicMark::Crescendo | DynamicMark::Diminuendo)
    }
}

impl fmt::Display for DynamicMark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DynamicMark::Level(level) => write!(f, "{level}"),
            DynamicMark::Crescendo => write!(f, "<"),
            DynamicMark::Diminuendo => write!(f, ">"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_normal(normal: u8) -> Vec<u8> {
        let normal: Velocity = Velocity::new(normal).unwrap();
        [
            DynamicLevel::Pianissimo,
            DynamicLevel::Piano,
            DynamicLevel::MezzoPiano,
            DynamicLevel::MezzoForte,
            DynamicLevel::Forte,
            DynamicLevel::Fortissimo,
        ]
        .iter()
        .map(|level: &DynamicLevel| level.velocity(normal).value())
        .collect()
    }

    #[test]
    fn the_levels_match_the_table_in_song_notation() {
        // pp, p, mp, mf, f, ff at normal velocity 64, 90 and 100.
        assert_eq!(at_normal(64), vec![17, 40, 52, 64, 88, 111]);
        assert_eq!(at_normal(90), vec![23, 57, 73, 90, 104, 118]);
        assert_eq!(at_normal(100), vec![26, 63, 81, 100, 110, 120]);
    }

    #[test]
    fn a_level_is_always_a_valid_velocity() {
        for normal in 1..=127_u8 {
            for velocity in at_normal(normal) {
                assert!((1..=127).contains(&velocity), "{normal}");
            }
        }
    }
}
