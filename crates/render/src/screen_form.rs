//! F1 Arrange Song Sections: the `song_composition` block as rows, one
//! per line of it in the order written. The song's start (key, meter and
//! tempo), its key and tempo changes, and its entries (section, plays, key,
//! meter, tempo). Every cell shown in the text colour is what the block
//! writes and can be edited; to the right, in the derived colour, are what
//! follows from it and is not typed here: each entry's bars and harmony.
//! Its dialogs (key, meter, tempo, name, new section) are drawn on top.

use composer::arrange_editor::{
    ArrangeDialog, ArrangeEditor, ArrangeTarget, FormField, KeyChoice, METER_PRESETS, MOST_FLATS,
    MOST_SHARPS, MeterChoice, NewSectionForm, key_of, meter_text,
};
use composer::arrangement::{ArrangeColumn, ArrangeRow, ChangeRow, EntryRow, Origin, StartRow};
use composer::chord_symbol::note_name_text;
use composer::key_signature::KeySignature;
use composer::session::Session;
use composer::text_field::TextField;
use composer::timeline::{EntryView, Timeline, TimelinePlace};
use vocabulary::key::{Key, Mode};

use crate::canvas::{Align, Canvas, HitTarget, rect};
use crate::error::RenderError;
use crate::screen::{count_as_f32, key_label_of, meter_label, tempo_label, whole_count};
use crate::screen_theme as colors;
use crate::strip_layout::{ScreenRect, TEXT_CAP_HEIGHT_EMS};

/// Column starts, in units from the view's left edge.
const NUMBER_UNITS: f32 = 1.0;
const NAME_UNITS: f32 = 3.5;
const PLAYS_UNITS: f32 = 20.0;
const KEY_UNITS: f32 = 26.0;
const METER_UNITS: f32 = 40.0;
const TEMPO_UNITS: f32 = 46.0;
/// Where the derived values start, past a dividing line.
const DERIVED_UNITS: f32 = 60.0;
const BARS_UNITS: f32 = 61.0;
const HARMONY_UNITS: f32 = 69.0;
const ROW_UNITS: f32 = 3.0;
const HEADING_UNITS: f32 = 2.6;

/// The cells' left edges and widths, in units.
fn cell_span(column: ArrangeColumn) -> (f32, f32) {
    match column {
        ArrangeColumn::Name => (NAME_UNITS - 0.5, PLAYS_UNITS - NAME_UNITS),
        ArrangeColumn::Plays => (PLAYS_UNITS - 0.5, KEY_UNITS - PLAYS_UNITS),
        ArrangeColumn::Key => (KEY_UNITS - 0.5, METER_UNITS - KEY_UNITS),
        ArrangeColumn::Meter => (METER_UNITS - 0.5, TEMPO_UNITS - METER_UNITS),
        ArrangeColumn::Tempo => (TEMPO_UNITS - 0.5, DERIVED_UNITS - TEMPO_UNITS),
    }
}

