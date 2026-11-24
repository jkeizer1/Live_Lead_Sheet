//! The name prompt over the new-song browser: for a new folder, or for a
//! copy of a song folder. A text field for the name, and why the last try
//! failed. Sizes are in the screen's
//! units (window height ÷ 90).

use composer::shell::{NewFolderPrompt, ShellTarget};

use crate::canvas::{Align, Canvas, HitTarget, MessageSource, rect};
use crate::error::RenderError;
use crate::screen::ScreenLayout;
use crate::screen_shell::{WrappedText, dialog_area, dialog_button, dim_everything, draw_wrapped};
use crate::screen_theme as colors;
use crate::strip_layout::{ScreenRect, TEXT_CAP_HEIGHT_EMS};

const WIDTH_UNITS: f32 = 70.0;
const HEIGHT_UNITS: f32 = 24.0;

/// # Errors
///
/// When the atlas fails to place text.
pub fn draw(
    canvas: &mut Canvas<'_>,
    layout: &ScreenLayout,
    prompt: &NewFolderPrompt,
) -> Result<(), RenderError> {
    let unit: f32 = layout.unit_pixels;
    dim_everything(canvas, layout);
    let dialog: ScreenRect = dialog_area(layout, WIDTH_UNITS, HEIGHT_UNITS);
    canvas.fill(dialog, colors::panel_raised());
    canvas.frame(dialog, 1.0, colors::border());
    let left: f32 = dialog.left_x_pixels + 1.5 * unit;
    let inner_width: f32 = dialog.right_x_pixels - dialog.left_x_pixels - 3.0 * unit;
    let copy_of: Option<String> = copied_song_name(prompt);
    canvas.text(
        if copy_of.is_some() {
            "Copy a song"
        } else {
            "New folder"
        },
        left,
        dialog.top_y_pixels + 2.8 * unit,
        1.6 * unit,
        Align::Left,
        colors::text(),
    )?;
    canvas.text_fitted(
        &explanation(prompt, copy_of.as_deref()),
        left,
        dialog.top_y_pixels + 5.0 * unit,
        1.1 * unit,
        inner_width,
        colors::text_soft(),
    )?;
    let box_area: ScreenRect = rect(
        left,
        dialog.top_y_pixels + 6.4 * unit,
        inner_width - 22.0 * unit,
        2.8 * unit,
    );
    canvas.fill(box_area, colors::panel());
    canvas.frame(box_area, 1.0, colors::selection_edge());
    let em: f32 = 1.3 * unit;
    let baseline: f32 = f32::midpoint(box_area.top_y_pixels, box_area.bottom_y_pixels)
        + TEXT_CAP_HEIGHT_EMS * em / 2.0;
    let text_left: f32 = box_area.left_x_pixels + unit;
    canvas.text(
        prompt.field.text(),
        text_left,
        baseline,
        em,
        Align::Left,
        colors::text(),
    )?;
    let cursor_x: f32 = text_left + canvas.text_width(prompt.field.text_before_cursor(), em);
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
        if copy_of.is_some() { "Copy" } else { "Make" },
        1.2 * unit,
        true,
        ShellTarget::NewFolderAccept,
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
        ShellTarget::NewFolderCancel,
    )?;
    if let Some(message) = &prompt.message {
        draw_message(
            canvas,
            unit,
            message,
            left,
            inner_width,
            box_area.bottom_y_pixels,
        )?;
    }
    Ok(())
}

/// Why the last try failed; clicking it copies it, like the other dialogs'.
fn draw_message(
    canvas: &mut Canvas<'_>,
    unit: f32,
    message: &str,
    left: f32,
    width: f32,
    field_bottom: f32,
) -> Result<(), RenderError> {
    canvas.hit(
        rect(left, field_bottom + 0.6 * unit, width, 4.6 * unit),
        HitTarget::CopyMessage(MessageSource::Dialog),
    );
    draw_wrapped(
        canvas,
        message,
        WrappedText {
            left_pixels: left,
            first_baseline_pixels: field_bottom + 2.4 * unit,
            line_step_pixels: 1.5 * unit,
            em_pixels: 1.15 * unit,
            max_width_pixels: width,
            max_lines: 3,
        },
        colors::problem(),
    )
}

/// The name of the song folder being copied, when the prompt is for a copy.
fn copied_song_name(prompt: &NewFolderPrompt) -> Option<String> {
    prompt.copy_from.as_ref().and_then(|source| {
        source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    })
}

/// What the name is for, and where the folder will be made.
fn explanation(prompt: &NewFolderPrompt, copy_of: Option<&str>) -> String {
    let directory: String = prompt.browser.directory().display().to_string();
    match copy_of {
        Some(old) => format!(
            "Name of the copy of {old}. Its folder and files get the name; made in {directory}"
        ),
        None => format!("Name of the new folder, made in {directory}"),
    }
}
