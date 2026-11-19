//! The marks of a dynamics line (Song Notation, section 5, "Dynamics, in
//! rhythm notation"): the levels `pp`, `p`, `mp`, `mf`, `f`, `ff`, and the
//! crescendo `<` and diminuendo `>` between them.

use std::fmt;

use crate::midi_values::Velocity;

/// A level of loudness, relative to the track's normal velocity, which is
/// `mf`. The softer levels move the normal velocity a share of the way toward
/// the softest level, the louder ones toward the loudest, so `pp` to `ff`
/// use the range of velocities, less a safety gap at each end.
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

    /// `pp` all the way from `normal` to the softest level (6), `p` half way
    /// and `mp` a quarter of the way; `mf` is `normal`; `f` half way and
    /// `ff` all the way to the loudest level (121). So `pp` to `ff` use the
    /// whole range, less a safety gap at each end (Song Notation, section
    /// 6). A normal velocity already past an end keeps the levels on that
    /// side at `normal`. Rounded to the nearest whole velocity.
    #[must_use]
    pub fn velocity(self, normal: Velocity) -> Velocity {
        let softest: Velocity = normal.min(Velocity::SOFTEST_LEVEL);
        let loudest: Velocity = normal.max(Velocity::LOUDEST_LEVEL);
        match self {
            DynamicLevel::Pianissimo => normal.sixteenths_of_the_way_toward(softest, 16),
            DynamicLevel::Piano => normal.sixteenths_of_the_way_toward(softest, 8),
            DynamicLevel::MezzoPiano => normal.sixteenths_of_the_way_toward(softest, 4),
            DynamicLevel::MezzoForte => normal,
            DynamicLevel::Forte => normal.sixteenths_of_the_way_toward(loudest, 8),
            DynamicLevel::Fortissimo => normal.sixteenths_of_the_way_toward(loudest, 16),
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
        assert_eq!(at_normal(64), vec![6, 35, 50, 64, 93, 121]);
        assert_eq!(at_normal(90), vec![6, 48, 69, 90, 106, 121]);
        assert_eq!(at_normal(100), vec![6, 53, 77, 100, 111, 121]);
    }

    #[test]
    fn a_normal_past_an_end_keeps_the_levels_on_that_side_at_normal() {
        // Normal 3 is already softer than the softest level.
        assert_eq!(at_normal(3)[..4], [3, 3, 3, 3]);
        // Normal 125 is already louder than the loudest level.
        assert_eq!(at_normal(125)[3..], [125, 125, 125]);
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