/// Draws the rows.
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
    let rows: Vec<ArrangeRow> = session.arrange_rows();
    let editor: &ArrangeEditor = session.arrange();
    let left: f32 = area.left_x_pixels;
    let heading_baseline: f32 = area.top_y_pixels + 1.8 * unit;
    for (column, title, color) in [
        (NUMBER_UNITS, "#", colors::text_faint()),
        (NAME_UNITS, "Section", colors::text_faint()),
        (PLAYS_UNITS, "Plays", colors::text_faint()),
        (KEY_UNITS, "Key", colors::text_faint()),
        (METER_UNITS, "Meter", colors::text_faint()),
        (TEMPO_UNITS, "Beats/min", colors::text_faint()),
        (BARS_UNITS, "Bars", colors::derived_note()),
        (HARMONY_UNITS, "Harmony (derived)", colors::derived_note()),
    ] {
        canvas.text(
            title,
            left + column * unit,
            heading_baseline,
            1.05 * unit,
            Align::Left,
            color,
        )?;
    }
    let row_height: f32 = ROW_UNITS * unit;
    let rows_top: f32 = area.top_y_pixels + HEADING_UNITS * unit;
    let divider_x: f32 = left + DERIVED_UNITS * unit - 0.5 * unit;
    canvas.line(
        (divider_x, area.top_y_pixels + 0.5 * unit),
        (divider_x, area.bottom_y_pixels - 0.5 * unit),
        1.0,
        colors::border(),
    );
    let visible_rows: usize = whole_count((area.bottom_y_pixels - rows_top) / row_height);
    let first_row: usize = editor.row.saturating_sub(visible_rows.saturating_sub(2));
    let mut entry_number: usize = rows[..first_row.min(rows.len())]
        .iter()
        .filter(|row: &&ArrangeRow| matches!(row, ArrangeRow::Entry(_)))
        .count();
    for (shown, (index, row)) in rows
        .iter()
        .enumerate()
        .skip(first_row)
        .take(visible_rows)
        .enumerate()
    {
        let top: f32 = rows_top + count_as_f32(shown) * row_height;
        let row_area: ScreenRect = rect(
            left,
            top,
            area.right_x_pixels - left,
            row_height - 0.2 * unit,
        );
        if index == editor.row {
            canvas.fill(row_area, colors::selection_fill());
        }
        let drawn: RowPlace = RowPlace {
            area: row_area,
            unit,
        };
        match row {
            ArrangeRow::Start(start) => draw_start(canvas, &drawn, start)?,
            ArrangeRow::Change(change) => draw_change(canvas, &drawn, change)?,
            ArrangeRow::Entry(entry) => {
                entry_number += 1;
                if let Some(view) = timeline.entries.get(entry.entry_index) {
                    draw_entry(canvas, &drawn, entry_number, entry, view)?;
                    if place.is_some_and(|place| place.entry_index == entry.entry_index) {
                        canvas.frame(row_area, 2.0, colors::playhead());
                    }
                }
            }
        }
        for column in row.columns() {
            let (start_units, width_units) = cell_span(*column);
            let cell: ScreenRect = rect(
                left + start_units * unit,
                top,
                width_units * unit,
                row_height - 0.2 * unit,
            );
            if index == editor.row && *column == editor.column {
                canvas.frame(cell, 2.0, colors::focus_edge());
            }
            canvas.hit(
                cell,
                HitTarget::Arrange(ArrangeTarget::Cell {
                    row: index,
                    column: *column,
                }),
            );
        }
    }
    Ok(())
}

/// The song's start: its key, meter and tempo.
fn draw_start(
    canvas: &mut Canvas<'_>,
    drawn: &RowPlace,
    start: &StartRow,
) -> Result<(), RenderError> {
    draw_label(canvas, drawn, "Song starts in", colors::text_soft())?;
    draw_key_cell(
        canvas,
        drawn,
        start
            .key_words
            .as_deref()
            .and_then(composer::arrange_editor::key_from_words),
        None,
    )?;
    draw_cell_text(
        canvas,
        drawn,
        ArrangeColumn::Meter,
        start.meter_words.as_deref().unwrap_or("?"),
        colors::text(),
    )?;
    draw_cell_text(
        canvas,
        drawn,
        ArrangeColumn::Tempo,
        &tempo_number(start.tempo_words.as_deref()),
        colors::text(),
    )?;
    Ok(())
}

/// A key or tempo change line: what it changes, or "same".
fn draw_change(
    canvas: &mut Canvas<'_>,
    drawn: &RowPlace,
    change: &ChangeRow,
) -> Result<(), RenderError> {
    draw_label(canvas, drawn, "From here on", colors::text_soft())?;
    match change
        .key_words
        .as_deref()
        .and_then(composer::arrange_editor::key_from_words)
    {
        Some(key) => draw_key_cell(canvas, drawn, Some(key), None)?,
        None => draw_cell_text(
            canvas,
            drawn,
            ArrangeColumn::Key,
            "same key",
            colors::text_faint(),
        )?,
    }
    match change.tempo_words.as_deref() {
        Some(words) => draw_cell_text(
            canvas,
            drawn,
            ArrangeColumn::Tempo,
            &tempo_number(Some(words)),
            colors::text(),
        )?,
        None => draw_cell_text(
            canvas,
            drawn,
            ArrangeColumn::Tempo,
            "same tempo",
            colors::text_faint(),
        )?,
    }
    Ok(())
}

