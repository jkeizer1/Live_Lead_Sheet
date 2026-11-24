//! What surrounds the song on the screen: the menu bar, the screen shown
//! while no song is open, and the menu, folder browser and questions drawn
//! on top. Sizes are in the screen's units (window height ÷ 90), like the
//! rest of the screen (EFS-1.12), and the colours are the dark theme's, so
//! no control is bright (EFS-1.13).

use composer::folder_browser::{
    BrowserEntry, BrowserEntryKind, BrowserPurpose, FolderBrowser, VISIBLE_ROWS,
};
use composer::scroll_list::ScrollList;
use composer::session::{MessageKind, Session};
use composer::shell::{Confirmation, MenuCommand, MenuId, Modal, Shell, ShellTarget};

use crate::canvas::{Align, Canvas, HitTarget, MessageSource, rect};
use crate::error::RenderError;
use crate::screen::ScreenLayout;
use crate::screen_settings;
use crate::screen_theme as colors;
use crate::strip_layout::{ScreenRect, TEXT_CAP_HEIGHT_EMS};

const MENU_ROW_UNITS: f32 = 2.4;
const BROWSER_ROW_UNITS: f32 = 2.2;
const BROWSER_WIDTH_UNITS: f32 = 60.0;
const BROWSER_HEIGHT_UNITS: f32 = 54.0;
pub const SCROLL_BAR_WIDTH_UNITS: f32 = 1.0;
const CONFIRM_WIDTH_UNITS: f32 = 60.0;
const CONFIRM_HEIGHT_UNITS: f32 = 14.0;

/// A row number or count as a length factor. Menus and lists have a few
/// dozen rows at most, far below 2^24, so the conversion is exact.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn rows_as_f32(rows: usize) -> f32 {
    rows as f32
}

fn target(target: ShellTarget) -> HitTarget {
    HitTarget::Shell(target)
}

/// Text at `x_pixels`, centred up and down in `area`.
pub(crate) fn text_left_in(
    canvas: &mut Canvas<'_>,
    text: &str,
    x_pixels: f32,
    area: ScreenRect,
    em_pixels: f32,
    color_linear_rgba: [f32; 4],
) -> Result<(), RenderError> {
    let middle: f32 = f32::midpoint(area.top_y_pixels, area.bottom_y_pixels);
    canvas.text(
        text,
        x_pixels,
        middle + TEXT_CAP_HEIGHT_EMS * em_pixels / 2.0,
        em_pixels,
        Align::Left,
        color_linear_rgba,
    )?;
    Ok(())
}

/// Where and how large wrapped text is drawn.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WrappedText {
    pub left_pixels: f32,
    pub first_baseline_pixels: f32,
    pub line_step_pixels: f32,
    pub em_pixels: f32,
    pub max_width_pixels: f32,
    pub max_lines: usize,
}

