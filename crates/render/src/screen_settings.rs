//! The settings dialog, in two panes side by side. The song's pane has the
//! title, the chord numeral standard, and a row for each track with its
//! type, clef and lead. The studio's pane has the studio's name, its ports,
//! the output settings of the track chosen in the song's pane, and the cue
//! destinations. Sizes are in the screen's units (window height ÷ 90); the
//! colours are the dark theme's.

use composer::session::{MessageKind, Session};
use composer::settings_view::{
    Pane, Picker, ROWS_BEFORE_TRACKS, SettingsButton, SettingsView, Typing, VISIBLE_SETTINGS_ROWS,
};
use composer::shell::ShellTarget;
use composer::song_settings::{TrackType, clef_word};
use composer::studio_settings::{StudioRow, StudioRowView};
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::song::{Track, TrackKind};

use crate::canvas::{Align, Canvas, HitTarget, MessageSource, rect};
use crate::error::RenderError;
use crate::screen::ScreenLayout;
use crate::screen_shell::{
    SCROLL_BAR_WIDTH_UNITS, WrappedText, dialog_area, dialog_button, dim_everything,
    draw_scroll_bar, draw_wrapped, rows_as_f32, text_left_in,
};
use crate::screen_theme as colors;
use crate::strip_layout::{ScreenRect, TEXT_CAP_HEIGHT_EMS};

const DIALOG_WIDTH_UNITS: f32 = 136.0;
/// The space between the two panes.
const PANE_GAP_UNITS: f32 = 1.5;
const DIALOG_HEIGHT_UNITS: f32 = 63.0;
const ROW_UNITS: f32 = 2.6;
/// Rows a pane has room for: the visible rows and, in the song pane, the
/// line of column titles above the tracks.
const PANE_SLOTS: usize = VISIBLE_SETTINGS_ROWS + 1;
/// Where each column starts, in units from the row's left edge.
const TYPE_COLUMN_UNITS: f32 = 18.0;
const CLEF_COLUMN_UNITS: f32 = 33.0;
const LEAD_COLUMN_UNITS: f32 = 40.0;
/// Where the voice layer names start, and how much room they have.
const LAYERS_COLUMN_UNITS: f32 = 47.0;
const LAYERS_WIDTH_UNITS: f32 = 14.0;
/// Where the value of a title, numerals or studio row starts.
const VALUE_COLUMN_UNITS: f32 = 18.0;
/// Where the value of a studio output row starts (the labels are longer).
const STUDIO_VALUE_COLUMN_UNITS: f32 = 20.0;

fn target(target: ShellTarget) -> HitTarget {
    HitTarget::Shell(target)
}

/// What the numeral standard row shows.
fn numerals_text(standard: Option<NumeralStandard>) -> &'static str {
    match standard {
        None => "not set",
        Some(NumeralStandard::KeyDependent) => "key dependent",
        Some(NumeralStandard::MajorReferential) => "major referential",
    }
}