/// Where a row is drawn.
struct RowPlace {
    area: ScreenRect,
    unit: f32,
}

impl RowPlace {
    fn baseline(&self) -> f32 {
        self.area.top_y_pixels + 1.9 * self.unit
    }

    fn x(&self, column_units: f32) -> f32 {
        self.area.left_x_pixels + column_units * self.unit
    }
}

fn draw_label(
    canvas: &mut Canvas<'_>,
    row: &RowPlace,
    text: &str,
    color: [f32; 4],
) -> Result<(), RenderError> {
    canvas.text_fitted(
        text,
        row.x(NAME_UNITS),
        row.baseline(),
        1.2 * row.unit,
        (PLAYS_UNITS - NAME_UNITS - 0.5) * row.unit,
        color,
    )
}

fn column_units(column: ArrangeColumn) -> f32 {
    match column {
        ArrangeColumn::Name => NAME_UNITS,
        ArrangeColumn::Plays => PLAYS_UNITS,
        ArrangeColumn::Key => KEY_UNITS,
        ArrangeColumn::Meter => METER_UNITS,
        ArrangeColumn::Tempo => TEMPO_UNITS,
    }
}

fn draw_cell_text(
    canvas: &mut Canvas<'_>,
    row: &RowPlace,
    column: ArrangeColumn,
    text: &str,
    color: [f32; 4],
) -> Result<(), RenderError> {
    let (_, width_units) = cell_span(column);
    canvas.text_fitted(
        text,
        row.x(column_units(column)),
        row.baseline(),
        1.2 * row.unit,
        (width_units - 1.0) * row.unit,
        color,
    )
}

/// The cell's text, giving its width so a faint word can follow it.
fn draw_cell_width(
    canvas: &mut Canvas<'_>,
    row: &RowPlace,
    column: ArrangeColumn,
    text: &str,
    color: [f32; 4],
) -> Result<f32, RenderError> {
    canvas.text(
        text,
        row.x(column_units(column)),
        row.baseline(),
        1.2 * row.unit,
        Align::Left,
        color,
    )
}

/// The key in its colour chip, with where it comes from in faint words.
fn draw_key_cell(
    canvas: &mut Canvas<'_>,
    row: &RowPlace,
    key: Option<Key>,
    origin: Option<&str>,
) -> Result<(), RenderError> {
    let Some(key) = key else {
        return draw_cell_text(
            canvas,
            row,
            ArrangeColumn::Key,
            "no key",
            colors::text_faint(),
        );
    };
    let unit: f32 = row.unit;
    let label: String = key_label_of(key);
    let em: f32 = 1.15 * unit;
    let chip: ScreenRect = rect(
        row.x(KEY_UNITS) - 0.3 * unit,
        row.area.top_y_pixels + 0.4 * unit,
        canvas.text_width(&label, em) + 1.0 * unit,
        2.0 * unit,
    );
    canvas.fill(chip, key_chip_color(key));
    canvas.text(
        &label,
        chip.left_x_pixels + 0.5 * unit,
        row.baseline(),
        em,
        Align::Left,
        colors::text(),
    )?;
    if let Some(origin) = origin {
        canvas.text(
            origin,
            chip.right_x_pixels + 0.5 * unit,
            row.baseline(),
            0.95 * unit,
            Align::Left,
            colors::text_faint(),
        )?;
    }
    Ok(())
}

fn key_chip_color(key: Key) -> [f32; 4] {
    colors::key_hue(KeySignature::of(key).fifths(), key.mode() == Mode::Minor)
}

