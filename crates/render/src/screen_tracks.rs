//! The tracks view: the selected section, every voice track as a lane
//! across its bars. Each element is a block as long as it lasts, labelled
//! with what it plays. A coupled line is one lane; an uncoupled one shows
//! its two streams apart — the sample source's chords above, the rhythm's
//! gates below — so pitch and rhythm can be composed separately and seen
//! together. The lead's chord symbols run along the top.
//!
//! As on the staff (EFS-1.12), tracks that do not fit are hidden, never
//! squeezed: the selected track and the lead first, then the others in the
//! song's order, and the hidden ones are named. Rows of bars that do not
//! fit are not drawn; the view keeps the selected bar's row in view and
//! names the bars it leaves out.

use composer::session::{Mode, Session};
use composer::timeline::{BarView, ElementView, Timeline, TimelinePlace, TrackBarView, TrackView};
use vocabulary::rhythm_pattern::AccentLevel;
use vocabulary::song::VoiceKind;

use crate::canvas::{Align, Canvas, HitTarget, rect};
use crate::error::RenderError;
use crate::screen::{count_as_f32, key_label, meter_label, tempo_label};
use crate::screen_theme as colors;
use crate::strip_layout::ScreenRect;

/// Bars across: the section's own count, from 4 to 8.
const MOST_BARS_PER_ROW: usize = 8;
const LEAST_BARS_PER_ROW: usize = 4;
const LABEL_UNITS: f32 = 11.0;
const TITLE_UNITS: f32 = 3.0;
const CHORD_ROW_UNITS: f32 = 2.6;
const LANE_UNITS: f32 = 3.4;
const SOURCE_LANE_UNITS: f32 = 2.4;
const ROW_GAP_UNITS: f32 = 1.2;
/// Room at the bottom for the note naming what is hidden.
const HIDDEN_NOTE_UNITS: f32 = 1.6;

