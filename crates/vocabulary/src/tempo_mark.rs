//! A tempo mark on the lead line (Song Notation, section 5, "Tempo marks on
//! the lead line"): a new tempo from the start of a bar, or a ramp that
//! moves the tempo in a straight line over a number of bars and then holds
//! the end tempo.

use crate::tempo::Tempo;

/// What a mark does at its bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempoMarkKind {
    /// The tempo from the start of the bar, until the next mark.
    Step { tempo: Tempo },
    /// From `start` at the start of the bar to `end` after `bar_count`
    /// bars, in a straight line; then `end` holds.
    Ramp {
        start: Tempo,
        end: Tempo,
        bar_count: u32,
    },
}

/// A tempo mark at a bar of a section, counted from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempoMark {
    bar_index: u32,
    kind: TempoMarkKind,
}

impl TempoMark {
    #[must_use]
    pub fn step(bar_index: u32, tempo: Tempo) -> TempoMark {
        TempoMark {
            bar_index,
            kind: TempoMarkKind::Step { tempo },
        }
    }

    /// A ramp over `bar_count` bars. A ramp over no bars would never move,
    /// so a `bar_count` of 0 gives `None`.
    #[must_use]
    pub fn ramp(bar_index: u32, start: Tempo, end: Tempo, bar_count: u32) -> Option<TempoMark> {
        if bar_count == 0 {
            return None;
        }
        Some(TempoMark {
            bar_index,
            kind: TempoMarkKind::Ramp {
                start,
                end,
                bar_count,
            },
        })
    }

    /// The bar the mark is in, counted from 0 along the section.
    #[must_use]
    pub fn bar_index(&self) -> u32 {
        self.bar_index
    }

    #[must_use]
    pub fn kind(&self) -> TempoMarkKind {
        self.kind
    }

    /// The bar after the last one the mark covers: the next bar for a step,
    /// the bar where a ramp arrives.
    #[must_use]
    pub fn end_bar_index(&self) -> u32 {
        match self.kind {
            TempoMarkKind::Step { .. } => self.bar_index + 1,
            TempoMarkKind::Ramp { bar_count, .. } => self.bar_index + bar_count,
        }
    }

    /// The tempo in force once the mark has done its work.
    #[must_use]
    pub fn end_tempo(&self) -> Tempo {
        match self.kind {
            TempoMarkKind::Step { tempo } => tempo,
            TempoMarkKind::Ramp { end, .. } => end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempo(milli_quarters_per_minute: u64) -> Tempo {
        Tempo::from_milli_quarters_per_minute(milli_quarters_per_minute).unwrap()
    }

    #[test]
    fn a_step_covers_its_own_bar() {
        let mark: TempoMark = TempoMark::step(3, tempo(130_000));
        assert_eq!(mark.end_bar_index(), 4);
        assert_eq!(mark.end_tempo(), tempo(130_000));
    }

    #[test]
    fn a_ramp_covers_its_bars_and_ends_at_its_end_tempo() {
        let mark: TempoMark = TempoMark::ramp(2, tempo(110_000), tempo(130_000), 2).unwrap();
        assert_eq!(mark.end_bar_index(), 4);
        assert_eq!(mark.end_tempo(), tempo(130_000));
    }

    #[test]
    fn a_ramp_over_no_bars_is_refused() {
        assert!(TempoMark::ramp(0, tempo(110_000), tempo(130_000), 0).is_none());
    }
}