/// `100` from `100 beats/min`; other units as written.
fn tempo_number(words: Option<&str>) -> String {
    match words {
        Some(words) => words
            .strip_suffix(" beats/min")
            .unwrap_or(words)
            .to_string(),
        None => String::from("?"),
    }
}

/// Where an inherited value comes from, in faint words.
fn origin_words(origin: Origin) -> &'static str {
    match origin {
        Origin::Entry => "entry",
        Origin::Section => "section",
        Origin::Composition => "",
    }
}

fn draw_entry(
    canvas: &mut Canvas<'_>,
    row: &RowPlace,
    number: usize,
    entry: &EntryRow,
    view: &EntryView,
) -> Result<(), RenderError> {
    let unit: f32 = row.unit;
    let color: [f32; 4] = if view.muted {
        colors::text_faint()
    } else {
        colors::text()
    };
    canvas.text(
        &format!("{number}"),
        row.x(NUMBER_UNITS),
        row.baseline(),
        1.2 * unit,
        Align::Left,
        colors::text_soft(),
    )?;
    let name: String = if view.count_in {
        format!("{} (count-in)", entry.section_name)
    } else {
        entry.section_name.clone()
    };
    draw_cell_text(canvas, row, ArrangeColumn::Name, &name, color)?;
    let plays: String = if view.muted {
        String::from("left out")
    } else {
        format!("x{}", view.repeat_count)
    };
    draw_cell_text(canvas, row, ArrangeColumn::Plays, &plays, color)?;
    if view.muted && view.bars.is_empty() {
        return Ok(());
    }
    draw_key_cell(canvas, row, view.key, Some(origin_words(entry.key_origin)))?;
    // A meter F1 may not change (the section has music) is drawn softer.
    let meter_color: [f32; 4] = if entry.meter_editable {
        color
    } else {
        colors::text_soft()
    };
    draw_cell_text(
        canvas,
        row,
        ArrangeColumn::Meter,
        &meter_label(view),
        meter_color,
    )?;
    let tempo_width: f32 =
        draw_cell_width(canvas, row, ArrangeColumn::Tempo, &tempo_label(view), color)?;
    canvas.text(
        origin_words(entry.tempo_origin),
        row.x(TEMPO_UNITS) + tempo_width + 0.8 * unit,
        row.baseline(),
        0.95 * unit,
        Align::Left,
        colors::text_faint(),
    )?;
    // Derived: what follows from the section, not typed here.
    let bars: String = if view.repeat_count > 1 {
        format!("{} x{}", view.bar_count(), view.repeat_count)
    } else {
        format!("{}", view.bar_count())
    };
    canvas.text(
        &bars,
        row.x(BARS_UNITS),
        row.baseline(),
        1.15 * unit,
        Align::Left,
        colors::derived_note(),
    )?;
    let harmony_width: f32 = row.area.right_x_pixels - row.x(HARMONY_UNITS) - unit;
    canvas.text_fitted(
        &harmony_summary(view),
        row.x(HARMONY_UNITS),
        row.baseline(),
        1.15 * unit,
        harmony_width,
        colors::derived_note(),
    )?;
    Ok(())
}

/// The entry's chords bar by bar, the numeral first and the chord in the
/// entry's key after it: `| I (G) | V7 (D7) |`.
#[must_use]
pub fn harmony_summary(entry: &EntryView) -> String {
    let bars: Vec<String> = entry
        .bars
        .iter()
        .map(|bar| {
            let chords: Vec<String> = bar
                .chords
                .iter()
                .map(|chord| format!("{} ({})", chord.numeral, chord.symbol))
                .collect();
            if chords.is_empty() {
                String::from("-")
            } else {
                chords.join(" ")
            }
        })
        .collect();
    if bars.is_empty() {
        String::new()
    } else {
        format!("| {} |", bars.join(" | "))
    }
}

// ---- Dialogs ---------------------------------------------------------------