/// Draws the view.
///
/// # Errors
///
/// When the atlas lacks a glyph.
// A layout reads top to bottom, as the screen does; split up it would
// scatter one picture over several functions.
#[allow(clippy::too_many_lines)]
pub fn draw(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    session: &Session,
    place: Option<TimelinePlace>,
) -> Result<(), RenderError> {
    let timeline: &Timeline = session.timeline();
    let Some(entry) = timeline.entries.get(session.selection.entry) else {
        return Ok(());
    };
    let left: f32 = area.left_x_pixels;
    let title: String = format!(
        "{}   {}   {}   {} beats/min   {} bars",
        entry.section_name,
        key_label(entry),
        meter_label(entry),
        tempo_label(entry),
        entry.bar_count()
    );
    canvas.text(
        &title,
        left + unit,
        area.top_y_pixels + 2.0 * unit,
        1.4 * unit,
        Align::Left,
        colors::text(),
    )?;
    if session.mode == Mode::SongHarmony {
        canvas.text(
            "Song Harmony: edits go to the lead track",
            area.right_x_pixels - unit,
            area.top_y_pixels + 2.0 * unit,
            1.0 * unit,
            Align::Right,
            colors::text_faint(),
        )?;
    }
    let uncoupled: Vec<bool> = (0..timeline.tracks.len())
        .map(|track| {
            entry.bars.iter().any(|bar| {
                bar.tracks
                    .get(track)
                    .is_some_and(|track_bar| track_bar.sampled)
            })
        })
        .collect();
    let lane_height = |track_index: usize| -> f32 {
        if uncoupled[track_index] {
            (SOURCE_LANE_UNITS + LANE_UNITS) * unit
        } else {
            LANE_UNITS * unit
        }
    };
    // Which tracks fit in one row of bars, the selected track and the lead
    // first. A hidden track's lane has no height.
    let rows_top: f32 = area.top_y_pixels + TITLE_UNITS * unit;
    let rows_bottom: f32 = area.bottom_y_pixels - HIDDEN_NOTE_UNITS * unit;
    let mut budget: f32 = rows_bottom - rows_top - (CHORD_ROW_UNITS + ROW_GAP_UNITS) * unit;
    let target_track: usize = if session.mode == Mode::SongHarmony {
        timeline
            .tracks
            .iter()
            .position(|track: &TrackView| track.lead)
            .unwrap_or(session.selection.track)
    } else {
        session.selection.track
    };
    let mut priority: Vec<usize> = (0..timeline.tracks.len()).collect();
    priority.sort_by_key(|index: &usize| {
        (
            *index != target_track,
            !timeline.tracks[*index].lead,
            *index,
        )
    });
    let mut shown: Vec<bool> = vec![false; timeline.tracks.len()];
    let mut hidden_names: Vec<&str> = Vec::new();
    for index in priority {
        let height: f32 = lane_height(index);
        if height <= budget {
            budget -= height;
            shown[index] = true;
        } else {
            hidden_names.push(&timeline.tracks[index].name);
        }
    }
    let lane_heights: Vec<f32> = (0..timeline.tracks.len())
        .map(|index: usize| {
            if shown[index] {
                lane_height(index)
            } else {
                0.0
            }
        })
        .collect();
    let row_height: f32 =
        CHORD_ROW_UNITS * unit + lane_heights.iter().sum::<f32>() + ROW_GAP_UNITS * unit;
    let bars_per_row: usize = entry
        .bar_count()
        .clamp(LEAST_BARS_PER_ROW, MOST_BARS_PER_ROW);
    let cells_left: f32 = left + LABEL_UNITS * unit;
    let bar_width: f32 = (area.right_x_pixels - cells_left - unit) / count_as_f32(bars_per_row);
    let rows: usize = entry.bar_count().div_ceil(bars_per_row).max(1);
    // Whole rows only, at least one; the selected bar's row stays in view.
    let rows_in_view: usize =
        crate::screen::whole_count((rows_bottom - rows_top + ROW_GAP_UNITS * unit) / row_height)
            .clamp(1, rows);
    let selected_row: usize = session.selection.first_bar / bars_per_row;
    let first_row: usize = selected_row
        .saturating_sub(rows_in_view - 1)
        .min(rows - rows_in_view);
    let mut hidden: Vec<String> = hidden_names.iter().map(ToString::to_string).collect();
    let bar_count: usize = entry.bar_count();
    if first_row > 0 {
        hidden.push(format!("bars 1–{}", first_row * bars_per_row));
    }
    let after_bar: usize = (first_row + rows_in_view) * bars_per_row;
    if after_bar < bar_count {
        hidden.push(format!("bars {}–{bar_count}", after_bar + 1));
    }
    if !hidden.is_empty() {
        canvas.text(
            &format!("Hidden for room: {}", hidden.join(", ")),
            area.right_x_pixels - unit,
            area.bottom_y_pixels - 0.6 * unit,
            1.0 * unit,
            Align::Right,
            colors::problem(),
        )?;
    }
    for row in first_row..first_row + rows_in_view {
        let row_top: f32 = rows_top + count_as_f32(row - first_row) * row_height;
        // Track names down the left.
        let mut lane_top: f32 = row_top + CHORD_ROW_UNITS * unit;
        for (track_index, track) in timeline.tracks.iter().enumerate() {
            if !shown[track_index] {
                continue;
            }
            let height: f32 = lane_heights[track_index];
            let label_area: ScreenRect = rect(
                left,
                lane_top,
                LABEL_UNITS * unit - 0.5 * unit,
                height - 0.3 * unit,
            );
            let selected: bool = track_index == session.selection.track
                && session.mode != Mode::SongHarmony
                || track.lead && session.mode == Mode::SongHarmony;
            canvas.fill(
                label_area,
                if selected {
                    colors::selection_fill()
                } else {
                    colors::panel_raised()
                },
            );
            draw_track_label(canvas, label_area, unit, track, uncoupled[track_index])?;
            canvas.hit(label_area, HitTarget::Track(track_index));
            // The form line of the label is the control that changes it.
            canvas.hit(
                rect(
                    label_area.left_x_pixels,
                    label_area.top_y_pixels + 2.0 * unit,
                    label_area.right_x_pixels - label_area.left_x_pixels,
                    1.2 * unit,
                ),
                HitTarget::LineForm(track_index),
            );
            lane_top += height;
        }
        for column in 0..bars_per_row {
            let bar_index: usize = row * bars_per_row + column;
            let Some(bar) = entry.bars.get(bar_index) else {
                break;
            };
            let bar_left: f32 = cells_left + count_as_f32(column) * bar_width;
            draw_bar(
                canvas,
                session,
                place,
                bar_index,
                bar,
                bar_left,
                bar_width,
                row_top,
                unit,
                &lane_heights,
                &uncoupled,
                &shown,
            )?;
        }
    }
    Ok(())
}

