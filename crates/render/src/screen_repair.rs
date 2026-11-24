//! The screen shown when a song or its studio file could not be read: the
//! list of problems, the line being typed, and the buttons that repair
//! them. Sizes are in the screen's units (window height ÷ 90).

use composer::repair::{EditMode, LineEdit, Problem, Repair, RepairTarget, VISIBLE_PROBLEMS};
use composer::shell::Shell;

use crate::canvas::{Align, Canvas, HitTarget, rect};
use crate::error::RenderError;
use crate::screen::ScreenLayout;
use crate::screen_shell::{SCROLL_BAR_WIDTH_UNITS, draw_message_footer, draw_scroll_bar};
use crate::screen_theme as colors;
use crate::strip_layout::{ScreenRect, TEXT_CAP_HEIGHT_EMS};

const ROW_UNITS: f32 = 3.8;
const BUTTON_HEIGHT_UNITS: f32 = 2.6;

/// A row number or count as a length factor (a few dozen at most, so the
/// conversion is exact).
#[allow(clippy::cast_precision_loss)]
fn rows_as_f32(rows: usize) -> f32 {
    rows as f32
}

fn target(target: RepairTarget) -> HitTarget {
    HitTarget::Repair(target)
}

/// The repair screen, between the menu bar and the footer.
///
/// # Errors
///
/// When the atlas fails to place text.
pub fn draw(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    repair: &Repair,
    shell: &Shell,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    let body: ScreenRect = rect(
        0.0,
        layout.menu_bar.bottom_y_pixels,
        layout.footer.right_x_pixels,
        layout.footer.top_y_pixels - layout.menu_bar.bottom_y_pixels,
    );
    canvas.fill(body, colors::panel());
    let left: f32 = 2.0 * unit;
    let width: f32 = body.right_x_pixels - 4.0 * unit;
    let top: f32 = body.top_y_pixels;

    let (headline, headline_color): (String, [f32; 4]) = if repair.is_clean() {
        (String::from("All problems are repaired."), colors::done())
    } else if repair.problems().len() == 1 {
        (String::from("1 problem to repair."), colors::problem())
    } else {
        (
            format!("{} problems to repair.", repair.problems().len()),
            colors::problem(),
        )
    };
    canvas.text(
        &headline,
        left,
        top + 3.0 * unit,
        1.8 * unit,
        Align::Left,
        headline_color,
    )?;
    let advice: &str = if repair.is_clean() {
        "Save to write the files and open the song."
    } else {
        "Playing and editing are off until each line is fixed or tossed."
    };
    canvas.text_fitted(
        advice,
        left,
        top + 5.2 * unit,
        1.2 * unit,
        width,
        colors::text_soft(),
    )?;

    let list: ScreenRect = rect(
        left,
        top + 6.4 * unit,
        width,
        ROW_UNITS * unit * rows_as_f32(VISIBLE_PROBLEMS) + 0.4 * unit,
    );
    canvas.frame(list, 1.0, colors::border());
    draw_rows(canvas, list, unit, repair)?;

    let below: f32 = list.bottom_y_pixels + 1.0 * unit;
    match repair.editing() {
        Some(edit) => draw_edit_line(canvas, left, below, width, unit, repair, edit)?,
        None => draw_buttons(canvas, left, below, unit, repair)?,
    }
    let keys: &str = if repair.editing().is_some() {
        "Enter accepts the line   Esc cancels"
    } else {
        "E edit   I insert after   T toss   Ctrl+T toss all   Ctrl+Z undo   Ctrl+S save   F10 menu"
    };
    draw_message_footer(canvas, layout, shell, keys)
}

fn draw_rows(
    canvas: &mut Canvas<'_>,
    list: ScreenRect,
    unit: f32,
    repair: &Repair,
) -> Result<(), RenderError> {
    let rows = repair.list();
    let scrolls: bool = rows.len() > rows.visible_rows();
    let bar_room: f32 = if scrolls {
        (SCROLL_BAR_WIDTH_UNITS + 0.8) * unit
    } else {
        0.0
    };
    for (slot, (index, problem)) in repair
        .problems()
        .iter()
        .enumerate()
        .skip(rows.top())
        .take(rows.visible_rows())
        .enumerate()
    {
        let row: ScreenRect = rect(
            list.left_x_pixels + 0.2 * unit,
            list.top_y_pixels + 0.2 * unit + ROW_UNITS * unit * rows_as_f32(slot),
            list.right_x_pixels - list.left_x_pixels - 0.4 * unit - bar_room,
            ROW_UNITS * unit,
        );
        if index == rows.selected() {
            canvas.fill(row, colors::selection_fill());
            canvas.frame(row, 1.0, colors::selection_edge());
        }
        draw_problem(canvas, row, unit, repair, problem)?;
        canvas.hit(row, target(RepairTarget::Row(index)));
    }
    if scrolls {
        draw_scroll_bar(
            canvas,
            list,
            unit,
            rows,
            target(RepairTarget::Page { up: true }),
            target(RepairTarget::Page { up: false }),
        );
    }
    Ok(())
}