/// Draws F1's open dialog, if any, over the main view.
///
/// # Errors
///
/// When the atlas lacks a glyph.
pub fn draw_dialog(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    session: &Session,
) -> Result<(), RenderError> {
    let Some(dialog) = session.arrange().dialog.as_ref() else {
        return Ok(());
    };
    let (title, hint, height_units): (String, &str, f32) = dialog_words(dialog);
    let width: f32 = (70.0 * unit).min(area.right_x_pixels - area.left_x_pixels - 2.0 * unit);
    let height: f32 = height_units * unit;
    let panel: ScreenRect = rect(
        f32::midpoint(area.left_x_pixels, area.right_x_pixels) - width / 2.0,
        area.top_y_pixels + 3.0 * unit,
        width,
        height,
    );
    canvas.fill(panel, colors::panel_raised());
    canvas.frame(panel, 2.0, colors::focus_edge());
    // The panel takes clicks, so a click inside it misses the rows below.
    canvas.hit(panel, HitTarget::Arrange(ArrangeTarget::Panel));
    let left: f32 = panel.left_x_pixels + 1.5 * unit;
    let inner_width: f32 = width - 3.0 * unit;
    canvas.text(
        &title,
        left,
        panel.top_y_pixels + 2.2 * unit,
        1.35 * unit,
        Align::Left,
        colors::text(),
    )?;
    let body_top: f32 = panel.top_y_pixels + 3.5 * unit;
    draw_dialog_body(canvas, dialog, left, body_top, inner_width, unit)?;
    // OK and Cancel along the bottom, with the keys above them.
    let buttons_top: f32 = panel.bottom_y_pixels - 3.2 * unit;
    canvas.text_fitted(
        hint,
        left,
        buttons_top - 0.8 * unit,
        1.0 * unit,
        inner_width,
        colors::text_soft(),
    )?;
    let ok_label: &str = match dialog {
        ArrangeDialog::NewSection(_) => "Make it",
        ArrangeDialog::AddEntry { .. } => "Add",
        _ => "OK",
    };
    dialog_button(
        canvas,
        rect(left, buttons_top, 10.0 * unit, 2.4 * unit),
        ok_label,
        unit,
        true,
        HitTarget::Arrange(ArrangeTarget::Confirm),
    )?;
    dialog_button(
        canvas,
        rect(left + 11.0 * unit, buttons_top, 10.0 * unit, 2.4 * unit),
        "Cancel",
        unit,
        false,
        HitTarget::Arrange(ArrangeTarget::Cancel),
    )?;
    Ok(())
}

/// What a dialog holds between its title and its buttons.
fn draw_dialog_body(
    canvas: &mut Canvas<'_>,
    dialog: &ArrangeDialog,
    left: f32,
    body_top: f32,
    inner_width: f32,
    unit: f32,
) -> Result<(), RenderError> {
    match dialog {
        ArrangeDialog::Rename { field, .. } | ArrangeDialog::Tempo { field, .. } => {
            draw_field(
                canvas,
                rect(left, body_top, 30.0 * unit, 2.8 * unit),
                unit,
                field,
                true,
            )?;
        }
        ArrangeDialog::Key {
            choice, can_clear, ..
        } => {
            draw_key_grid(canvas, left, body_top, inner_width, unit, Some(*choice))?;
            if *can_clear {
                dialog_button(
                    canvas,
                    rect(left, body_top + 5.6 * unit, 26.0 * unit, 2.4 * unit),
                    "No key of its own (inherit)",
                    unit,
                    false,
                    HitTarget::Arrange(ArrangeTarget::NoKey),
                )?;
            }
        }
        ArrangeDialog::Meter { choice, .. } => {
            draw_meter_choice(canvas, left, body_top, inner_width, unit, *choice, true)?;
        }
        ArrangeDialog::AddEntry { names, chosen, .. } => {
            for (index, name) in names.iter().enumerate().take(12) {
                let line: ScreenRect = rect(
                    left,
                    body_top + count_as_f32(index) * 2.2 * unit,
                    inner_width,
                    2.0 * unit,
                );
                if index == *chosen {
                    canvas.fill(line, colors::selection_fill());
                    canvas.frame(line, 1.0, colors::selection_edge());
                }
                canvas.text(
                    name,
                    line.left_x_pixels + 0.6 * unit,
                    line.top_y_pixels + 1.45 * unit,
                    1.2 * unit,
                    Align::Left,
                    colors::text(),
                )?;
                canvas.hit(
                    line,
                    HitTarget::Arrange(ArrangeTarget::SectionChoice(index)),
                );
            }
        }
        ArrangeDialog::NewSection(form) => {
            draw_new_section(canvas, left, body_top, inner_width, unit, form)?;
        }
    }
    Ok(())
}