/// `text` in as many lines as fit `max_width_pixels` (at most
/// `max_lines`, the last one cut short with an ellipsis if the text still
/// does not fit), the first baseline at `first_baseline_pixels`. Messages
/// are drawn whole, so what is said can be read and copied.
pub(crate) fn draw_wrapped(
    canvas: &mut Canvas<'_>,
    text: &str,
    place: WrappedText,
    color_linear_rgba: [f32; 4],
) -> Result<(), RenderError> {
    let WrappedText {
        left_pixels,
        first_baseline_pixels,
        line_step_pixels,
        em_pixels,
        max_width_pixels,
        max_lines,
    } = place;
    let mut lines: Vec<String> = Vec::new();
    let mut current: String = String::new();
    for word in text.split_whitespace() {
        let candidate: String = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if current.is_empty() || canvas.text_width(&candidate, em_pixels) <= max_width_pixels {
            current = candidate;
        } else {
            lines.push(std::mem::replace(&mut current, word.to_string()));
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.len() > max_lines {
        // The rest goes on the last line, which `text_fitted` cuts.
        let rest: String = lines.split_off(max_lines - 1).join(" ");
        lines.push(rest);
    }
    for (index, line) in lines.iter().enumerate() {
        canvas.text_fitted(
            line,
            left_pixels,
            first_baseline_pixels + line_step_pixels * rows_as_f32(index),
            em_pixels,
            max_width_pixels,
            color_linear_rgba,
        )?;
    }
    Ok(())
}

/// Where each menu's title sits in the bar.
fn menu_titles(canvas: &Canvas<'_>, layout: &ScreenLayout) -> Vec<(MenuId, ScreenRect)> {
    let unit: f32 = layout.unit_pixels;
    let mut x: f32 = 0.8 * unit;
    MenuId::ALL
        .iter()
        .map(|id: &MenuId| {
            let width: f32 = canvas.text_width(id.title(), 1.2 * unit) + 2.0 * unit;
            let area: ScreenRect = rect(
                x,
                layout.menu_bar.top_y_pixels + 0.3 * unit,
                width,
                layout.menu_bar.bottom_y_pixels - layout.menu_bar.top_y_pixels - 0.6 * unit,
            );
            x += width + 0.4 * unit;
            (*id, area)
        })
        .collect()
}

/// The bar of menu titles across the top.
///
/// # Errors
///
/// When the atlas fails to place text.
pub fn draw_menu_bar(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    shell: &Shell,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    canvas.fill(layout.menu_bar, colors::panel());
    canvas.fill(
        rect(
            0.0,
            layout.menu_bar.bottom_y_pixels - 1.0,
            layout.menu_bar.right_x_pixels,
            1.0,
        ),
        colors::border(),
    );
    if shell.copied() {
        canvas.text_in(
            "Message copied to the clipboard",
            rect(
                layout.menu_bar.right_x_pixels - 40.0 * unit,
                layout.menu_bar.top_y_pixels,
                39.0 * unit,
                layout.menu_bar.bottom_y_pixels - layout.menu_bar.top_y_pixels,
            ),
            1.1 * unit,
            colors::done(),
        )?;
    }
    let open: Option<MenuId> = shell.open_menu().map(|(id, _)| id);
    for (id, area) in menu_titles(canvas, layout) {
        if open == Some(id) {
            canvas.fill(area, colors::selection_fill());
            canvas.frame(area, 1.0, colors::selection_edge());
        }
        canvas.text_in(id.title(), area, 1.2 * unit, colors::text())?;
        canvas.hit(area, target(ShellTarget::Menu(id)));
    }
    Ok(())
}

/// The screen while no song is open: what to do, and the last message.
///
/// # Errors
///
/// When the atlas fails to place text.
pub fn draw_empty(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
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
    let center_x: f32 = f32::midpoint(body.left_x_pixels, body.right_x_pixels);
    let center_y: f32 = f32::midpoint(body.top_y_pixels, body.bottom_y_pixels);
    canvas.text(
        "No song open",
        center_x,
        center_y,
        2.4 * unit,
        Align::Center,
        colors::text(),
    )?;
    canvas.text(
        "File, Open song...  (Ctrl+O)",
        center_x,
        center_y + 3.2 * unit,
        1.4 * unit,
        Align::Center,
        colors::text_soft(),
    )?;
    draw_message_footer(
        canvas,
        layout,
        shell,
        "F10 menu   Ctrl+O open a song   Ctrl+Q quit",
    )
}

/// The footer: the last message, and below it the keys that work now.
///
/// # Errors
///
/// When the atlas fails to place text.
pub fn draw_message_footer(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    shell: &Shell,
    keys: &str,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    canvas.fill(layout.footer, colors::panel());
    if let Some((kind, text)) = shell.message() {
        let color: [f32; 4] = match kind {
            MessageKind::Done => colors::done(),
            MessageKind::Problem => colors::problem(),
        };
        canvas.hit(
            rect(
                0.0,
                layout.footer.top_y_pixels,
                layout.footer.right_x_pixels,
                3.4 * unit,
            ),
            HitTarget::CopyMessage(MessageSource::Footer),
        );
        draw_wrapped(
            canvas,
            text,
            WrappedText {
                left_pixels: 0.8 * unit,
                first_baseline_pixels: layout.footer.top_y_pixels + 1.6 * unit,
                line_step_pixels: 1.5 * unit,
                em_pixels: 1.2 * unit,
                max_width_pixels: layout.footer.right_x_pixels - 1.6 * unit,
                max_lines: 2,
            },
            color,
        )?;
    }
    canvas.text_fitted(
        keys,
        0.8 * unit,
        layout.footer.top_y_pixels + 4.9 * unit,
        1.05 * unit,
        layout.footer.right_x_pixels - 1.6 * unit,
        colors::text_soft(),
    )?;
    Ok(())
}

/// The open menu, or the dialog, on top of everything drawn so far.
///
/// # Errors
///
/// When the atlas fails to place text.
pub fn draw_overlays(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    shell: &Shell,
    session: Option<&Session>,
) -> Result<(), RenderError> {
    if let Some((id, cursor)) = shell.open_menu() {
        draw_menu(canvas, layout, shell, id, cursor)?;
    }
    match shell.modal() {
        Some(Modal::Browser(browser)) => draw_browser(canvas, layout, browser)?,
        Some(Modal::Confirm(confirmation)) => draw_confirmation(canvas, layout, confirmation)?,
        Some(Modal::Settings(view)) => screen_settings::draw(canvas, layout, view, session)?,
        // The browser is not drawn under the prompt: the canvas has one
        // overlay layer, so its text would show through the prompt.
        Some(Modal::NewFolder(prompt)) => crate::screen_new_folder::draw(canvas, layout, prompt)?,
        None => {}
    }
    Ok(())
}

fn draw_menu(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    shell: &Shell,
    id: MenuId,
    cursor: usize,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    let Some((_, title_area)) = menu_titles(canvas, layout)
        .into_iter()
        .find(|(candidate, _)| *candidate == id)
    else {
        return Ok(());
    };
    // A click anywhere below the bar closes the menu.
    canvas.hit(
        rect(
            0.0,
            layout.menu_bar.bottom_y_pixels,
            layout.footer.right_x_pixels,
            layout.footer.bottom_y_pixels - layout.menu_bar.bottom_y_pixels,
        ),
        target(ShellTarget::Outside),
    );
    let em: f32 = 1.2 * unit;
    let widest: f32 = id
        .items()
        .iter()
        .map(|command: &MenuCommand| {
            canvas.text_width(command.label(), em) + canvas.text_width(command.shortcut(), em)
        })
        .fold(0.0, f32::max);
    let width: f32 = widest + 6.0 * unit;
    let height: f32 = MENU_ROW_UNITS * unit * rows_as_f32(id.items().len()) + 0.8 * unit;
    let panel: ScreenRect = rect(
        title_area.left_x_pixels,
        layout.menu_bar.bottom_y_pixels,
        width,
        height,
    );
    canvas.fill(panel, colors::panel_raised());
    canvas.frame(panel, 1.0, colors::border());
    for (index, command) in id.items().iter().enumerate() {
        let row: ScreenRect = rect(
            panel.left_x_pixels + 0.2 * unit,
            panel.top_y_pixels + 0.4 * unit + MENU_ROW_UNITS * unit * rows_as_f32(index),
            width - 0.4 * unit,
            MENU_ROW_UNITS * unit,
        );
        let enabled: bool = command.enabled(shell.can_save(), shell.song_open());
        if index == cursor {
            canvas.fill(row, colors::selection_fill());
        }
        let color: [f32; 4] = if enabled {
            colors::text()
        } else {
            colors::text_faint()
        };
        text_left_in(
            canvas,
            command.label(),
            row.left_x_pixels + unit,
            row,
            em,
            color,
        )?;
        let middle: f32 = f32::midpoint(row.top_y_pixels, row.bottom_y_pixels);
        canvas.text(
            command.shortcut(),
            row.right_x_pixels - unit,
            middle + TEXT_CAP_HEIGHT_EMS * em / 2.0,
            em,
            Align::Right,
            colors::text_faint(),
        )?;
        canvas.hit(row, target(ShellTarget::MenuItem(*command)));
    }
    Ok(())
}

/// The window's whole area, dimmed, and a click on it that does nothing:
/// a dialog is answered on its own buttons.
pub(crate) fn dim_everything(canvas: &mut Canvas<'_>, layout: &ScreenLayout) {
    let all: ScreenRect = rect(
        0.0,
        0.0,
        layout.footer.right_x_pixels,
        layout.footer.bottom_y_pixels,
    );
    canvas.fill(all, colors::with_alpha(colors::srgb(0, 0, 0), 0.55));
    canvas.hit(all, target(ShellTarget::Outside));
}

pub(crate) fn dialog_area(
    layout: &ScreenLayout,
    width_units: f32,
    height_units: f32,
) -> ScreenRect {
    let unit: f32 = layout.unit_pixels;
    let screen_width: f32 = layout.footer.right_x_pixels;
    let screen_height: f32 = layout.footer.bottom_y_pixels;
    let width: f32 = (width_units * unit).min(screen_width - 4.0 * unit);
    let height: f32 = (height_units * unit).min(screen_height - 4.0 * unit);
    rect(
        (screen_width - width) / 2.0,
        (screen_height - height) / 2.0,
        width,
        height,
    )
}

/// A labelled button; the first choice of a dialog is drawn emphasised.
pub(crate) fn dialog_button(
    canvas: &mut Canvas<'_>,
    area: ScreenRect,
    label: &str,
    em: f32,
    emphasised: bool,
    click: ShellTarget,
) -> Result<(), RenderError> {
    if emphasised {
        canvas.fill(area, colors::selection_fill());
        canvas.frame(area, 1.0, colors::selection_edge());
    } else {
        canvas.frame(area, 1.0, colors::border());
    }
    canvas.text_in(label, area, em, colors::text())?;
    canvas.hit(area, target(click));
    Ok(())
}

fn draw_browser(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    browser: &FolderBrowser,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    dim_everything(canvas, layout);
    let dialog: ScreenRect = dialog_area(layout, BROWSER_WIDTH_UNITS, BROWSER_HEIGHT_UNITS);
    canvas.fill(dialog, colors::panel_raised());
    canvas.frame(dialog, 1.0, colors::border());
    let left: f32 = dialog.left_x_pixels + 1.5 * unit;
    let inner_width: f32 = dialog.right_x_pixels - dialog.left_x_pixels - 3.0 * unit;
    let choosing_folder: bool = browser.purpose().chooses_folder();
    let saving_as: bool = browser.purpose() == BrowserPurpose::ChooseFolderForSaveAs;
    canvas.text(
        if saving_as {
            "Save as: make or choose the folder for the new name"
        } else if choosing_folder {
            "New song: make or choose its folder"
        } else {
            "Open a song"
        },
        left,
        dialog.top_y_pixels + 2.8 * unit,
        1.6 * unit,
        Align::Left,
        colors::text(),
    )?;
    canvas.text_fitted(
        &browser.directory().display().to_string(),
        left,
        dialog.top_y_pixels + 5.0 * unit,
        1.1 * unit,
        inner_width - 18.0 * unit,
        colors::text_soft(),
    )?;
    if browser.entries().len() > VISIBLE_ROWS {
        let last_shown: usize = (browser.top() + VISIBLE_ROWS).min(browser.entries().len());
        canvas.text(
            &format!(
                "Rows {}-{} of {}",
                browser.top() + 1,
                last_shown,
                browser.entries().len()
            ),
            dialog.right_x_pixels - 1.5 * unit,
            dialog.top_y_pixels + 5.0 * unit,
            1.1 * unit,
            Align::Right,
            colors::text_faint(),
        )?;
    }
    if let Some(problem) = browser.problem() {
        canvas.text_fitted(
            problem,
            left,
            dialog.top_y_pixels + 6.8 * unit,
            1.1 * unit,
            inner_width,
            colors::problem(),
        )?;
    }
    let list: ScreenRect = rect(
        left,
        dialog.top_y_pixels + 7.6 * unit,
        inner_width,
        BROWSER_ROW_UNITS * unit * rows_as_f32(VISIBLE_ROWS) + 0.4 * unit,
    );
    canvas.frame(list, 1.0, colors::border());
    draw_browser_rows(canvas, list, unit, browser)?;
    let buttons_top: f32 = dialog.bottom_y_pixels - 3.6 * unit;
    text_left_in(
        canvas,
        if choosing_folder {
            if saving_as {
                "N new folder   U save in this folder   Esc cancels"
            } else {
                "N new folder   C copy the chosen song   U use this folder   Esc cancels"
            }
        } else {
            "Enter opens   Backspace up one folder   Esc cancels"
        },
        left,
        rect(
            left,
            if choosing_folder {
                buttons_top - 3.0 * unit
            } else {
                buttons_top
            },
            inner_width,
            2.4 * unit,
        ),
        1.05 * unit,
        colors::text_faint(),
    )?;
    draw_browser_buttons(
        canvas,
        unit,
        dialog,
        buttons_top,
        (choosing_folder, saving_as),
    )
}

fn draw_browser_buttons(
    canvas: &mut Canvas<'_>,
    unit: f32,
    dialog: ScreenRect,
    buttons_top: f32,
    (choosing_folder, saving_as): (bool, bool),
) -> Result<(), RenderError> {
    if choosing_folder {
        let left: f32 = dialog.left_x_pixels + 1.5 * unit;
        let mut x: f32 = left;
        for (label, width_units, emphasised, target) in [
            ("New folder (N)", 15.0, false, ShellTarget::BrowserNewFolder),
            ("Copy song (C)", 15.0, false, ShellTarget::BrowserCopy),
            (
                if saving_as {
                    "Save here (U)"
                } else {
                    "Use this folder (U)"
                },
                19.0,
                true,
                ShellTarget::BrowserChoose,
            ),
        ] {
            if saving_as && label == "Copy song (C)" {
                continue;
            }
            dialog_button(
                canvas,
                rect(x, buttons_top, width_units * unit, 2.4 * unit),
                label,
                1.2 * unit,
                emphasised,
                target,
            )?;
            x += (width_units + 1.0) * unit;
        }
    }
    dialog_button(
        canvas,
        rect(
            dialog.right_x_pixels - 1.5 * unit - 9.0 * unit,
            buttons_top,
            9.0 * unit,
            2.4 * unit,
        ),
        "Cancel",
        1.2 * unit,
        false,
        ShellTarget::BrowserCancel,
    )
}

fn draw_browser_rows(
    canvas: &mut Canvas<'_>,
    list: ScreenRect,
    unit: f32,
    browser: &FolderBrowser,
) -> Result<(), RenderError> {
    let row_height: f32 = BROWSER_ROW_UNITS * unit;
    let em: f32 = 1.2 * unit;
    let count: usize = browser.entries().len();
    let scrolls: bool = count > VISIBLE_ROWS;
    // Rows leave room on the right for the scroll bar when there is one.
    let bar_room: f32 = if scrolls {
        (SCROLL_BAR_WIDTH_UNITS + 0.8) * unit
    } else {
        0.0
    };
    for (slot, (index, entry)) in browser
        .entries()
        .iter()
        .enumerate()
        .skip(browser.top())
        .take(VISIBLE_ROWS)
        .enumerate()
    {
        let row: ScreenRect = rect(
            list.left_x_pixels + 0.2 * unit,
            list.top_y_pixels + 0.2 * unit + row_height * rows_as_f32(slot),
            list.right_x_pixels - list.left_x_pixels - 0.4 * unit - bar_room,
            row_height,
        );
        if index == browser.selected() {
            canvas.fill(row, colors::selection_fill());
            canvas.frame(row, 1.0, colors::selection_edge());
        }
        draw_browser_row(canvas, row, unit, em, entry)?;
        canvas.hit(row, target(ShellTarget::BrowserRow(index)));
    }
    if scrolls {
        draw_scroll_bar(
            canvas,
            list,
            unit,
            browser.list(),
            target(ShellTarget::BrowserPage { up: true }),
            target(ShellTarget::BrowserPage { up: false }),
        );
    }
    if browser
        .entries()
        .iter()
        .all(|entry: &BrowserEntry| entry.kind == BrowserEntryKind::ParentFolder)
    {
        text_left_in(
            canvas,
            "No song folders or .song files here.",
            list.left_x_pixels + unit,
            rect(
                list.left_x_pixels,
                list.top_y_pixels + 0.2 * unit + row_height,
                list.right_x_pixels - list.left_x_pixels,
                row_height,
            ),
            em,
            colors::text_faint(),
        )?;
    }
    Ok(())
}

/// The bar down the list's right edge: a thumb as long as the share of the
/// rows in view, placed by how far down the view is. A click above or below
/// the thumb moves one page (`page_up` and `page_down` are what those
/// clicks ask for).
pub fn draw_scroll_bar(
    canvas: &mut Canvas<'_>,
    list: ScreenRect,
    unit: f32,
    rows: &ScrollList,
    page_up: HitTarget,
    page_down: HitTarget,
) {
    let track: ScreenRect = rect(
        list.right_x_pixels - (SCROLL_BAR_WIDTH_UNITS + 0.4) * unit,
        list.top_y_pixels + 0.2 * unit,
        SCROLL_BAR_WIDTH_UNITS * unit,
        list.bottom_y_pixels - list.top_y_pixels - 0.4 * unit,
    );
    canvas.fill(track, colors::panel());
    let track_height: f32 = track.bottom_y_pixels - track.top_y_pixels;
    let count: f32 = rows_as_f32(rows.len());
    // rows in view ÷ rows in all = the share of the track the thumb fills.
    let thumb_height: f32 = (track_height * rows_as_f32(rows.visible_rows()) / count)
        .clamp((2.0 * unit).min(track_height), track_height);
    // rows above the view ÷ rows that can be above it = how far down the
    // free part of the track the thumb sits.
    let scrollable_rows: f32 = rows_as_f32(rows.len().saturating_sub(rows.visible_rows()).max(1));
    let thumb_top: f32 = track.top_y_pixels
        + (track_height - thumb_height) * rows_as_f32(rows.top()) / scrollable_rows;
    let thumb: ScreenRect = rect(
        track.left_x_pixels,
        thumb_top,
        track.right_x_pixels - track.left_x_pixels,
        thumb_height,
    );
    canvas.fill(thumb, colors::text_faint());
    canvas.hit(
        rect(
            track.left_x_pixels,
            track.top_y_pixels,
            track.right_x_pixels - track.left_x_pixels,
            thumb_top - track.top_y_pixels,
        ),
        page_up,
    );
    canvas.hit(
        rect(
            track.left_x_pixels,
            thumb.bottom_y_pixels,
            track.right_x_pixels - track.left_x_pixels,
            track.bottom_y_pixels - thumb.bottom_y_pixels,
        ),
        page_down,
    );
}

fn draw_browser_row(
    canvas: &mut Canvas<'_>,
    row: ScreenRect,
    unit: f32,
    em: f32,
    entry: &BrowserEntry,
) -> Result<(), RenderError> {
    let (label, tag, color): (String, &str, [f32; 4]) = match entry.kind {
        BrowserEntryKind::ParentFolder => {
            (String::from(".."), "up one folder", colors::text_soft())
        }
        BrowserEntryKind::Folder => (format!("{}/", entry.name), "", colors::text_soft()),
        BrowserEntryKind::SongFolder => (entry.name.clone(), "song", colors::text()),
        BrowserEntryKind::SongFile => (entry.name.clone(), "song file", colors::text()),
    };
    text_left_in(canvas, &label, row.left_x_pixels + unit, row, em, color)?;
    if !tag.is_empty() {
        let middle: f32 = f32::midpoint(row.top_y_pixels, row.bottom_y_pixels);
        canvas.text(
            tag,
            row.right_x_pixels - unit,
            middle + TEXT_CAP_HEIGHT_EMS * 1.0 * unit / 2.0,
            unit,
            Align::Right,
            colors::text_faint(),
        )?;
    }
    Ok(())
}

fn draw_confirmation(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    confirmation: &Confirmation,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    dim_everything(canvas, layout);
    let dialog: ScreenRect = dialog_area(layout, CONFIRM_WIDTH_UNITS, CONFIRM_HEIGHT_UNITS);
    canvas.fill(dialog, colors::panel_raised());
    canvas.frame(dialog, 1.0, colors::border());
    let left: f32 = dialog.left_x_pixels + 1.5 * unit;
    canvas.text_fitted(
        &confirmation.question,
        left,
        dialog.top_y_pixels + 4.2 * unit,
        1.3 * unit,
        dialog.right_x_pixels - dialog.left_x_pixels - 3.0 * unit,
        colors::text(),
    )?;
    let buttons_top: f32 = dialog.bottom_y_pixels - 4.2 * unit;
    let button_width: f32 = 11.0 * unit;
    let yes_left: f32 = dialog.right_x_pixels - 1.5 * unit - button_width;
    dialog_button(
        canvas,
        rect(yes_left, buttons_top, button_width, 2.6 * unit),
        &confirmation.yes_label,
        1.2 * unit,
        true,
        ShellTarget::ConfirmYes,
    )?;
    dialog_button(
        canvas,
        rect(
            yes_left - unit - button_width,
            buttons_top,
            button_width,
            2.6 * unit,
        ),
        &confirmation.no_label,
        1.2 * unit,
        false,
        ShellTarget::ConfirmNo,
    )?;
    text_left_in(
        canvas,
        "Enter says yes   Esc says no",
        left,
        rect(left, buttons_top, 20.0 * unit, 2.6 * unit),
        1.0 * unit,
        colors::text_faint(),
    )
}