/// The dialog, on top of everything drawn so far.
///
/// # Errors
///
/// When the atlas fails to place text.
pub fn draw(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    view: &SettingsView,
    session: Option<&Session>,
) -> Result<(), RenderError> {
    let Some(session) = session else {
        return Ok(());
    };
    let unit: f32 = layout.unit_pixels;
    dim_everything(canvas, layout);
    let dialog: ScreenRect = dialog_area(layout, DIALOG_WIDTH_UNITS, DIALOG_HEIGHT_UNITS);
    canvas.fill(dialog, colors::panel_raised());
    canvas.frame(dialog, 1.0, colors::border());
    let left: f32 = dialog.left_x_pixels + 1.5 * unit;
    let inner_width: f32 = dialog.right_x_pixels - dialog.left_x_pixels - 3.0 * unit;
    canvas.text(
        "Settings",
        left,
        dialog.top_y_pixels + 2.8 * unit,
        1.6 * unit,
        Align::Left,
        colors::text(),
    )?;
    let pane_width: f32 = (inner_width - PANE_GAP_UNITS * unit) / 2.0;
    let list_height: f32 = ROW_UNITS * unit * rows_as_f32(PANE_SLOTS) + 0.4 * unit;
    let list_top: f32 = dialog.top_y_pixels + 6.0 * unit;
    let list: ScreenRect = rect(left, list_top, pane_width, list_height);
    let studio_list: ScreenRect = rect(
        left + pane_width + PANE_GAP_UNITS * unit,
        list_top,
        pane_width,
        list_height,
    );
    // Under the rows (later hits win): the whole pane, for the wheel and
    // for clicks on its empty part.
    canvas.hit(list, target(ShellTarget::SongPane));
    canvas.hit(studio_list, target(ShellTarget::StudioPane));
    draw_pane_frames(canvas, unit, view, list, studio_list)?;
    draw_rows(canvas, list, unit, view, session)?;
    draw_studio_rows(canvas, studio_list, unit, view)?;
    if let Some((kind, text)) = view.message() {
        let color: [f32; 4] = match kind {
            MessageKind::Done => colors::done(),
            MessageKind::Problem => colors::problem(),
        };
        canvas.hit(
            rect(
                left,
                list.bottom_y_pixels + 0.2 * unit,
                inner_width,
                4.6 * unit,
            ),
            HitTarget::CopyMessage(MessageSource::Dialog),
        );
        draw_wrapped(
            canvas,
            text,
            WrappedText {
                left_pixels: left,
                first_baseline_pixels: list.bottom_y_pixels + 1.9 * unit,
                line_step_pixels: 1.5 * unit,
                em_pixels: 1.15 * unit,
                max_width_pixels: inner_width,
                max_lines: 3,
            },
            color,
        )?;
    }
    let below: f32 = list.bottom_y_pixels + 5.4 * unit;
    if let Some(track_index) = view.confirming_delete() {
        draw_delete_question(canvas, left, below, unit, view, track_index)?;
    } else if let Some(row) = view.confirming_studio_removal() {
        draw_question(
            canvas,
            left,
            below,
            unit,
            &removal_question(view, row),
            "Remove",
        )?;
    } else if let Some((typing, field)) = view.typing() {
        draw_typing(canvas, left, below, inner_width, unit, *typing, field)?;
    } else if let Some(picker) = view.picker() {
        draw_picker(canvas, left, below, unit, picker)?;
    } else {
        draw_buttons(canvas, left, below, unit, view.pane())?;
    }
    let keys: &str = key_help(view);
    text_left_in(
        canvas,
        keys,
        left,
        rect(
            left,
            dialog.bottom_y_pixels - 3.4 * unit,
            inner_width,
            2.4 * unit,
        ),
        1.05 * unit,
        colors::text_faint(),
    )
}

/// The line of key help along the dialog's bottom.
fn key_help(view: &SettingsView) -> &'static str {
    if view.is_typing() {
        "Enter accepts   Esc cancels typing"
    } else if view.picker().is_some() {
        "Left and Right choose   Enter takes it   Esc puts it away"
    } else if view.pane() == Pane::Song {
        "Enter edits   T type   C clef   L lead   V layer names   Ctrl+Up/Down moves   N new   Delete removes   Tab studio   Esc closes"
    } else {
        "Enter edits or steps the value   Delete removes   Tab song   Esc closes"
    }
}

/// Each pane's heading and frame, brighter for the pane that has the keys.
fn draw_pane_frames(
    canvas: &mut Canvas<'_>,
    unit: f32,
    view: &SettingsView,
    song_area: ScreenRect,
    studio_area: ScreenRect,
) -> Result<(), RenderError> {
    for (area, heading, pane) in [
        (song_area, "Song", Pane::Song),
        (studio_area, studio_heading(view), Pane::Studio),
    ] {
        let focused: bool = view.pane() == pane;
        canvas.text(
            heading,
            area.left_x_pixels,
            area.top_y_pixels - 0.6 * unit,
            1.2 * unit,
            Align::Left,
            if focused {
                colors::text()
            } else {
                colors::text_soft()
            },
        )?;
        canvas.frame(
            area,
            1.0,
            if focused {
                colors::selection_edge()
            } else {
                colors::border()
            },
        );
    }
    Ok(())
}

