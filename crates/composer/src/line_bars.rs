//! A section's line as bars the editor can change, and the harmony a line
//! holds: which chord (or note) is in effect from where in each bar. Both
//! come from the song as read, so they hold exactly what the text says.

use note_spelling::note_value::{BaseValue, NoteValue};
use vocabulary::bar_meters::BarMeters;
use vocabulary::rhythm_pattern::{ElementKind, ElementMarks};
use vocabulary::song::{ElementContents, Line, LineElement};

use crate::line_text::{WrittenBar, WrittenElement, WrittenSound};

/// The line's elements, bar by bar. Elements never cross a barline (the
/// song file refuses it), so each element belongs to the bar it starts in.
#[must_use]
pub fn written_bars_of_line(line: &Line, bar_meters: &BarMeters) -> Vec<WrittenBar> {
    let bar_count: usize = usize::try_from(line.bar_count()).unwrap_or(0);
    let mut bars: Vec<WrittenBar> = vec![WrittenBar::default(); bar_count];
    for line_element in line.elements() {
        let (bar_index, _): (usize, u64) =
            bar_meters.bar_at(line_element.element().start().ticks());
        if let Some(bar) = bars.get_mut(bar_index) {
            bar.elements.push(written_element(line_element));
        }
    }
    bars
}

fn written_element(line_element: &LineElement) -> WrittenElement {
    let sound: WrittenSound = match (line_element.element().kind(), line_element.contents()) {
        (ElementKind::Rest, _) => WrittenSound::Rest,
        (ElementKind::Sounding, None) => WrittenSound::FromSource,
        (ElementKind::Sounding, Some(contents)) => WrittenSound::Plays(contents.clone()),
    };
    WrittenElement {
        sound,
        length_ticks: line_element.element().length().ticks(),
        marks: line_element.element().marks(),
        tied_to_next: line_element.element().tied_to_next(),
        octave_shift_octaves: line_element.octave_shift_octaves(),
    }
}

/// A bar of rests filling `bar_length_ticks`: one rest of the bar's value
/// when there is one (a whole rest in 4/4, a dotted half in 3/4), else the
/// longest values that fit, in turn (5/4 is a whole rest and a quarter).
#[must_use]
pub fn rest_bar(bar_length_ticks: u64) -> WrittenBar {
    let mut elements: Vec<WrittenElement> = Vec::new();
    let mut left_ticks: u64 = bar_length_ticks;
    let values: [NoteValue; 7] = [
        NoteValue::new(BaseValue::Whole, 1),
        NoteValue::new(BaseValue::Whole, 0),
        NoteValue::new(BaseValue::Half, 1),
        NoteValue::new(BaseValue::Half, 0),
        NoteValue::new(BaseValue::Quarter, 0),
        NoteValue::new(BaseValue::Eighth, 0),
        NoteValue::new(BaseValue::Sixteenth, 0),
    ];
    while left_ticks > 0 {
        let Some(value) = values
            .iter()
            .find(|value: &&NoteValue| value.ticks() <= left_ticks)
        else {
            break;
        };
        elements.push(WrittenElement {
            sound: WrittenSound::Rest,
            length_ticks: value.ticks(),
            marks: ElementMarks::NONE,
            tied_to_next: false,
            octave_shift_octaves: 0,
        });
        left_ticks -= value.ticks();
    }
    WrittenBar { elements }
}

/// What is in effect from a place in a bar: a chord, or notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedContents {
    /// Ticks from the start of the bar.
    pub offset_ticks: u64,
    pub contents: ElementContents,
}

/// The harmony of one bar: what is in effect at its start, then each
/// change, in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarHarmony {
    pub changes: Vec<PlacedContents>,
}