/// A dialog's title, the keys it takes, and its height in units.
fn dialog_words(dialog: &ArrangeDialog) -> (String, &'static str, f32) {
    match dialog {
        ArrangeDialog::Rename { section_name, .. } => (
            format!("Rename section \"{section_name}\" (every entry follows)"),
            "Enter renames   Esc cancels",
            12.0,
        ),
        ArrangeDialog::Tempo { .. } => (
            String::from("Tempo in beats/min"),
            "Type a number such as 112.5   + and - change it by 1   Enter sets it   Esc cancels",
            12.0,
        ),
        ArrangeDialog::Key { can_clear, .. } => (
            String::from("Key"),
            if *can_clear {
                "Left/Right round the circle of fifths   Up/Down major or minor   Enter picks   Delete: no key of its own   Esc cancels"
            } else {
                "Left/Right round the circle of fifths   Up/Down major or minor   Enter picks   Esc cancels"
            },
            15.0,
        ),
        ArrangeDialog::Meter { .. } => (
            String::from("Meter"),
            "Up/Down or + and - the upper figure   Left/Right the lower figure   Enter picks   Esc cancels",
            14.0,
        ),
        ArrangeDialog::AddEntry { names, .. } => (
            String::from("Add an entry for the section"),
            "Up/Down choose   Enter adds   Esc cancels",
            8.0 + 2.2 * count_as_f32(names.len().min(12)),
        ),
        ArrangeDialog::NewSection(_) => (
            String::from("New section"),
            "Up/Down or Tab field   Enter makes it   Esc cancels   (its bars are set in the Section Editor, F3)",
            27.0,
        ),
    }
}

fn dialog_button(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    label: &str,
    unit: f32,
    chosen: bool,
    target: HitTarget,
) -> Result<(), RenderError> {
    if chosen {
        canvas.fill(area, colors::selection_fill());
        canvas.frame(area, 1.0, colors::selection_edge());
    } else {
        canvas.fill(area, colors::panel());
        canvas.frame(area, 1.0, colors::border());
    }
    canvas.text_in(label, area, 1.15 * unit, colors::text())?;
    canvas.hit(area, target);
    Ok(())
}

/// A text field, with its cursor when it has the keyboard.
fn draw_field(
    canvas: &mut Canvas<'_>,
    box_area: ScreenRect,
    unit: f32,
    field: &TextField,
    focused: bool,
) -> Result<(), RenderError> {
    canvas.fill(box_area, colors::panel());
    let edge: [f32; 4] = if focused {
        colors::focus_edge()
    } else {
        colors::border()
    };
    canvas.frame(box_area, 1.0, edge);
    let em: f32 = 1.3 * unit;
    let baseline: f32 = f32::midpoint(box_area.top_y_pixels, box_area.bottom_y_pixels)
        + TEXT_CAP_HEIGHT_EMS * em / 2.0;
    let text_left: f32 = box_area.left_x_pixels + unit;
    canvas.text(
        field.text(),
        text_left,
        baseline,
        em,
        Align::Left,
        colors::text(),
    )?;
    if focused {
        let cursor_x: f32 = text_left + canvas.text_width(field.text_before_cursor(), em);
        canvas.fill(
            rect(
                cursor_x,
                box_area.top_y_pixels + 0.5 * unit,
                0.15 * unit,
                box_area.bottom_y_pixels - box_area.top_y_pixels - unit,
            ),
            colors::text(),
        );
    }
    Ok(())
}

