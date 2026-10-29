//! The tick: the unit of score quantity, 1/5040 of a quarter note.
//!
//! A tick is a unit of note length, not of seconds. "Tick" means only this
//! unit; MIDI's 24-per-quarter-note messages are clock pulses and live with
//! the clock, not here. See docs/Architecture/Musical Time and Clock.md.

/// 5040 = 2⁴ × 3² × 5 × 7, so triplets, quintuplets, septuplets, dotted
/// values and sixty-fourth notes are all whole numbers of ticks.
pub const TICKS_PER_QUARTER_NOTE: u64 = 5040;

/// 4 q × 5040 ticks/q = 20160 ticks/w.
pub const TICKS_PER_WHOLE_NOTE: u64 = 4 * TICKS_PER_QUARTER_NOTE;