fn draw_rows(
    canvas: &mut Canvas<'_>,
    list: ScreenRect,
    unit: f32,
    view: &SettingsView,
    session: &Session,
) -> Result<(), RenderError> {
    let rows = view.list();
    let song = session.document().song();
    let scrolls: bool = rows.len() > rows.visible_rows();
    let bar_room: f32 = if scrolls {
        (SCROLL_BAR_WIDTH_UNITS + 0.8) * unit
    } else {
        0.0
    };
    let em: f32 = 1.2 * unit;
    let mut next_slot: usize = 0;
    for index in (rows.top()..rows.len()).take(rows.visible_rows()) {
        if index == ROWS_BEFORE_TRACKS {
            draw_track_titles(canvas, list, unit, next_slot, bar_room)?;
            next_slot += 1;
        }
        let slot: usize = next_slot;
        next_slot += 1;
        let row: ScreenRect = rect(
            list.left_x_pixels + 0.2 * unit,
            list.top_y_pixels + 0.2 * unit + ROW_UNITS * unit * rows_as_f32(slot),
            list.right_x_pixels - list.left_x_pixels - 0.4 * unit - bar_room,
            ROW_UNITS * unit,
        );
        if index == rows.selected() {
            highlight(canvas, row, view.pane() == Pane::Song);
        }
        let text_x: f32 = row.left_x_pixels + unit;
        match index {
            0 => {
                text_left_in(canvas, "Title", text_x, row, em, colors::text_soft())?;
                text_left_in(
                    canvas,
                    song.name(),
                    row.left_x_pixels + VALUE_COLUMN_UNITS * unit,
                    row,
                    em,
                    colors::text(),
                )?;
            }
            1 => {
                text_left_in(
                    canvas,
                    "Chord numerals",
                    text_x,
                    row,
                    em,
                    colors::text_soft(),
                )?;
                text_left_in(
                    canvas,
                    numerals_text(song.numeral_standard()),
                    row.left_x_pixels + VALUE_COLUMN_UNITS * unit,
                    row,
                    em,
                    colors::text(),
                )?;
            }
            _ => {
                if let Some(track) = song.tracks().get(index - ROWS_BEFORE_TRACKS) {
                    draw_track_row(canvas, row, unit, em, track)?;
                    draw_track_note(canvas, row, unit, em, view, index)?;
                }
            }
        }
        canvas.hit(row, target(ShellTarget::SettingsRow(index)));
    }
    if scrolls {
        draw_scroll_bar(
            canvas,
            list,
            unit,
            rows,
            target(ShellTarget::SettingsPage { up: true }),
            target(ShellTarget::SettingsPage { up: false }),
        );
    }
    Ok(())
}

/// At the right of a track row, a red "!" when the studio lacks what the
/// song needs of this track (the studio pane says what).
fn draw_track_note(
    canvas: &mut Canvas<'_>,
    row: ScreenRect,
    unit: f32,
    em: f32,
    view: &SettingsView,
    index: usize,
) -> Result<(), RenderError> {
    let needs_studio: bool = view
        .track_name(index - ROWS_BEFORE_TRACKS)
        .is_some_and(|name| view.track_needs_studio(name));
    if needs_studio {
        text_left_in(
            canvas,
            "!",
            row.right_x_pixels - 2.5 * unit,
            row,
            em,
            colors::problem(),
        )?;
    }
    Ok(())
}

/// The choice of a type or a clef: its options as buttons, the one shown
/// filled.
fn draw_picker(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    unit: f32,
    picker: &Picker,
) -> Result<(), RenderError> {
    let em: f32 = 1.2 * unit;
    let title: String = format!("{}:", picker.what);
    canvas.text(
        &title,
        left,
        top + 2.7 * unit,
        em,
        Align::Left,
        colors::text_soft(),
    )?;
    let mut x: f32 = left + canvas.text_width(&title, em) + 1.5 * unit;
    for (index, option) in picker.options.iter().enumerate() {
        let width: f32 = canvas.text_width(&option.label, em) + 3.0 * unit;
        dialog_button(
            canvas,
            rect(x, top + 1.0 * unit, width, 2.6 * unit),
            &option.label,
            em,
            index == picker.selected,
            ShellTarget::SettingsPick(index),
        )?;
        x += width + unit;
    }
    Ok(())
}

/// The column titles above the track rows.
fn draw_track_titles(
    canvas: &mut Canvas<'_>,
    list: ScreenRect,
    unit: f32,
    slot: usize,
    bar_room: f32,
) -> Result<(), RenderError> {
    let area: ScreenRect = rect(
        list.left_x_pixels + 0.2 * unit,
        list.top_y_pixels + 0.2 * unit + ROW_UNITS * unit * rows_as_f32(slot),
        list.right_x_pixels - list.left_x_pixels - 0.4 * unit - bar_room,
        ROW_UNITS * unit,
    );
    let em: f32 = 1.1 * unit;
    for (title, column_units) in [
        ("Track", 0.0),
        ("Type", TYPE_COLUMN_UNITS),
        ("Clef", CLEF_COLUMN_UNITS),
        ("Lead", LEAD_COLUMN_UNITS),
        ("Layers", LAYERS_COLUMN_UNITS),
    ] {
        bold_left_in(
            canvas,
            title,
            area.left_x_pixels
                + (column_units + if column_units == 0.0 { 1.0 } else { 0.0 }) * unit,
            area,
            em,
            colors::text_soft(),
        )?;
    }
    // A rule under the titles, to set them apart from the rows.
    canvas.fill(
        rect(
            area.left_x_pixels,
            area.bottom_y_pixels - 0.15 * unit,
            area.right_x_pixels - area.left_x_pixels,
            1.0,
        ),
        colors::border(),
    );
    Ok(())
}

