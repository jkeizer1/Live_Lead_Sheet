//! The form view, zoomed all the way out: the composition as a list of its
//! entries, one row each, with what the musician arranges by — repeats,
//! key, meter, tempo, length — and the harmony in brief, so a song is
//! sequenced, its key centres planned and its tempo shaped in one place.

use composer::session::Session;
use composer::timeline::{EntryView, Timeline, TimelinePlace};

use crate::canvas::{Align, Canvas, HitTarget, rect};
use crate::error::RenderError;
use crate::screen::{count_as_f32, key_color, key_label, meter_label, tempo_label, whole_count};
use crate::screen_theme as colors;
use crate::strip_layout::ScreenRect;

/// Column starts, in units from the view's left edge.
const NUMBER_UNITS: f32 = 1.0;
const NAME_UNITS: f32 = 3.5;
const REPEAT_UNITS: f32 = 19.0;
const KEY_UNITS: f32 = 24.0;
const METER_UNITS: f32 = 36.0;
const TEMPO_UNITS: f32 = 41.0;
const BARS_UNITS: f32 = 49.0;
const CHORDS_UNITS: f32 = 58.0;
const ROW_UNITS: f32 = 3.0;
const HEADING_UNITS: f32 = 2.6;

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
    let left: f32 = area.left_x_pixels;
    let heading_baseline: f32 = area.top_y_pixels + 1.8 * unit;
    for (column, title) in [
        (NUMBER_UNITS, "#"),
        (NAME_UNITS, "Section"),
        (REPEAT_UNITS, "Plays"),
        (KEY_UNITS, "Key"),
        (METER_UNITS, "Meter"),
        (TEMPO_UNITS, "Beats/min"),
        (BARS_UNITS, "Bars"),
        (CHORDS_UNITS, "Harmony"),
    ] {
        canvas.text(
            title,
            left + column * unit,
            heading_baseline,
            1.05 * unit,
            Align::Left,
            colors::text_faint(),
        )?;
    }
    let row_height: f32 = ROW_UNITS * unit;
    let rows_top: f32 = area.top_y_pixels + HEADING_UNITS * unit;
    let visible_rows: usize = whole_count((area.bottom_y_pixels - rows_top) / row_height);
    let first_row: usize = session
        .selection
        .entry
        .saturating_sub(visible_rows.saturating_sub(2));
    for (row, (index, entry)) in timeline
        .entries
        .iter()
        .enumerate()
        .skip(first_row)
        .take(visible_rows)
        .enumerate()
    {
        let top: f32 = rows_top + count_as_f32(row) * row_height;
        let row_area: ScreenRect = rect(
            left,
            top,
            area.right_x_pixels - left,
            row_height - 0.2 * unit,
        );
        if index == session.selection.entry {
            canvas.fill(row_area, colors::selection_fill());
            canvas.frame(row_area, 1.0, colors::selection_edge());
        }
        if place.is_some_and(|place| place.entry_index == index) {
            canvas.frame(row_area, 2.0, colors::playhead());
        }
        draw_row(canvas, row_area, unit, index, entry)?;
        canvas.hit(row_area, HitTarget::Entry(index));
    }
    Ok(())
}

/// One entry's row.
// A layout reads top to bottom, as the screen does; split up it would
// scatter one picture over several functions.
#[allow(clippy::too_many_lines)]
fn draw_row(
    canvas: &mut Canvas<'_>,
    row: ScreenRect,
    unit: f32,
    index: usize,
    entry: &EntryView,
) -> Result<(), RenderError> {
    let left: f32 = row.left_x_pixels;
    let top: f32 = row.top_y_pixels;
    let em: f32 = 1.25 * unit;
    let baseline: f32 = top + 1.9 * unit;
    let color: [f32; 4] = if entry.muted {
        colors::text_faint()
    } else {
        colors::text()
    };
    canvas.text(
        &format!("{}", index + 1),
        left + NUMBER_UNITS * unit,
        baseline,
        em,
        Align::Left,
        colors::text_soft(),
    )?;
    let name: String = if entry.count_in {
        format!("{} (count-in)", entry.section_name)
    } else {
        entry.section_name.clone()
    };
    canvas.text_fitted(
        &name,
        left + NAME_UNITS * unit,
        baseline,
        em,
        (REPEAT_UNITS - NAME_UNITS - 0.5) * unit,
        color,
    )?;
    let plays: String = if entry.muted {
        String::from("muted")
    } else {
        format!("x{}", entry.repeat_count)
    };
    canvas.text(
        &plays,
        left + REPEAT_UNITS * unit,
        baseline,
        em,
        Align::Left,
        color,
    )?;
    if entry.muted {
        return Ok(());
    }
    let chip: ScreenRect = rect(
        left + KEY_UNITS * unit,
        top + 0.4 * unit,
        11.0 * unit,
        2.0 * unit,
    );
    canvas.fill(chip, key_color(entry));
    let key_written: &str = if entry.key_written_on_entry {
        "  (entry)"
    } else {
        ""
    };
    canvas.text(
        &format!("{}{key_written}", key_label(entry)),
        chip.left_x_pixels + 0.5 * unit,
        baseline,
        1.1 * unit,
        Align::Left,
        colors::text(),
    )?;
    canvas.text(
        &meter_label(entry),
        left + METER_UNITS * unit,
        baseline,
        em,
        Align::Left,
        color,
    )?;
    let tempo_written: &str = if entry.tempo_written_on_entry {
        " (entry)"
    } else {
        ""
    };
    canvas.text(
        &format!("{}{tempo_written}", tempo_label(entry)),
        left + TEMPO_UNITS * unit,
        baseline,
        em,
        Align::Left,
        color,
    )?;
    let repeat_count: usize = usize::try_from(entry.repeat_count).unwrap_or(usize::MAX);
    let bars: String = if entry.repeat_count > 1 {
        format!(
            "{} x{} = {}",
            entry.bar_count(),
            entry.repeat_count,
            entry.bar_count().saturating_mul(repeat_count)
        )
    } else {
        format!("{}", entry.bar_count())
    };
    canvas.text(
        &bars,
        left + BARS_UNITS * unit,
        baseline,
        em,
        Align::Left,
        color,
    )?;
    let chords_width: f32 = row.right_x_pixels - left - CHORDS_UNITS * unit - unit;
    canvas.text_fitted(
        &harmony_summary(entry),
        left + CHORDS_UNITS * unit,
        baseline,
        1.15 * unit,
        chords_width,
        colors::text_soft(),
    )?;
    Ok(())
}

/// The entry's chords bar by bar, `| C | Am | Dm7 | G7 |`.
#[must_use]
pub fn harmony_summary(entry: &EntryView) -> String {
    let bars: Vec<String> = entry
        .bars
        .iter()
        .map(|bar| {
            let symbols: Vec<&str> = bar
                .chords
                .iter()
                .map(|chord| chord.symbol.as_str())
                .collect();
            if symbols.is_empty() {
                String::from("-")
            } else {
                symbols.join(" ")
            }
        })
        .collect();
    if bars.is_empty() {
        String::new()
    } else {
        format!("| {} |", bars.join(" | "))
    }
}
