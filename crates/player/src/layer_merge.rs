//! One note for the same pitch in two voice layers (Song Notation, section
//! 5, "Voice layers"): a note on at the first start, no second note on while
//! it sounds, a note off when the longest ends.
//!
//! Only notes of the same pitch, on the same track, from different voice
//! layers, that overlap in time merge. Notes that only touch (one ends where
//! the other starts) follow the existing rules for touching notes.

use std::collections::BTreeMap;

use crate::note_track::TimedNote;

/// A planned note and the voice layers it comes from. A merged note comes
/// from every layer of the notes it was made of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerNote {
    layers: Vec<u8>,
    note: TimedNote,
}

impl LayerNote {
    /// A note planned from the voice layer `layer_number`.
    #[must_use]
    pub fn new(layer_number: u8, note: TimedNote) -> LayerNote {
        LayerNote {
            layers: vec![layer_number],
            note,
        }
    }

    /// Whether two notes may merge: the same pitch, overlapping in time, and
    /// not both from the same single layer (a layer's own notes never merge).
    fn merges_with(&self, other: &LayerNote) -> bool {
        let same_single_layer: bool = self.layers.len() == 1 && self.layers == other.layers;
        self.note.note == other.note.note
            && !same_single_layer
            && self.note.start_ticks < end_ticks(&other.note)
            && other.note.start_ticks < end_ticks(&self.note)
    }

    /// The merged note: from the earlier start to the later end. It keeps the
    /// velocity of the note that starts first, and the legato join of the
    /// note that ends last, because that is the note the next one follows.
    fn merged_with(&self, other: &LayerNote) -> LayerNote {
        let (first, second): (&LayerNote, &LayerNote) =
            if other.note.start_ticks < self.note.start_ticks {
                (other, self)
            } else {
                (self, other)
            };
        let start_ticks: u64 = first.note.start_ticks;
        let last_end_ticks: u64 = end_ticks(&first.note).max(end_ticks(&second.note));
        let ends_last: &LayerNote = if end_ticks(&second.note) > end_ticks(&first.note) {
            second
        } else {
            first
        };
        let mut layers: Vec<u8> = first.layers.clone();
        for layer in &second.layers {
            if !layers.contains(layer) {
                layers.push(*layer);
            }
        }
        LayerNote {
            layers,
            note: TimedNote {
                start_ticks,
                // ticks − ticks → ticks.
                length_ticks: last_end_ticks - start_ticks,
                note: first.note.note,
                velocity: first.note.velocity,
                legato_to_next: ends_last.note.legato_to_next,
            },
        }
    }
}

/// Where a note ends. Lengths and starts are far below the limit of a `u64`,
/// so the sum cannot overflow in practice; a note that does overflow ends at
/// the limit.
fn end_ticks(note: &TimedNote) -> u64 {
    note.start_ticks.saturating_add(note.length_ticks)
}

/// The notes of a track with equal pitches of different layers merged, in
/// the order the notes were planned (a merged note takes the place of the
/// first note it was made of).
#[must_use]
pub fn merge_equal_pitches_across_layers(notes: Vec<LayerNote>) -> Vec<TimedNote> {
    // Only notes of one pitch can merge, so each pitch is merged on its own.
    let mut by_pitch: BTreeMap<u8, Vec<(usize, LayerNote)>> = BTreeMap::new();
    for (index, layer_note) in notes.into_iter().enumerate() {
        by_pitch
            .entry(layer_note.note.note.value())
            .or_default()
            .push((index, layer_note));
    }
    let mut merged: Vec<(usize, LayerNote)> = Vec::new();
    for (_, mut group) in by_pitch {
        // Merge until no pair is left: a merged note may now overlap a third
        // layer's note. Each merge removes a note, so this ends.
        'again: loop {
            for first_index in 0..group.len() {
                for second_index in (first_index + 1)..group.len() {
                    if group[first_index].1.merges_with(&group[second_index].1) {
                        let combined: LayerNote =
                            group[first_index].1.merged_with(&group[second_index].1);
                        let place: usize = group[first_index].0.min(group[second_index].0);
                        group[first_index] = (place, combined);
                        group.remove(second_index);
                        continue 'again;
                    }
                }
            }
            break;
        }
        merged.extend(group);
    }
    merged.sort_by_key(|&(index, _): &(usize, LayerNote)| index);
    merged
        .into_iter()
        .map(|(_, layer_note): (usize, LayerNote)| layer_note.note)
        .collect()
}