/// Text made heavier by drawing it twice, a hair apart: the atlas has no
/// bold face.
fn bold_left_in(
    canvas: &mut Canvas<'_>,
    text: &str,
    x_pixels: f32,
    area: ScreenRect,
    em_pixels: f32,
    color_linear_rgba: [f32; 4],
) -> Result<(), RenderError> {
    for offset_em in [0.0, 0.07] {
        text_left_in(
            canvas,
            text,
            x_pixels + offset_em * em_pixels,
            area,
            em_pixels,
            color_linear_rgba,
        )?;
    }
    Ok(())
}

fn draw_track_row(
    canvas: &mut Canvas<'_>,
    row: ScreenRect,
    unit: f32,
    em: f32,
    track: &Track,
) -> Result<(), RenderError> {
    text_left_in(
        canvas,
        track.name(),
        row.left_x_pixels + unit,
        row,
        em,
        colors::text(),
    )?;
    text_left_in(
        canvas,
        TrackType::of(track).word(),
        row.left_x_pixels + TYPE_COLUMN_UNITS * unit,
        row,
        em,
        colors::text_soft(),
    )?;
    if let TrackKind::Voice {
        clef,
        lead,
        voice_layer_names,
        ..
    } = track.kind()
    {
        let layers: Vec<String> = voice_layer_names
            .iter()
            .map(|layer| format!("{} {}", layer.number().value(), layer.name()))
            .collect();
        canvas.text_fitted(
            &layers.join(", "),
            row.left_x_pixels + LAYERS_COLUMN_UNITS * unit,
            f32::midpoint(row.top_y_pixels, row.bottom_y_pixels) + TEXT_CAP_HEIGHT_EMS * em / 2.0,
            em,
            LAYERS_WIDTH_UNITS * unit,
            colors::text_soft(),
        )?;
        text_left_in(
            canvas,
            clef_word(*clef),
            row.left_x_pixels + CLEF_COLUMN_UNITS * unit,
            row,
            em,
            colors::text_soft(),
        )?;
        if *lead {
            text_left_in(
                canvas,
                "lead",
                row.left_x_pixels + LEAD_COLUMN_UNITS * unit,
                row,
                em,
                colors::done(),
            )?;
        }
    }
    Ok(())
}

fn draw_typing(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    width: f32,
    unit: f32,
    typing: Typing,
    field: &composer::text_field::TextField,
) -> Result<(), RenderError> {
    let what: &str = match typing {
        Typing::Title => "Song title:",
        Typing::Rename { .. } => "Track name:",
        Typing::NewTrack => "Name of the new track (you choose its type next):",
        Typing::VoiceLayer { .. } => {
            "Layer number and name, e.g. 2 Inner (the number alone clears its name):"
        }
        Typing::Studio(row) => studio_prompt(row),
    };
    canvas.text(
        what,
        left,
        top + 1.2 * unit,
        1.1 * unit,
        Align::Left,
        colors::text_soft(),
    )?;
    let box_area: ScreenRect = rect(left, top + 2.0 * unit, width - 22.0 * unit, 2.8 * unit);
    canvas.fill(box_area, colors::panel());
    canvas.frame(box_area, 1.0, colors::selection_edge());
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
    dialog_button(
        canvas,
        rect(
            box_area.right_x_pixels + unit,
            box_area.top_y_pixels,
            9.0 * unit,
            2.6 * unit,
        ),
        "Accept",
        1.2 * unit,
        true,
        ShellTarget::SettingsButton(SettingsButton::Accept),
    )?;
    dialog_button(
        canvas,
        rect(
            box_area.right_x_pixels + 11.0 * unit,
            box_area.top_y_pixels,
            9.0 * unit,
            2.6 * unit,
        ),
        "Cancel",
        1.2 * unit,
        false,
        ShellTarget::SettingsButton(SettingsButton::Cancel),
    )
}