fn draw_track_label(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    track: &TrackView,
    uncoupled: bool,
) -> Result<(), RenderError> {
    let width: f32 = area.right_x_pixels - area.left_x_pixels - unit;
    canvas.text_fitted(
        &track.name,
        area.left_x_pixels + 0.5 * unit,
        area.top_y_pixels + 1.6 * unit,
        1.2 * unit,
        width,
        colors::text(),
    )?;
    let kind: &str = match (track.voice, track.lead) {
        (VoiceKind::Poly, true) => "poly, lead",
        (VoiceKind::Poly, false) => "poly",
        (VoiceKind::Mono, _) => "mono",
    };
    let detail: String = if uncoupled {
        format!("{kind}, sampled")
    } else {
        format!("{kind}, coupled")
    };
    canvas.text_fitted(
        &detail,
        area.left_x_pixels + 0.5 * unit,
        area.top_y_pixels + 2.8 * unit,
        0.9 * unit,
        width,
        colors::text_soft(),
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)] // each is a separate, named quantity of the layout
fn draw_bar(
    canvas: &mut Canvas<'_>,
    session: &Session,
    place: Option<TimelinePlace>,
    bar_index: usize,
    bar: &BarView,
    bar_left: f32,
    bar_width: f32,
    row_top: f32,
    unit: f32,
    lane_heights: &[f32],
    uncoupled: &[bool],
    shown: &[bool],
) -> Result<(), RenderError> {
    let entry_index: usize = session.selection.entry;
    let total_height: f32 = CHORD_ROW_UNITS * unit + lane_heights.iter().sum::<f32>();
    let bar_area: ScreenRect = rect(bar_left, row_top, bar_width - 0.2 * unit, total_height);
    if session.selection.contains_bar(entry_index, bar_index) {
        canvas.fill(bar_area, colors::selection_fill());
    }
    canvas.line(
        (bar_left, row_top),
        (bar_left, row_top + total_height),
        1.0,
        colors::border(),
    );
    canvas.text(
        &format!("{}", bar_index + 1),
        bar_left + 0.3 * unit,
        row_top + 1.0 * unit,
        0.85 * unit,
        Align::Left,
        colors::text_faint(),
    )?;
    let ticks_to_x = |offset_ticks: u64| -> f32 {
        // Song lengths in ticks are well within f32 for a proportion.
        #[allow(clippy::cast_precision_loss)]
        let fraction: f32 = offset_ticks as f32 / bar.length_ticks.max(1) as f32;
        bar_left + 0.3 * unit + fraction * (bar_width - 0.8 * unit)
    };
    for chord in &bar.chords {
        canvas.text(
            &chord.symbol,
            ticks_to_x(chord.offset_ticks),
            row_top + 2.2 * unit,
            1.3 * unit,
            Align::Left,
            colors::text(),
        )?;
    }
    canvas.hit(
        rect(bar_left, row_top, bar_width, CHORD_ROW_UNITS * unit),
        HitTarget::Bar {
            entry: entry_index,
            bar: bar_index,
        },
    );
    let mut lane_top: f32 = row_top + CHORD_ROW_UNITS * unit;
    for (track_index, track_bar) in bar.tracks.iter().enumerate() {
        if !shown.get(track_index).copied().unwrap_or(false) {
            continue;
        }
        let height: f32 = lane_heights
            .get(track_index)
            .copied()
            .unwrap_or(LANE_UNITS * unit);
        let split: bool = uncoupled.get(track_index).copied().unwrap_or(false);
        let mut gate_top: f32 = lane_top;
        if split {
            draw_source(canvas, track_bar, &ticks_to_x, lane_top, unit)?;
            gate_top += SOURCE_LANE_UNITS * unit;
        }
        draw_gates(
            canvas,
            session,
            entry_index,
            bar_index,
            track_index,
            track_bar,
            &ticks_to_x,
            gate_top,
            unit,
        )?;
        lane_top += height;
    }
    if let Some(place) = place
        && place.entry_index == entry_index
        && place.bar_index == bar_index
    {
        let x: f32 = ticks_to_x(place.offset_ticks);
        canvas.line(
            (x, row_top),
            (x, row_top + total_height),
            3.0,
            colors::playhead(),
        );
    }
    Ok(())
}