impl BarHarmony {
    /// What is in effect `offset_ticks` into the bar.
    #[must_use]
    pub fn at(&self, offset_ticks: u64) -> Option<&ElementContents> {
        self.changes
            .iter()
            .rev()
            .find(|change: &&PlacedContents| change.offset_ticks <= offset_ticks)
            .or_else(|| self.changes.first())
            .map(|change: &PlacedContents| &change.contents)
    }
}

/// The harmony of a line that writes its own contents (a coupled line, or a
/// sample source): each bar starts with what is in effect at its start,
/// carried from the bar before when the bar writes nothing new there.
#[must_use]
pub fn harmony_of_line(line: &Line, bar_meters: &BarMeters) -> Vec<BarHarmony> {
    let bar_count: usize = usize::try_from(line.bar_count()).unwrap_or(0);
    let mut bars: Vec<BarHarmony> = vec![BarHarmony::default(); bar_count];
    let mut in_effect: Option<ElementContents> = None;
    for line_element in line.elements() {
        let (bar_index, offset_ticks): (usize, u64) =
            bar_meters.bar_at(line_element.element().start().ticks());
        let Some(bar) = bars.get_mut(bar_index) else {
            continue;
        };
        if bar.changes.is_empty()
            && offset_ticks > 0
            && let Some(carried) = &in_effect
        {
            bar.changes.push(PlacedContents {
                offset_ticks: 0,
                contents: carried.clone(),
            });
        }
        let Some(contents) = line_element.contents() else {
            continue;
        };
        if line_element.element().kind() == ElementKind::Rest {
            continue;
        }
        if in_effect.as_ref() != Some(contents) || bar.changes.is_empty() {
            bar.changes.push(PlacedContents {
                offset_ticks,
                contents: contents.clone(),
            });
            in_effect = Some(contents.clone());
        }
    }
    // A bar of rests only: the harmony before it carries through.
    let mut carried: Option<ElementContents> = None;
    for bar in &mut bars {
        if bar.changes.is_empty() {
            if let Some(contents) = &carried {
                bar.changes.push(PlacedContents {
                    offset_ticks: 0,
                    contents: contents.clone(),
                });
            }
        } else if let Some(last) = bar.changes.last() {
            carried = Some(last.contents.clone());
        }
    }
    bars
}

/// The harmony of a progression's bars laid over `bar_count` bars: the
/// progression repeats from its first bar to fill them, and the chords of a
/// bar share it evenly (Song Notation, section 7).
#[must_use]
pub fn harmony_of_progression(
    progression_bars: &[Vec<vocabulary::roman_chord::RomanChord>],
    bar_meters: &BarMeters,
    first_bar_index: usize,
    bar_count: usize,
) -> Vec<BarHarmony> {
    let mut bars: Vec<BarHarmony> = Vec::with_capacity(bar_count);
    for offset in 0..bar_count {
        let Some(chords) = progression_bars.get(offset % progression_bars.len().max(1)) else {
            bars.push(BarHarmony::default());
            continue;
        };
        let bar_length_ticks: u64 = bar_meters.bar_length_ticks(first_bar_index + offset);
        let share_count: u64 = u64::try_from(chords.len()).unwrap_or(1).max(1);
        let changes: Vec<PlacedContents> = chords
            .iter()
            .enumerate()
            .map(|(index, chord)| PlacedContents {
                // bar ticks × index ÷ chords: each chord's share of the bar.
                offset_ticks: bar_length_ticks * u64::try_from(index).unwrap_or(0) / share_count,
                contents: ElementContents::Chord(*chord),
            })
            .collect();
        bars.push(BarHarmony { changes });
    }
    bars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_bars_fill_the_meter() {
        let five_four: u64 = 5 * 5040;
        let bar: WrittenBar = rest_bar(five_four);
        let lengths: Vec<u64> = bar
            .elements
            .iter()
            .map(|element| element.length_ticks)
            .collect();
        assert_eq!(lengths, vec![20_160, 5040]);
        assert_eq!(rest_bar(3 * 5040).elements.len(), 1);
    }
}