/// Every key signature, flats on the left and sharps on the right: the
/// major keys in the upper row, their relative minors below.
fn draw_key_grid(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    width: f32,
    unit: f32,
    chosen: Option<KeyChoice>,
) -> Result<(), RenderError> {
    let label_width: f32 = 6.0 * unit;
    let count: f32 = f32::from(MOST_SHARPS - MOST_FLATS + 1);
    let chip_width: f32 = (width - label_width) / count;
    for (row_index, mode) in [Mode::Major, Mode::Minor].into_iter().enumerate() {
        let row_top: f32 = top + count_as_f32(row_index) * 2.6 * unit;
        canvas.text(
            mode.text(),
            left,
            row_top + 1.6 * unit,
            1.1 * unit,
            Align::Left,
            colors::text_soft(),
        )?;
        for fifths in MOST_FLATS..=MOST_SHARPS {
            let key: Key = key_of(fifths, mode);
            let chip: ScreenRect = rect(
                left + label_width + f32::from(fifths - MOST_FLATS) * chip_width,
                row_top,
                chip_width - 0.2 * unit,
                2.3 * unit,
            );
            canvas.fill(chip, key_chip_color(key));
            if chosen == Some(KeyChoice { fifths, mode }) {
                canvas.frame(chip, 2.0, colors::focus_edge());
            }
            canvas.text_in(
                &note_name_text(key.tonic()),
                chip,
                1.1 * unit,
                colors::text(),
            )?;
            canvas.hit(
                chip,
                HitTarget::Arrange(ArrangeTarget::KeyChoice {
                    fifths,
                    minor: mode == Mode::Minor,
                }),
            );
        }
    }
    Ok(())
}

/// The meter in large figures and the presets below it.
fn draw_meter_choice(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    width: f32,
    unit: f32,
    choice: MeterChoice,
    large: bool,
) -> Result<(), RenderError> {
    let em: f32 = if large { 2.2 * unit } else { 1.3 * unit };
    let figures_height: f32 = if large { 3.2 * unit } else { 2.0 * unit };
    canvas.text(
        &meter_text(choice),
        left,
        top + figures_height - 0.4 * unit,
        em,
        Align::Left,
        colors::text(),
    )?;
    let presets_top: f32 = top + figures_height + 0.4 * unit;
    let preset_width: f32 = (width / count_as_f32(METER_PRESETS.len())).min(6.0 * unit);
    for (index, (upper, lower)) in METER_PRESETS.iter().enumerate() {
        let chip: ScreenRect = rect(
            left + count_as_f32(index) * preset_width,
            presets_top,
            preset_width - 0.3 * unit,
            2.2 * unit,
        );
        let chosen: bool =
            choice.pulses_per_bar == *upper && choice.pulses_per_whole_note == *lower;
        dialog_button(
            canvas,
            chip,
            &format!("{upper}/{lower}"),
            unit,
            chosen,
            HitTarget::Arrange(ArrangeTarget::MeterPreset {
                pulses_per_bar: *upper,
                pulses_per_whole_note: *lower,
            }),
        )?;
    }
    Ok(())
}

