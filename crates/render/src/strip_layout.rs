//! Spike layout: the 8-bar strip's staff, barlines and playhead as line
//! records, and the geometry the notes are placed by. No GPU here, so it can
//! be tested directly. The real layout will take the published stream
//! instead of a playhead fraction.

use crate::line_instance::LineInstance;
use crate::theme;

/// Bars shown in the strip.
pub const BARS_IN_STRIP: u16 = 8;

/// Staff lines in a five-line staff.
const STAFF_LINES: u16 = 5;

/// Staff line spacing as a fraction of window height (1/60), so the display
/// scales with the window (EFS-1.12).
const STAFF_SPACE_FRACTION_OF_HEIGHT: f32 = 1.0 / 60.0;

/// Left and right margin as a fraction of window width.
const MARGIN_FRACTION_OF_WIDTH: f32 = 0.04;

/// Staff top as a fraction of window height.
const STAFF_TOP_FRACTION_OF_HEIGHT: f32 = 0.45;

/// Room before the first bar for the clef and time signature, in staff
/// spaces: G clef 2.7 spaces wide, time signature 1.8, and gaps.
const CLEF_AND_TIME_SPACES: f32 = 7.0;

/// Line thicknesses in staff spaces, from Bravura's engraving defaults
/// (`Bravura.json`: staffLineThickness, thinBarlineThickness, stemThickness).
const STAFF_LINE_THICKNESS_SPACES: f32 = 0.13;
const BARLINE_THICKNESS_SPACES: f32 = 0.16;
pub const STEM_THICKNESS_SPACES: f32 = 0.12;
const PLAYHEAD_THICKNESS_SPACES: f32 = 0.2;

/// How far the playhead extends above and below the staff, in staff spaces.
const PLAYHEAD_OVERHANG_SPACES: f32 = 3.0;

/// Where the strip's parts are in the window, in pixels. Computed once per
/// frame from the window size; the lines and the notes share it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StripGeometry {
    /// Whole pixels, so the five lines are evenly spaced at every size.
    pub space_pixels: f32,
    pub staff_left_pixels: f32,
    pub staff_right_pixels: f32,
    /// Where the first bar starts, after the clef and time signature.
    pub music_left_pixels: f32,
    pub bar_width_pixels: f32,
    /// Centre of the top staff line.
    pub staff_top_pixels: f32,
    /// Centre of the bottom staff line.
    pub staff_bottom_pixels: f32,
    staff_line_thickness_pixels: f32,
}

impl StripGeometry {
    #[must_use]
    pub fn new(width_pixels: f32, height_pixels: f32) -> StripGeometry {
        let space_pixels: f32 = (height_pixels * STAFF_SPACE_FRACTION_OF_HEIGHT)
            .round()
            .max(1.0);
        let staff_left_pixels: f32 = (width_pixels * MARGIN_FRACTION_OF_WIDTH).round();
        let staff_right_pixels: f32 = width_pixels - staff_left_pixels;
        let music_left_pixels: f32 =
            staff_left_pixels + (space_pixels * CLEF_AND_TIME_SPACES).round();
        let staff_line_thickness_pixels: f32 =
            whole_thickness_pixels(space_pixels * STAFF_LINE_THICKNESS_SPACES);
        let staff_top_pixels: f32 = snapped_center_pixels(
            (height_pixels * STAFF_TOP_FRACTION_OF_HEIGHT).round(),
            staff_line_thickness_pixels,
        );
        StripGeometry {
            space_pixels,
            staff_left_pixels,
            staff_right_pixels,
            music_left_pixels,
            bar_width_pixels: (staff_right_pixels - music_left_pixels) / f32::from(BARS_IN_STRIP),
            staff_top_pixels,
            staff_bottom_pixels: staff_top_pixels + space_pixels * f32::from(STAFF_LINES - 1),
            staff_line_thickness_pixels,
        }
    }

    /// The height of a staff step: half staff spaces up from the bottom line
    /// (0 the bottom line, 1 the space above it, 8 the top line).
    #[must_use]
    pub fn step_y_pixels(&self, staff_step: i8) -> f32 {
        self.staff_bottom_pixels - f32::from(staff_step) * self.space_pixels / 2.0
    }

    /// Where bar `bar` (0 the first) starts.
    #[must_use]
    pub fn bar_left_pixels(&self, bar: u16) -> f32 {
        self.music_left_pixels + self.bar_width_pixels * f32::from(bar)
    }

    /// One em of the music font: 4 staff spaces (SMuFL).
    #[must_use]
    pub fn music_em_pixels(&self) -> f32 {
        4.0 * self.space_pixels
    }
}