fn draw_problem(
    canvas: &mut Canvas<'_>,
    row: ScreenRect,
    unit: f32,
    repair: &Repair,
    problem: &Problem,
) -> Result<(), RenderError> {
    let file_name: &str = repair
        .files()
        .get(problem.file_index)
        .map_or("", |file| file.file_name.as_str());
    let max_width: f32 = row.right_x_pixels - row.left_x_pixels - 2.0 * unit;
    canvas.text_fitted(
        &format!(
            "{file_name}  line {}:  {}",
            problem.line_number, problem.message
        ),
        row.left_x_pixels + unit,
        row.top_y_pixels + 1.5 * unit,
        1.05 * unit,
        max_width,
        colors::problem(),
    )?;
    let (text, color): (&str, [f32; 4]) = if problem.line_text.trim().is_empty() {
        ("(blank line)", colors::text_faint())
    } else {
        (problem.line_text.trim(), colors::text())
    };
    canvas.text_fitted(
        text,
        row.left_x_pixels + unit,
        row.top_y_pixels + 3.1 * unit,
        1.2 * unit,
        max_width,
        color,
    )?;
    Ok(())
}

fn draw_edit_line(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    width: f32,
    unit: f32,
    repair: &Repair,
    edit: &LineEdit,
) -> Result<(), RenderError> {
    let what: String = match (repair.selected_problem(), edit.mode) {
        (Some(problem), EditMode::Replace) => format!("New text for line {}:", problem.line_number),
        (Some(problem), EditMode::InsertAfter) => {
            format!("New line to go after line {}:", problem.line_number)
        }
        (None, _) => String::new(),
    };
    canvas.text(
        &what,
        left,
        top + 1.2 * unit,
        1.1 * unit,
        Align::Left,
        colors::text_soft(),
    )?;
    let field: ScreenRect = rect(left, top + 2.0 * unit, width - 22.0 * unit, 2.8 * unit);
    canvas.fill(field, colors::panel_raised());
    canvas.frame(field, 1.0, colors::selection_edge());
    let em: f32 = 1.3 * unit;
    let baseline: f32 =
        f32::midpoint(field.top_y_pixels, field.bottom_y_pixels) + TEXT_CAP_HEIGHT_EMS * em / 2.0;
    let text_left: f32 = field.left_x_pixels + unit;
    canvas.text(
        edit.field.text(),
        text_left,
        baseline,
        em,
        Align::Left,
        colors::text(),
    )?;
    let cursor_x: f32 = text_left + canvas.text_width(edit.field.text_before_cursor(), em);
    canvas.fill(
        rect(
            cursor_x,
            field.top_y_pixels + 0.5 * unit,
            0.15 * unit,
            field.bottom_y_pixels - field.top_y_pixels - unit,
        ),
        colors::text(),
    );
    let button_top: f32 = field.top_y_pixels;
    button(
        canvas,
        rect(
            field.right_x_pixels + unit,
            button_top,
            9.0 * unit,
            BUTTON_HEIGHT_UNITS * unit,
        ),
        "Accept",
        true,
        true,
        RepairTarget::Accept,
        unit,
    )?;
    button(
        canvas,
        rect(
            field.right_x_pixels + 11.0 * unit,
            button_top,
            9.0 * unit,
            BUTTON_HEIGHT_UNITS * unit,
        ),
        "Cancel",
        true,
        false,
        RepairTarget::Cancel,
        unit,
    )
}

fn draw_buttons(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    unit: f32,
    repair: &Repair,
) -> Result<(), RenderError> {
    let has_problem: bool = repair.selected_problem().is_some();
    // Save is offered when something changed; it also opens the song once
    // the files read cleanly.
    let save_label: &str = if repair.is_clean() {
        "Save and open"
    } else {
        "Save"
    };
    let buttons: [(&str, bool, bool, RepairTarget, f32); 6] = [
        ("Edit", has_problem, false, RepairTarget::Edit, 8.0),
        (
            "Insert after",
            has_problem,
            false,
            RepairTarget::Insert,
            12.0,
        ),
        ("Toss", has_problem, false, RepairTarget::Toss, 8.0),
        ("Toss all", has_problem, false, RepairTarget::TossAll, 10.0),
        ("Undo", repair.can_undo(), false, RepairTarget::Undo, 8.0),
        (
            save_label,
            repair.changed(),
            repair.is_clean(),
            RepairTarget::Save,
            14.0,
        ),
    ];
    let mut x: f32 = left;
    for (label, enabled, emphasised, click, width_units) in buttons {
        button(
            canvas,
            rect(x, top, width_units * unit, BUTTON_HEIGHT_UNITS * unit),
            label,
            enabled,
            emphasised,
            click,
            unit,
        )?;
        x += (width_units + 1.0) * unit;
    }
    Ok(())
}

/// A button; one that cannot be used now is drawn faint and takes no click.
fn button(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    label: &str,
    enabled: bool,
    emphasised: bool,
    click: RepairTarget,
    unit: f32,
) -> Result<(), RenderError> {
    if enabled && emphasised {
        canvas.fill(area, colors::selection_fill());
        canvas.frame(area, 1.0, colors::selection_edge());
    } else {
        canvas.frame(area, 1.0, colors::border());
    }
    let color: [f32; 4] = if enabled {
        colors::text()
    } else {
        colors::text_faint()
    };
    canvas.text_in(label, area, 1.2 * unit, color)?;
    if enabled {
        canvas.hit(area, target(click));
    }
    Ok(())
}
