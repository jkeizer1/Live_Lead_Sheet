//! What a track's sample source holds while one of its rhythm elements
//! sounds (Song Notation, section 5, "Sample sources and uncoupled lines").
//!
//! The rhythm is the gate and the source is the voltage being tracked: an
//! element plays what the source holds when it starts, and if the source
//! changes while the element is still sounding, the element follows it
//! ("track and hold"), so a resolving chord is never skipped. A change that
//! falls wholly inside a rest is not heard, because nothing here is asked
//! about a rest.

use vocabulary::song::{ElementContents, Line};

/// A stretch of one sounding rhythm element during which the source holds
/// the same chord or notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldStretch<'a> {
    /// Ticks from the start of the line.
    pub start_ticks: u64,
    pub length_ticks: u64,
    pub contents: &'a ElementContents,
}

/// The stretches of `source` over `[start_ticks, start_ticks + length_ticks)`
/// of the line, in order. A source element that repeats the previous
/// element's contents (`IV IV`) is not a change, so it joins the stretch
/// before it.
#[must_use]
pub fn held_stretches(source: &Line, start_ticks: u64, length_ticks: u64) -> Vec<HeldStretch<'_>> {
    let end_ticks: u64 = start_ticks + length_ticks;
    let mut stretches: Vec<HeldStretch> = Vec::new();
    for source_element in source.elements() {
        let Some(contents) = source_element.contents() else {
            continue; // a source holds contents in every element
        };
        let element_start_ticks: u64 = source_element.element().start().ticks();
        let element_end_ticks: u64 =
            element_start_ticks + source_element.element().length().ticks();
        let overlap_start_ticks: u64 = element_start_ticks.max(start_ticks);
        let overlap_end_ticks: u64 = element_end_ticks.min(end_ticks);
        if overlap_start_ticks >= overlap_end_ticks {
            continue;
        }
        match stretches.last_mut() {
            Some(last) if last.contents == contents => {
                last.length_ticks = overlap_end_ticks - last.start_ticks;
            }
            _ => stretches.push(HeldStretch {
                start_ticks: overlap_start_ticks,
                length_ticks: overlap_end_ticks - overlap_start_ticks,
                contents,
            }),
        }
    }
    stretches
}