/// Lines for the strip, with the playhead `playhead_fraction` of the way
/// through the 8 bars (0.0 to 1.0).
#[must_use]
pub fn strip_lines(geometry: &StripGeometry, playhead_fraction: f32) -> Vec<LineInstance> {
    let mut lines: Vec<LineInstance> = Vec::new();
    for staff_line in 0..STAFF_LINES {
        let y_pixels: f32 =
            geometry.staff_top_pixels + geometry.space_pixels * f32::from(staff_line);
        lines.push(LineInstance {
            start_x_pixels: geometry.staff_left_pixels,
            start_y_pixels: y_pixels,
            end_x_pixels: geometry.staff_right_pixels,
            end_y_pixels: y_pixels,
            thickness_pixels: geometry.staff_line_thickness_pixels,
            color_linear_rgba: theme::STAFF_LINE,
        });
    }
    // The staff's opening line, then one barline at the end of each bar.
    let barline_thickness_pixels: f32 =
        whole_thickness_pixels(geometry.space_pixels * BARLINE_THICKNESS_SPACES);
    let mut barline_xs_pixels: Vec<f32> = vec![geometry.staff_left_pixels];
    for bar in 1..=BARS_IN_STRIP {
        barline_xs_pixels.push(geometry.bar_left_pixels(bar));
    }
    for x_pixels in barline_xs_pixels {
        lines.push(vertical_line(
            snapped_center_pixels(x_pixels, barline_thickness_pixels),
            geometry.staff_top_pixels,
            geometry.staff_bottom_pixels,
            barline_thickness_pixels,
            theme::BARLINE,
        ));
    }
    // The playhead moves smoothly, so only its thickness is whole pixels.
    let playhead_x_pixels: f32 = geometry.music_left_pixels
        + (geometry.staff_right_pixels - geometry.music_left_pixels) * playhead_fraction;
    lines.push(vertical_line(
        playhead_x_pixels,
        geometry.staff_top_pixels - geometry.space_pixels * PLAYHEAD_OVERHANG_SPACES,
        geometry.staff_bottom_pixels + geometry.space_pixels * PLAYHEAD_OVERHANG_SPACES,
        whole_thickness_pixels(geometry.space_pixels * PLAYHEAD_THICKNESS_SPACES),
        theme::PLAYHEAD,
    ));
    lines
}

/// A vertical line from `top_pixels` down to `bottom_pixels`.
#[must_use]
pub fn vertical_line(
    x_pixels: f32,
    top_pixels: f32,
    bottom_pixels: f32,
    thickness_pixels: f32,
    color_linear_rgba: [f32; 4],
) -> LineInstance {
    LineInstance {
        start_x_pixels: x_pixels,
        start_y_pixels: top_pixels,
        end_x_pixels: x_pixels,
        end_y_pixels: bottom_pixels,
        thickness_pixels,
        color_linear_rgba,
    }
}

/// A thickness of whole pixels, at least 1: a line thinner than a pixel can
/// miss every pixel centre and vanish, as lines have no antialiasing.
#[must_use]
pub fn whole_thickness_pixels(thickness_pixels: f32) -> f32 {
    thickness_pixels.round().max(1.0)
}

/// The centre that puts a line's two edges on pixel boundaries, so it fills
/// whole pixels and is not smeared or dropped. `center_pixels` is where the
/// line should be; the result is within half a pixel of it.
#[must_use]
pub fn snapped_center_pixels(center_pixels: f32, thickness_pixels: f32) -> f32 {
    let half_thickness_pixels: f32 = thickness_pixels / 2.0;
    (center_pixels - half_thickness_pixels).round() + half_thickness_pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_has_five_staff_lines_nine_barlines_and_one_playhead() {
        let lines: Vec<LineInstance> = strip_lines(&StripGeometry::new(2560.0, 1440.0), 0.5);
        assert_eq!(lines.len(), 5 + 9 + 1);
    }

    #[test]
    fn playhead_runs_from_the_first_bar_to_the_last_barline() {
        let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0);
        let at_start: Vec<LineInstance> = strip_lines(&geometry, 0.0);
        let at_end: Vec<LineInstance> = strip_lines(&geometry, 1.0);
        let last_barline_x_pixels: f32 = at_start[13].start_x_pixels;
        assert!((at_start[14].start_x_pixels - geometry.music_left_pixels).abs() < 0.001);
        // Barlines are snapped to whole pixels, so up to half a pixel off.
        assert!((at_end[14].start_x_pixels - last_barline_x_pixels).abs() <= 0.5);
    }

    #[test]
    fn staff_lines_are_evenly_spaced_and_whole_pixels_at_every_window_height() {
        for height in 400_u16..=2160 {
            let lines: Vec<LineInstance> =
                strip_lines(&StripGeometry::new(2560.0, f32::from(height)), 0.0);
            let first_gap_pixels: f32 = lines[1].start_y_pixels - lines[0].start_y_pixels;
            for pair in lines[..5].windows(2) {
                let gap_pixels: f32 = pair[1].start_y_pixels - pair[0].start_y_pixels;
                assert!(
                    (gap_pixels - first_gap_pixels).abs() < 0.001,
                    "height {height}"
                );
            }
            for line in &lines[..5] {
                let top_edge_pixels: f32 = line.start_y_pixels - line.thickness_pixels / 2.0;
                assert!((top_edge_pixels - top_edge_pixels.round()).abs() < 0.001);
                assert!((line.thickness_pixels - line.thickness_pixels.round()).abs() < 0.001);
                assert!(line.thickness_pixels >= 1.0);
            }
        }
    }

    #[test]
    fn staff_steps_land_on_the_staff_lines() {
        let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0);
        let lines: Vec<LineInstance> = strip_lines(&geometry, 0.0);
        // Step 0 is the bottom line (the fifth drawn), step 8 the top line.
        assert!((geometry.step_y_pixels(0) - lines[4].start_y_pixels).abs() < 0.001);
        assert!((geometry.step_y_pixels(8) - lines[0].start_y_pixels).abs() < 0.001);
    }
}