fn draw_delete_question(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    unit: f32,
    view: &SettingsView,
    track_index: usize,
) -> Result<(), RenderError> {
    let name: &str = view.track_name(track_index).unwrap_or("");
    draw_question(
        canvas,
        left,
        top,
        unit,
        &format!("Delete track {name} and its line in every section?"),
        "Delete",
    )
}

/// A question in red with its yes button (named `yes`) and a Keep button.
fn draw_question(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    unit: f32,
    question: &str,
    yes: &str,
) -> Result<(), RenderError> {
    canvas.text(
        question,
        left,
        top + 2.4 * unit,
        1.3 * unit,
        Align::Left,
        colors::problem(),
    )?;
    dialog_button(
        canvas,
        rect(left, top + 3.6 * unit, 11.0 * unit, 2.6 * unit),
        yes,
        1.2 * unit,
        true,
        ShellTarget::SettingsButton(SettingsButton::Accept),
    )?;
    dialog_button(
        canvas,
        rect(
            left + 12.0 * unit,
            top + 3.6 * unit,
            11.0 * unit,
            2.6 * unit,
        ),
        "Keep",
        1.2 * unit,
        false,
        ShellTarget::SettingsButton(SettingsButton::Cancel),
    )
}

/// A chosen row: filled when its pane has the keys, only outlined when not.
fn highlight(canvas: &mut Canvas<'_>, row: ScreenRect, focused: bool) {
    if focused {
        canvas.fill(row, colors::selection_fill());
        canvas.frame(row, 1.0, colors::selection_edge());
    } else {
        canvas.frame(row, 1.0, colors::border());
    }
}

/// The studio pane's heading: what the pane is, and whose output it shows.
fn studio_heading(view: &SettingsView) -> &'static str {
    if view.pane() == Pane::Studio {
        "Studio (Tab for the song)"
    } else {
        "Studio (Tab to edit)"
    }
}

/// What is being typed into a studio row.
fn studio_prompt(row: StudioRow) -> &'static str {
    match row {
        StudioRow::Name => "Studio name:",
        StudioRow::Port(_) => "Port name:",
        StudioRow::AddPort => "Name of the new port:",
        StudioRow::DestinationName(_) => "Destination name:",
        StudioRow::AddDestination => "Name of the new destination (it gets the first port):",
        StudioRow::DestinationChannel(_) => "Channel, 1 to 16 (empty for none):",
        StudioRow::Field(_) => "Value (empty where a value may be left out):",
        _ => "",
    }
}

/// What asking to remove a studio row says.
fn removal_question(view: &SettingsView, row: StudioRow) -> String {
    let rows: Vec<StudioRowView> = view.studio_rows();
    let find = |wanted: StudioRow| rows.iter().find(|r| r.row == wanted);
    match row {
        StudioRow::Port(_) => format!(
            "Remove port {} from the studio?",
            find(row).map_or("", |r| r.label.as_str())
        ),
        StudioRow::DestinationName(i)
        | StudioRow::DestinationPort(i)
        | StudioRow::DestinationChannel(i) => format!(
            "Remove destination {}?",
            find(StudioRow::DestinationName(i)).map_or("", |r| r.value.as_str())
        ),
        _ => format!(
            "Remove the output of {}?",
            view.output_track_name().unwrap_or("this track")
        ),
    }
}

