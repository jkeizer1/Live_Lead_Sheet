//! The harmony view: one row per entry of the composition, its bars across
//! with the lead's chords at their places, named in the entry's key, and the
//! numerals beneath. A coloured band at each row's start shows its key
//! centre, so key changes and the roles of the chords read at a glance.
//! This is where a composer works the overall harmony: progressions from
//! the library, chords moved by degrees, sections moved to new keys.

use composer::session::Session;
use composer::timeline::{BarView, ChordView, EntryView, Timeline, TimelinePlace};

use crate::canvas::{Align, Canvas, HitTarget, rect};
use crate::error::RenderError;
use crate::screen::{count_as_f32, key_color, key_label, meter_label, tempo_label};
use crate::screen_theme as colors;
use crate::strip_layout::ScreenRect;

/// Most bars in a row of cells; a longer section goes on in a second row.
/// Short songs get wider cells: at least 4 across.
const MOST_BARS_PER_ROW: usize = 8;
const LEAST_BARS_PER_ROW: usize = 4;
const LABEL_UNITS: f32 = 15.0;
const CELL_ROW_UNITS: f32 = 6.2;
const ENTRY_GAP_UNITS: f32 = 0.8;
const MUTED_ROW_UNITS: f32 = 2.0;

/// Draws the view.
///
/// # Errors
///
/// When the atlas lacks a glyph.
pub fn draw(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    session: &Session,
    place: Option<TimelinePlace>,
) -> Result<(), RenderError> {
    let timeline: &Timeline = session.timeline();
    let longest: usize = timeline
        .entries
        .iter()
        .map(EntryView::bar_count)
        .max()
        .unwrap_or(0);
    let bars_per_row: usize = longest.clamp(LEAST_BARS_PER_ROW, MOST_BARS_PER_ROW);
    let cells_left: f32 = area.left_x_pixels + LABEL_UNITS * unit;
    let cell_width: f32 = (area.right_x_pixels - cells_left - unit) / count_as_f32(bars_per_row);
    let cell_height: f32 = CELL_ROW_UNITS * unit;
    // Each entry's height: as many cell rows as its bars need.
    let heights: Vec<f32> = timeline
        .entries
        .iter()
        .map(|entry: &EntryView| {
            if entry.muted {
                // A muted entry is listed, small: the musician still sees it
                // and can restore it, but it takes no room from the music.
                return MUTED_ROW_UNITS * unit;
            }
            let rows: usize = entry.bar_count().div_ceil(bars_per_row).max(1);
            count_as_f32(rows) * cell_height + ENTRY_GAP_UNITS * unit
        })
        .collect();
    // Scroll so the selected entry sits in the upper part of the view.
    let selected_top: f32 = heights[..session.selection.entry.min(heights.len())]
        .iter()
        .sum();
    let view_height: f32 = area.bottom_y_pixels - area.top_y_pixels;
    let total_height: f32 = heights.iter().sum();
    let scroll: f32 =
        (selected_top - view_height / 4.0).clamp(0.0, (total_height - view_height + unit).max(0.0));
    let mut top: f32 = area.top_y_pixels + 0.5 * unit - scroll;
    for (index, entry) in timeline.entries.iter().enumerate() {
        let height: f32 = heights[index];
        // Only rows wholly in view are drawn, so nothing spills over the
        // form strip above or the footer below.
        if top < area.top_y_pixels || top + height > area.bottom_y_pixels + ENTRY_GAP_UNITS * unit {
            top += height;
            continue;
        }
        if entry.muted {
            let row: ScreenRect = rect(
                area.left_x_pixels,
                top,
                area.right_x_pixels - area.left_x_pixels - unit,
                height - 0.3 * unit,
            );
            if index == session.selection.entry {
                canvas.fill(row, colors::selection_fill());
                canvas.frame(row, 1.0, colors::selection_edge());
            }
            let label: String = format!("{}   muted (repeat 0): M restores it", entry.section_name);
            canvas.text_fitted(
                &label,
                row.left_x_pixels + 0.6 * unit,
                top + 1.3 * unit,
                1.0 * unit,
                row.right_x_pixels - row.left_x_pixels - unit,
                colors::text_faint(),
            )?;
            canvas.hit(row, HitTarget::Entry(index));
            top += height;
            continue;
        }
        let cells: Cells = Cells {
            left: cells_left,
            width: cell_width,
            height: cell_height,
            per_row: bars_per_row,
        };
        draw_entry(canvas, area, unit, session, place, index, entry, top, cells)?;
        top += height;
    }
    Ok(())
}

/// The grid of bar cells.
#[derive(Debug, Clone, Copy)]
struct Cells {
    left: f32,
    width: f32,
    height: f32,
    per_row: usize,
}