fn draw_source(
    canvas: &mut Canvas<'_>,
    track_bar: &TrackBarView,
    ticks_to_x: &dyn Fn(u64) -> f32,
    top: f32,
    unit: f32,
) -> Result<(), RenderError> {
    for source in &track_bar.source {
        let start: f32 = ticks_to_x(source.offset_ticks);
        let end: f32 = ticks_to_x(source.offset_ticks + source.length_ticks);
        let block: ScreenRect = rect(
            start,
            top + 0.2 * unit,
            (end - start - 0.15 * unit).max(2.0),
            SOURCE_LANE_UNITS * unit - 0.5 * unit,
        );
        canvas.frame(block, 1.0, colors::preview());
        canvas.text_fitted(
            &source.label,
            start + 0.3 * unit,
            top + 1.45 * unit,
            1.0 * unit,
            end - start - 0.6 * unit,
            colors::done(),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // each is a separate, named quantity of the layout
fn draw_gates(
    canvas: &mut Canvas<'_>,
    session: &Session,
    entry_index: usize,
    bar_index: usize,
    track_index: usize,
    track_bar: &TrackBarView,
    ticks_to_x: &dyn Fn(u64) -> f32,
    top: f32,
    unit: f32,
) -> Result<(), RenderError> {
    if !track_bar.has_line {
        canvas.text(
            "rests",
            ticks_to_x(0) + 0.3 * unit,
            top + 2.0 * unit,
            0.9 * unit,
            Align::Left,
            colors::text_faint(),
        )?;
        return Ok(());
    }
    for element in &track_bar.elements {
        let start: f32 = ticks_to_x(element.offset_ticks);
        let end: f32 = ticks_to_x(element.offset_ticks + element.length_ticks);
        let block: ScreenRect = rect(
            start,
            top + 0.3 * unit,
            (end - start - 0.15 * unit).max(2.0),
            LANE_UNITS * unit - 0.8 * unit,
        );
        let selected: bool = session.selection.track == track_index
            && session.selection.element == element.line_element_index
            && session.selection.contains_bar(entry_index, bar_index);
        draw_element(canvas, element, block, unit, selected)?;
        canvas.hit(
            block,
            HitTarget::Element {
                entry: entry_index,
                bar: bar_index,
                track: track_index,
                element: element.line_element_index,
                // The tracks view shows a repeated entry once.
                pass_number: 1,
            },
        );
    }
    Ok(())
}

fn draw_element(
    canvas: &mut Canvas<'_>,
    element: &ElementView,
    block: ScreenRect,
    unit: f32,
    selected: bool,
) -> Result<(), RenderError> {
    let width: f32 = block.right_x_pixels - block.left_x_pixels;
    if element.rest {
        canvas.frame(block, 1.0, colors::border());
        return Ok(());
    }
    canvas.fill(
        block,
        if selected {
            colors::selection_edge()
        } else {
            colors::panel_raised()
        },
    );
    canvas.frame(
        block,
        1.0,
        if element.from_source {
            colors::preview()
        } else {
            colors::text_faint()
        },
    );
    let accent: &str = match element.marks.accent() {
        AccentLevel::Accent => ">",
        AccentLevel::Marcato => ">>",
        AccentLevel::Normal | AccentLevel::Soft | AccentLevel::Softer => "",
    };
    let label: String = format!("{accent}{}", element.label);
    let color: [f32; 4] = if element.from_source {
        colors::done()
    } else {
        colors::text()
    };
    canvas.text_fitted(
        &label,
        block.left_x_pixels + 0.3 * unit,
        block.top_y_pixels + 1.7 * unit,
        1.05 * unit,
        width - 0.5 * unit,
        color,
    )?;
    if element.tied_to_next {
        let y: f32 = block.bottom_y_pixels - 0.3 * unit;
        canvas.line(
            (block.right_x_pixels - 0.6 * unit, y),
            (block.right_x_pixels + 0.6 * unit, y),
            2.0,
            colors::text_soft(),
        );
    }
    Ok(())
}