/// What one studio row says: a section title, a note, or a setting with its
/// value.
fn draw_studio_row(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    unit: f32,
    em: f32,
    view_row: &StudioRowView,
) -> Result<(), RenderError> {
    if matches!(
        view_row.row,
        StudioRow::Heading | StudioRow::DestinationName(_)
    ) {
        // A section's title: bold and larger than the settings below it.
        let title: String = if matches!(view_row.row, StudioRow::DestinationName(_)) {
            format!("{}: {}", view_row.label, view_row.value)
        } else {
            view_row.label.clone()
        };
        bold_left_in(
            canvas,
            &title,
            area.left_x_pixels + unit,
            area,
            1.5 * unit,
            colors::text(),
        )?;
        canvas.fill(
            rect(
                area.left_x_pixels,
                area.bottom_y_pixels - 0.15 * unit,
                area.right_x_pixels - area.left_x_pixels,
                1.0,
            ),
            colors::border(),
        );
    } else {
        let action: bool = matches!(
            view_row.row,
            StudioRow::AddPort
                | StudioRow::AddDestination
                | StudioRow::AddOutput
                | StudioRow::Create
        );
        let label_color: [f32; 4] = if view_row.row == StudioRow::Warning {
            colors::problem()
        } else if view_row.row == StudioRow::Note {
            colors::text_faint()
        } else if action {
            colors::done()
        } else {
            colors::text_soft()
        };
        let indent: f32 = if matches!(view_row.row, StudioRow::Port(_)) {
            2.0
        } else {
            1.0
        };
        text_left_in(
            canvas,
            &view_row.label,
            area.left_x_pixels + indent * unit,
            area,
            em,
            label_color,
        )?;
        if !view_row.value.is_empty() {
            let value_color: [f32; 4] = if view_row.value == "none" || view_row.value == "default" {
                colors::text_faint()
            } else {
                colors::text()
            };
            let column: f32 = if matches!(view_row.row, StudioRow::Name | StudioRow::Field(_)) {
                STUDIO_VALUE_COLUMN_UNITS
            } else {
                VALUE_COLUMN_UNITS
            };
            text_left_in(
                canvas,
                &view_row.value,
                area.left_x_pixels + column * unit,
                area,
                em,
                value_color,
            )?;
        }
    }
    Ok(())
}

fn draw_studio_rows(
    canvas: &mut Canvas<'_>,
    list: ScreenRect,
    unit: f32,
    view: &SettingsView,
) -> Result<(), RenderError> {
    let rows: Vec<StudioRowView> = view.studio_rows();
    let scroll = view.studio_list();
    let scrolls: bool = scroll.len() > scroll.visible_rows();
    let bar_room: f32 = if scrolls {
        (SCROLL_BAR_WIDTH_UNITS + 0.8) * unit
    } else {
        0.0
    };
    let em: f32 = 1.2 * unit;
    for (slot, index) in (scroll.top()..rows.len())
        .take(scroll.visible_rows())
        .enumerate()
    {
        let view_row: &StudioRowView = &rows[index];
        let area: ScreenRect = rect(
            list.left_x_pixels + 0.2 * unit,
            list.top_y_pixels + 0.2 * unit + ROW_UNITS * unit * rows_as_f32(slot),
            list.right_x_pixels - list.left_x_pixels - 0.4 * unit - bar_room,
            ROW_UNITS * unit,
        );
        if index == scroll.selected() {
            highlight(canvas, area, view.pane() == Pane::Studio);
        }
        draw_studio_row(canvas, area, unit, em, view_row)?;
        canvas.hit(area, target(ShellTarget::StudioRow(index)));
    }
    if scrolls {
        draw_scroll_bar(
            canvas,
            list,
            unit,
            scroll,
            target(ShellTarget::StudioPage { up: true }),
            target(ShellTarget::StudioPage { up: false }),
        );
    }
    Ok(())
}

fn draw_buttons(
    canvas: &mut Canvas<'_>,
    left: f32,
    top: f32,
    unit: f32,
    pane: Pane,
) -> Result<(), RenderError> {
    let buttons: Vec<(&str, SettingsButton)> = if pane == Pane::Song {
        vec![
            ("Edit (Enter)", SettingsButton::Edit),
            ("Type (T)", SettingsButton::Type),
            ("Clef (C)", SettingsButton::Clef),
            ("Lead (L)", SettingsButton::Lead),
            ("Layers (V)", SettingsButton::Layer),
            ("Up (Ctrl+Up)", SettingsButton::MoveUp),
            ("Down (Ctrl+Down)", SettingsButton::MoveDown),
            ("New track (N)", SettingsButton::New),
            ("Delete track (Del)", SettingsButton::DeleteTrack),
            ("Close (Esc)", SettingsButton::Close),
        ]
    } else {
        vec![
            ("Edit (Enter)", SettingsButton::Edit),
            ("Remove (Del)", SettingsButton::DeleteTrack),
            ("Close (Esc)", SettingsButton::Close),
        ]
    };
    let mut x: f32 = left;
    let em: f32 = 1.1 * unit;
    for (label, button) in buttons {
        let width: f32 = canvas.text_width(label, em) + 2.4 * unit;
        dialog_button(
            canvas,
            rect(x, top + 1.0 * unit, width, 2.6 * unit),
            label,
            em,
            false,
            ShellTarget::SettingsButton(button),
        )?;
        x += width + 0.8 * unit;
    }
    Ok(())
}
