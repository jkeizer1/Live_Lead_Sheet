//! The library browser beside the main view (EFS-5.9): the progressions or
//! the rhythms as a tree of groups, the row under the cursor, and below the
//! list what that row is — a progression's numerals and its chords in the
//! selected entry's key, a rhythm's note values and a picture of its hits.
//! Moving the cursor previews the row on the selected bars at once.

use composer::chord_symbol::chord_symbol;
use composer::library::{LibraryRow, LibraryTab};
use composer::line_text::value_text;
use composer::session::{Pane, Session};
use composer::timeline::EntryView;
use note_spelling::note_value::NoteValue;
use vocabulary::key::Key;
use vocabulary::progression::Progression;
use vocabulary::rhythm_pattern::{AccentLevel, ElementKind, RhythmPattern};

use crate::canvas::{Align, Canvas, HitTarget, rect};
use crate::error::RenderError;
use crate::screen::count_as_f32;
use crate::screen_theme as colors;
use crate::strip_layout::ScreenRect;

const TAB_UNITS: f32 = 2.6;
const ROW_UNITS: f32 = 2.0;
const DETAIL_UNITS: f32 = 15.0;

/// Draws the panel.
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
) -> Result<(), RenderError> {
    canvas.fill(area, colors::panel());
    let browsing: bool = session.pane == Pane::Library;
    canvas.frame(
        area,
        if browsing { 2.0 } else { 1.0 },
        if browsing {
            colors::focus_edge()
        } else {
            colors::border()
        },
    );
    let width: f32 = area.right_x_pixels - area.left_x_pixels;
    // The two tabs.
    for (index, (tab, title)) in [
        (LibraryTab::Progressions, "Progressions"),
        (LibraryTab::Rhythms, "Rhythms"),
    ]
    .into_iter()
    .enumerate()
    {
        let tab_area: ScreenRect = rect(
            area.left_x_pixels + count_as_f32(index) * width / 2.0,
            area.top_y_pixels,
            width / 2.0,
            TAB_UNITS * unit,
        );
        let chosen: bool = session.tab == tab;
        canvas.fill(
            tab_area,
            if chosen {
                colors::panel_raised()
            } else {
                colors::panel()
            },
        );
        if chosen {
            canvas.fill(
                rect(
                    tab_area.left_x_pixels,
                    tab_area.bottom_y_pixels - 2.0,
                    tab_area.right_x_pixels - tab_area.left_x_pixels,
                    2.0,
                ),
                colors::selection_edge(),
            );
        }
        canvas.text_in(
            title,
            tab_area,
            1.2 * unit,
            if chosen {
                colors::text()
            } else {
                colors::text_soft()
            },
        )?;
        canvas.hit(tab_area, HitTarget::LibraryTab(tab));
    }
    let rows: Vec<LibraryRow> = session.library_rows();
    let cursor: usize = session.library_cursor();
    let list_top: f32 = area.top_y_pixels + TAB_UNITS * unit + 0.4 * unit;
    let list_bottom: f32 = area.bottom_y_pixels - DETAIL_UNITS * unit;
    let row_height: f32 = ROW_UNITS * unit;
    let visible: usize = crate::screen::whole_count((list_bottom - list_top) / row_height);
    let first: usize = cursor.saturating_sub(visible.saturating_sub(3));
    let entry: Option<&EntryView> = session.timeline().entries.get(session.selection.entry);
    let key: Option<Key> = entry.and_then(|entry| entry.key);
    for (shown, (index, row)) in rows
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .enumerate()
    {
        let top: f32 = list_top + count_as_f32(shown) * row_height;
        let row_area: ScreenRect = rect(
            area.left_x_pixels + 0.4 * unit,
            top,
            width - 0.8 * unit,
            row_height - 0.15 * unit,
        );
        let baseline: f32 = top + 1.4 * unit;
        let text_left: f32 = row_area.left_x_pixels + 0.6 * unit;
        let text_width: f32 = width - 6.0 * unit;
        match row {
            LibraryRow::Heading(title) => {
                canvas.text_fitted(
                    title,
                    text_left,
                    baseline,
                    1.0 * unit,
                    text_width + 4.0 * unit,
                    colors::text_faint(),
                )?;
            }
            LibraryRow::Progression(item) => {
                let Some(progression) = session.libraries().progression(*item) else {
                    continue;
                };
                highlight(canvas, row_area, index == cursor, browsing);
                canvas.text_fitted(
                    progression.name(),
                    text_left + unit,
                    baseline,
                    1.15 * unit,
                    text_width,
                    colors::text(),
                )?;
                let bars: String = format!("{} bars", progression.bars().len());
                canvas.text(
                    &bars,
                    row_area.right_x_pixels - 0.5 * unit,
                    baseline,
                    0.95 * unit,
                    Align::Right,
                    colors::text_soft(),
                )?;
                canvas.hit(row_area, HitTarget::LibraryRow(index));
            }
            LibraryRow::Rhythm { index: item, fits } => {
                let Some(pattern) = session.libraries().rhythm(*item) else {
                    continue;
                };
                highlight(canvas, row_area, index == cursor, browsing);
                let color: [f32; 4] = if *fits {
                    colors::text()
                } else {
                    colors::text_faint()
                };
                canvas.text_fitted(
                    pattern.name(),
                    text_left + unit,
                    baseline,
                    1.15 * unit,
                    text_width,
                    color,
                )?;
                let bars: String = format!(
                    "{} bar{}",
                    pattern.bar_count(),
                    if pattern.bar_count() == 1 { "" } else { "s" }
                );
                canvas.text(
                    &bars,
                    row_area.right_x_pixels - 0.5 * unit,
                    baseline,
                    0.95 * unit,
                    Align::Right,
                    colors::text_soft(),
                )?;
                canvas.hit(row_area, HitTarget::LibraryRow(index));
            }
        }
    }
    let detail: ScreenRect = rect(
        area.left_x_pixels + 0.4 * unit,
        list_bottom,
        width - 0.8 * unit,
        DETAIL_UNITS * unit - 0.4 * unit,
    );
    canvas.fill(detail, colors::panel_raised());
    match rows.get(cursor) {
        Some(LibraryRow::Progression(item)) => {
            if let Some(progression) = session.libraries().progression(*item) {
                draw_progression_detail(canvas, detail, unit, progression, key, session)?;
            }
        }
        Some(LibraryRow::Rhythm { index: item, .. }) => {
            if let Some(pattern) = session.libraries().rhythm(*item) {
                let group: &str = session.libraries().rhythm_group(*item).unwrap_or("");
                draw_rhythm_detail(canvas, detail, unit, pattern, group)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn highlight(canvas: &mut Canvas<'_>, row_area: ScreenRect, under_cursor: bool, browsing: bool) {
    if under_cursor {
        canvas.fill(
            row_area,
            if browsing {
                colors::selection_fill()
            } else {
                colors::panel_raised()
            },
        );
        if browsing {
            canvas.frame(row_area, 1.0, colors::selection_edge());
        }
    }
}

fn draw_progression_detail(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    progression: &Progression,
    key: Option<Key>,
    session: &Session,
) -> Result<(), RenderError> {
    let left: f32 = area.left_x_pixels + 0.8 * unit;
    let width: f32 = area.right_x_pixels - area.left_x_pixels - 1.6 * unit;
    let mut baseline: f32 = area.top_y_pixels + 1.8 * unit;
    let heading: String = format!(
        "{}   {} · {}   {}   {}/{}",
        progression.name(),
        progression.idiom(),
        progression.kind(),
        progression.mode().text(),
        progression.time_signature().pulses_per_bar(),
        progression.time_signature().pulses_per_whole_note()
    );
    canvas.text_fitted(&heading, left, baseline, 1.1 * unit, width, colors::text())?;
    let standard = session.libraries().numeral_standard();
    // Four bars to a text line, numerals over chord names, as many as fit
    // above the note at the bottom.
    for chunk in progression.bars().chunks(4) {
        if baseline + 3.7 * unit > area.bottom_y_pixels - 2.6 * unit {
            canvas.text(
                "...",
                left,
                baseline + 1.6 * unit,
                1.0 * unit,
                Align::Left,
                colors::text_soft(),
            )?;
            break;
        }
        baseline += 2.0 * unit;
        let numerals: Vec<String> = chunk
            .iter()
            .map(|bar| {
                bar.iter()
                    .map(|chord| composer::chord_symbol::numeral_text(*chord))
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .collect();
        canvas.text_fitted(
            &format!("| {} |", numerals.join(" | ")),
            left,
            baseline,
            1.05 * unit,
            width,
            colors::text_soft(),
        )?;
        if let Some(key) = key {
            baseline += 1.7 * unit;
            let names: Vec<String> = chunk
                .iter()
                .map(|bar| {
                    bar.iter()
                        .map(|chord| {
                            chord_symbol(*chord, key, standard)
                                .unwrap_or_else(|_| chord.to_string())
                        })
                        .collect::<Vec<String>>()
                        .join(" ")
                })
                .collect();
            canvas.text_fitted(
                &format!("| {} |", names.join(" | ")),
                left,
                baseline,
                1.15 * unit,
                width,
                colors::text(),
            )?;
        }
    }
    if let Some(key) = key {
        let note: String = format!("Chords shown in {}", crate::screen::key_label_of(key));
        canvas.text(
            &note,
            left,
            area.bottom_y_pixels - 0.8 * unit,
            0.9 * unit,
            Align::Left,
            colors::text_faint(),
        )?;
    }
    Ok(())
}

fn draw_rhythm_detail(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    pattern: &RhythmPattern,
    group: &str,
) -> Result<(), RenderError> {
    let left: f32 = area.left_x_pixels + 0.8 * unit;
    let width: f32 = area.right_x_pixels - area.left_x_pixels - 1.6 * unit;
    let meter = pattern.time_signature();
    let heading: String = format!(
        "{}   {group}   {}/{}",
        pattern.name(),
        meter.pulses_per_bar(),
        meter.pulses_per_whole_note()
    );
    canvas.text_fitted(
        &heading,
        left,
        area.top_y_pixels + 1.8 * unit,
        1.1 * unit,
        width,
        colors::text(),
    )?;
    // The values as the library writes them.
    let words: Vec<String> = pattern
        .elements()
        .iter()
        .map(|element| {
            let length_ticks: u64 = element.length().ticks();
            let value: String = NoteValue::from_ticks(length_ticks).map_or_else(
                || {
                    // A triplet note: written as 3/2 of what it plays.
                    NoteValue::from_ticks(length_ticks * 3 / 2).map_or_else(
                        || String::from("?"),
                        |value| format!("3:{}", value_text(value)),
                    )
                },
                value_text,
            );
            let rest: &str = if element.kind() == ElementKind::Rest {
                "r"
            } else {
                ""
            };
            let accent: &str = match element.marks().accent() {
                AccentLevel::Accent => ">",
                AccentLevel::Marcato => ">>",
                _ => "",
            };
            format!("{value}{rest}{accent}")
        })
        .collect();
    canvas.text_fitted(
        &words.join(" "),
        left,
        area.top_y_pixels + 3.8 * unit,
        1.1 * unit,
        width,
        colors::text_soft(),
    )?;
    // A picture of the hits: each sounding element a block as long as it lasts.
    let bar_ticks: u64 = meter.bar_length().ticks();
    let total_ticks: u64 = bar_ticks * u64::from(pattern.bar_count());
    let strip: ScreenRect = rect(left, area.top_y_pixels + 5.5 * unit, width, 2.4 * unit);
    canvas.frame(strip, 1.0, colors::border());
    // Song lengths in ticks are well within f32 for a proportion.
    #[allow(clippy::cast_precision_loss)]
    let x_of = |ticks: u64| -> f32 {
        strip.left_x_pixels + ticks as f32 / total_ticks.max(1) as f32 * width
    };
    for bar in 1..pattern.bar_count() {
        let x: f32 = x_of(bar_ticks * u64::from(bar));
        canvas.line(
            (x, strip.top_y_pixels),
            (x, strip.bottom_y_pixels),
            1.0,
            colors::border(),
        );
    }
    for element in pattern.elements() {
        if element.kind() == ElementKind::Rest {
            continue;
        }
        let start: f32 = x_of(element.start().ticks());
        let end: f32 = x_of(element.end_ticks());
        let accented: bool = element.marks().accent() != AccentLevel::Normal;
        let block: ScreenRect = rect(
            start + 1.0,
            strip.top_y_pixels + 0.3 * unit,
            (end - start - 2.0).max(2.0),
            1.8 * unit,
        );
        canvas.fill(
            block,
            if accented {
                colors::selection_edge()
            } else {
                colors::selection_fill()
            },
        );
    }
    canvas.text(
        "Rhythms in the selected bars' meter come first; the others are greyed.",
        left,
        area.bottom_y_pixels - 0.8 * unit,
        0.9 * unit,
        Align::Left,
        colors::text_faint(),
    )?;
    Ok(())
}