/// The new-section form, field by field.
// A layout reads top to bottom, as the screen does; split up it would
// scatter one picture over several functions.
#[allow(clippy::too_many_lines)]
fn draw_new_section(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    width: f32,
    unit: f32,
    form: &NewSectionForm,
) -> Result<(), RenderError> {
    let label_width: f32 = 9.0 * unit;
    let field_left: f32 = left + label_width;
    let field_width: f32 = width - label_width;
    let mut row_top: f32 = top;
    for field in [
        FormField::Name,
        FormField::Meter,
        FormField::Key,
        FormField::Tempo,
        FormField::CountIn,
    ] {
        let focused: bool = form.field == field;
        let (label, height_units): (&str, f32) = match field {
            FormField::Name => ("Name", 3.2),
            FormField::Meter => ("Meter", 5.0),
            FormField::Key => ("Key", 3.2),
            FormField::Tempo => ("Beats/min", 3.2),
            FormField::CountIn => ("Count-in", 3.2),
        };
        let label_area: ScreenRect = rect(left, row_top, label_width - unit, 2.8 * unit);
        canvas.text(
            label,
            left,
            row_top + 1.9 * unit,
            1.15 * unit,
            Align::Left,
            if focused {
                colors::focus_edge()
            } else {
                colors::text_soft()
            },
        )?;
        canvas.hit(
            label_area,
            HitTarget::Arrange(ArrangeTarget::FormField(field)),
        );
        match field {
            FormField::Name => {
                let area: ScreenRect = rect(field_left, row_top, 30.0 * unit, 2.8 * unit);
                canvas.hit(area, HitTarget::Arrange(ArrangeTarget::FormField(field)));
                draw_field(canvas, area, unit, &form.name, focused)?;
            }
            FormField::Tempo => {
                let area: ScreenRect = rect(field_left, row_top, 14.0 * unit, 2.8 * unit);
                canvas.hit(area, HitTarget::Arrange(ArrangeTarget::FormField(field)));
                draw_field(canvas, area, unit, &form.tempo, focused)?;
                if form.tempo.text().is_empty() {
                    canvas.text(
                        "empty: the composition's tempo",
                        area.right_x_pixels + unit,
                        row_top + 1.9 * unit,
                        1.0 * unit,
                        Align::Left,
                        colors::text_faint(),
                    )?;
                }
            }
            FormField::Meter => {
                let area: ScreenRect = rect(field_left, row_top, field_width, 4.6 * unit);
                canvas.hit(area, HitTarget::Arrange(ArrangeTarget::FormField(field)));
                if focused {
                    canvas.frame(area, 1.0, colors::focus_edge());
                }
                draw_meter_choice(
                    canvas,
                    field_left + 0.5 * unit,
                    row_top,
                    field_width - unit,
                    unit,
                    form.meter,
                    false,
                )?;
            }
            FormField::Key => {
                let area: ScreenRect = rect(field_left, row_top, 20.0 * unit, 2.8 * unit);
                canvas.hit(area, HitTarget::Arrange(ArrangeTarget::FormField(field)));
                canvas.fill(area, colors::panel());
                canvas.frame(
                    area,
                    1.0,
                    if focused {
                        colors::focus_edge()
                    } else {
                        colors::border()
                    },
                );
                let text: String = form.key.map_or_else(
                    || String::from("the composition's key"),
                    |choice: KeyChoice| key_label_of(choice.key()),
                );
                canvas.text(
                    &text,
                    field_left + unit,
                    row_top + 1.9 * unit,
                    1.2 * unit,
                    Align::Left,
                    colors::text(),
                )?;
                canvas.text(
                    "Left/Right fifths   M major/minor   Delete: composition's key",
                    area.right_x_pixels + unit,
                    row_top + 1.9 * unit,
                    1.0 * unit,
                    Align::Left,
                    colors::text_faint(),
                )?;
            }
            FormField::CountIn => {
                let area: ScreenRect = rect(field_left, row_top, 14.0 * unit, 2.8 * unit);
                canvas.hit(area, HitTarget::Arrange(ArrangeTarget::FormField(field)));
                canvas.fill(area, colors::panel());
                canvas.frame(
                    area,
                    1.0,
                    if focused {
                        colors::focus_edge()
                    } else {
                        colors::border()
                    },
                );
                let text: &str = if form.count_in { "yes" } else { "no" };
                canvas.text(
                    text,
                    field_left + unit,
                    row_top + 1.9 * unit,
                    1.2 * unit,
                    Align::Left,
                    colors::text(),
                )?;
                canvas.text(
                    "Space or click to change. Only the first section played",
                    area.right_x_pixels + unit,
                    row_top + 1.9 * unit,
                    1.0 * unit,
                    Align::Left,
                    colors::text_faint(),
                )?;
            }
        }
        row_top += height_units * unit;
    }
    Ok(())
}