// Each argument is a separate, named quantity of the layout. A layout
// reads top to bottom, as the screen does; split up it would scatter one
// picture over several functions.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn draw_entry(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    session: &Session,
    place: Option<TimelinePlace>,
    index: usize,
    entry: &EntryView,
    top: f32,
    cells: Cells,
) -> Result<(), RenderError> {
    let Cells {
        left: cells_left,
        width: cell_width,
        height: cell_height,
        per_row,
    } = cells;
    let left: f32 = area.left_x_pixels;
    let rows: usize = entry.bar_count().div_ceil(per_row).max(1);
    let label_area: ScreenRect = rect(
        left,
        top,
        LABEL_UNITS * unit - 0.6 * unit,
        count_as_f32(rows) * cell_height - 0.3 * unit,
    );
    let selected_entry: bool = index == session.selection.entry;
    canvas.fill(
        label_area,
        if entry.muted {
            colors::panel()
        } else {
            key_color(entry)
        },
    );
    if selected_entry {
        canvas.frame(label_area, 2.0, colors::selection_edge());
    }
    let text_color: [f32; 4] = if entry.muted {
        colors::text_faint()
    } else {
        colors::text()
    };
    let name: String = if entry.repeat_count > 1 {
        format!("{}  x{}", entry.section_name, entry.repeat_count)
    } else {
        entry.section_name.clone()
    };
    let label_width: f32 = label_area.right_x_pixels - label_area.left_x_pixels - unit;
    canvas.text_fitted(
        &name,
        left + 0.6 * unit,
        top + 1.9 * unit,
        1.35 * unit,
        label_width,
        text_color,
    )?;
    if entry.muted {
        canvas.text(
            "muted",
            left + 0.6 * unit,
            top + 3.6 * unit,
            1.05 * unit,
            Align::Left,
            colors::text_faint(),
        )?;
    } else {
        canvas.text_fitted(
            &key_label(entry),
            left + 0.6 * unit,
            top + 3.6 * unit,
            1.1 * unit,
            label_width,
            colors::text(),
        )?;
        let details: String = format!("{}  {} beats/min", meter_label(entry), tempo_label(entry));
        canvas.text_fitted(
            &details,
            left + 0.6 * unit,
            top + 5.1 * unit,
            1.0 * unit,
            label_width,
            colors::text_soft(),
        )?;
    }
    canvas.hit(label_area, HitTarget::Entry(index));
    for (bar_index, bar) in entry.bars.iter().enumerate() {
        let column: usize = bar_index % per_row;
        let row: usize = bar_index / per_row;
        let cell: ScreenRect = rect(
            cells_left + count_as_f32(column) * cell_width,
            top + count_as_f32(row) * cell_height,
            cell_width - 0.3 * unit,
            cell_height - 0.3 * unit,
        );
        let selected: bool = session.selection.contains_bar(index, bar_index);
        let playing: bool =
            place.is_some_and(|place| place.entry_index == index && place.bar_index == bar_index);
        canvas.fill(
            cell,
            if selected {
                colors::selection_fill()
            } else {
                colors::panel_raised()
            },
        );
        canvas.frame(
            cell,
            1.0,
            if selected {
                colors::selection_edge()
            } else {
                colors::border()
            },
        );
        if playing {
            canvas.frame(cell, 3.0, colors::playhead());
            if let Some(place) = place {
                // Song lengths in ticks are well within f32 for a proportion.
                #[allow(clippy::cast_precision_loss)]
                let fraction: f32 = place.offset_ticks as f32 / bar.length_ticks.max(1) as f32;
                let x: f32 =
                    cell.left_x_pixels + fraction * (cell.right_x_pixels - cell.left_x_pixels);
                canvas.line(
                    (x, cell.top_y_pixels),
                    (x, cell.bottom_y_pixels),
                    3.0,
                    colors::playhead(),
                );
            }
        }
        canvas.text(
            &format!("{}", bar_index + 1),
            cell.left_x_pixels + 0.4 * unit,
            cell.top_y_pixels + 1.2 * unit,
            0.9 * unit,
            Align::Left,
            colors::text_faint(),
        )?;
        draw_chords(canvas, cell, unit, bar, entry.muted)?;
        canvas.hit(
            cell,
            HitTarget::Bar {
                entry: index,
                bar: bar_index,
            },
        );
    }
    Ok(())
}

/// The chords of a bar at their places: symbol above, numeral below. A
/// crowded bar draws them smaller, never past the cell.
fn draw_chords(
    canvas: &mut Canvas<'_>,
    cell: ScreenRect,
    unit: f32,
    bar: &BarView,
    muted: bool,
) -> Result<(), RenderError> {
    let width: f32 = cell.right_x_pixels - cell.left_x_pixels;
    let (symbol_em, numeral_em): (f32, f32) = match bar.chords.len() {
        0 | 1 => (2.3 * unit, 1.3 * unit),
        2 => (1.9 * unit, 1.15 * unit),
        _ => (1.4 * unit, 0.95 * unit),
    };
    let symbol_color: [f32; 4] = if muted {
        colors::text_faint()
    } else {
        colors::text()
    };
    let chords: &[ChordView] = &bar.chords;
    for (index, chord) in chords.iter().enumerate() {
        // Song lengths in ticks are well within f32 for a proportion.
        #[allow(clippy::cast_precision_loss)]
        let fraction: f32 = chord.offset_ticks as f32 / bar.length_ticks.max(1) as f32;
        let x: f32 = cell.left_x_pixels + 0.6 * unit + fraction * (width - 1.2 * unit);
        let next_x: f32 = chords
            .get(index + 1)
            .map_or(cell.right_x_pixels - 0.3 * unit, |next| {
                #[allow(clippy::cast_precision_loss)]
                let next_fraction: f32 = next.offset_ticks as f32 / bar.length_ticks.max(1) as f32;
                cell.left_x_pixels + 0.6 * unit + next_fraction * (width - 1.2 * unit)
            });
        let room: f32 = (next_x - x - 0.3 * unit).max(unit);
        canvas.text_fitted(
            &chord.symbol,
            x,
            cell.top_y_pixels + 3.6 * unit,
            symbol_em,
            room,
            symbol_color,
        )?;
        canvas.text_fitted(
            &chord.numeral,
            x,
            cell.top_y_pixels + 5.3 * unit,
            numeral_em,
            room,
            colors::text_soft(),
        )?;
    }
    Ok(())
}
