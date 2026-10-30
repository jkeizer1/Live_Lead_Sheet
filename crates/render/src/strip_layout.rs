//! Spike layout of one part's area across the 8-bar strip (EFS-1.5), top to
//! bottom: chord symbols, the rhythm line (a one-line staff), and the pitch
//! staff, which the musician can hide. This module places the lines and the
//! hide/show control; `spike_music` places the music on them. No GPU here,
//! so it can be tested directly.

use crate::error::RenderError;
use crate::glyph_atlas::{GlyphAtlas, GlyphFont};
use crate::glyph_instance::GlyphInstance;
use crate::line_instance::LineInstance;
use crate::theme;

/// Bars shown in the strip.
pub const BARS_IN_STRIP: u16 = 8;

/// Staff lines in the pitch staff.
const STAFF_LINES: u16 = 5;

/// Pitch staff line spacing as a fraction of window height (1/60), so the
/// display scales with the window (EFS-1.12).
const STAFF_SPACE_FRACTION_OF_HEIGHT: f32 = 1.0 / 60.0;

/// The rhythm line is drawn smaller than the pitch staff, so eight slashes
/// fit a bar: its staff space is 3/4 of the pitch staff's.
const RHYTHM_SPACE_FRACTION_OF_STAFF_SPACE: f32 = 0.75;

/// Left and right margin as a fraction of window width.
const MARGIN_FRACTION_OF_WIDTH: f32 = 0.04;

/// Vertical layout. The chord symbols' baseline, as a fraction of window
/// height; everything else in staff spaces from the layer above.
const CHORD_BASELINE_FRACTION_OF_HEIGHT: f32 = 0.24;
const RHYTHM_LINE_BELOW_CHORD_BASELINE_SPACES: f32 = 5.5;
const PITCH_STAFF_BELOW_RHYTHM_LINE_SPACES: f32 = 5.0;
/// The part's area reaches above the chord symbols and below the pitch
/// staff (ledger lines); the playhead spans it.
const AREA_ABOVE_CHORD_BASELINE_SPACES: f32 = 2.5;
const AREA_BELOW_PITCH_STAFF_SPACES: f32 = 3.0;
const AREA_BELOW_CONTROL_SPACES: f32 = 1.0;

/// Room before the first bar for clefs and time signatures, in staff
/// spaces: G clef 2.7 spaces wide, time signature 1.8, and gaps.
const CLEF_AND_TIME_SPACES: f32 = 7.0;

/// The hide/show control: below the rhythm line, at the left end, in staff
/// spaces. Wide enough for "Show staff" at the label's size.
const CONTROL_BELOW_RHYTHM_LINE_SPACES: f32 = 1.8;
const CONTROL_HEIGHT_SPACES: f32 = 1.8;
const CONTROL_WIDTH_SPACES: f32 = 7.5;
const CONTROL_LABEL_EM_SPACES: f32 = 1.2;
const CONTROL_BORDER_THICKNESS_PIXELS: f32 = 1.0;

/// Line thicknesses in staff spaces, from Bravura's engraving defaults
/// (`Bravura.json`: staffLineThickness, thinBarlineThickness, stemThickness).
const STAFF_LINE_THICKNESS_SPACES: f32 = 0.13;
const BARLINE_THICKNESS_SPACES: f32 = 0.16;
pub const STEM_THICKNESS_SPACES: f32 = 0.12;
const PLAYHEAD_THICKNESS_SPACES: f32 = 0.2;

/// Rhythm-line barlines reach this far above and below the line, in rhythm
/// staff spaces: a little beyond the slashes (±1).
const RHYTHM_BARLINE_HALF_HEIGHT_RHYTHM_SPACES: f32 = 1.5;

/// DejaVu Sans capital height: 1493 font units ÷ 2048 font units/em.
pub const TEXT_CAP_HEIGHT_EMS: f32 = 0.729;

/// A rectangle on screen, in window pixels from the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    pub left_x_pixels: f32,
    pub top_y_pixels: f32,
    pub right_x_pixels: f32,
    pub bottom_y_pixels: f32,
}

impl ScreenRect {
    #[must_use]
    pub fn contains(&self, x_pixels: f32, y_pixels: f32) -> bool {
        (self.left_x_pixels..=self.right_x_pixels).contains(&x_pixels)
            && (self.top_y_pixels..=self.bottom_y_pixels).contains(&y_pixels)
    }
}

/// Where the pitch staff is, when shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PitchStaffPlace {
    /// Centre of the top staff line.
    pub top_y_pixels: f32,
    /// Centre of the bottom staff line.
    pub bottom_y_pixels: f32,
    pub space_pixels: f32,
}

impl PitchStaffPlace {
    /// The height of a staff step: half staff spaces up from the bottom line
    /// (0 the bottom line E4, 8 the top line F5, −2 the C4 ledger line).
    #[must_use]
    pub fn step_y_pixels(&self, staff_step: i8) -> f32 {
        self.bottom_y_pixels - f32::from(staff_step) * self.space_pixels / 2.0
    }
}

/// Where the part's parts are in the window, in pixels. Computed once per
/// frame from the window size; the lines, the music and the control share
/// it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StripGeometry {
    /// The pitch staff's space; whole pixels, so its lines are evenly spaced.
    pub space_pixels: f32,
    /// The rhythm line's smaller staff space.
    pub rhythm_space_pixels: f32,
    pub staff_left_pixels: f32,
    pub staff_right_pixels: f32,
    /// Where the first bar starts, after the clefs and time signatures.
    pub music_left_pixels: f32,
    pub bar_width_pixels: f32,
    pub chord_baseline_y_pixels: f32,
    /// Centre of the rhythm line.
    pub rhythm_line_y_pixels: f32,
    /// `None` while the musician has hidden it.
    pub pitch_staff: Option<PitchStaffPlace>,
    /// The hide/show control; the app hit-tests clicks against it.
    pub staff_control: ScreenRect,
    pub area_top_y_pixels: f32,
    pub area_bottom_y_pixels: f32,
    staff_line_thickness_pixels: f32,
}

impl StripGeometry {
    #[must_use]
    pub fn new(width_pixels: f32, height_pixels: f32, pitch_staff_shown: bool) -> StripGeometry {
        let space_pixels: f32 = (height_pixels * STAFF_SPACE_FRACTION_OF_HEIGHT)
            .round()
            .max(1.0);
        let staff_left_pixels: f32 = (width_pixels * MARGIN_FRACTION_OF_WIDTH).round();
        let staff_right_pixels: f32 = width_pixels - staff_left_pixels;
        let music_left_pixels: f32 =
            staff_left_pixels + (space_pixels * CLEF_AND_TIME_SPACES).round();
        let staff_line_thickness_pixels: f32 =
            whole_thickness_pixels(space_pixels * STAFF_LINE_THICKNESS_SPACES);
        let chord_baseline_y_pixels: f32 =
            (height_pixels * CHORD_BASELINE_FRACTION_OF_HEIGHT).round();
        let rhythm_line_y_pixels: f32 = snapped_center_pixels(
            chord_baseline_y_pixels + RHYTHM_LINE_BELOW_CHORD_BASELINE_SPACES * space_pixels,
            staff_line_thickness_pixels,
        );
        let control_top_y_pixels: f32 =
            (rhythm_line_y_pixels + CONTROL_BELOW_RHYTHM_LINE_SPACES * space_pixels).round();
        let staff_control: ScreenRect = ScreenRect {
            left_x_pixels: staff_left_pixels,
            top_y_pixels: control_top_y_pixels,
            right_x_pixels: staff_left_pixels + (CONTROL_WIDTH_SPACES * space_pixels).round(),
            bottom_y_pixels: control_top_y_pixels + (CONTROL_HEIGHT_SPACES * space_pixels).round(),
        };
        let pitch_staff: Option<PitchStaffPlace> = if pitch_staff_shown {
            let top_y_pixels: f32 = snapped_center_pixels(
                rhythm_line_y_pixels + PITCH_STAFF_BELOW_RHYTHM_LINE_SPACES * space_pixels,
                staff_line_thickness_pixels,
            );
            Some(PitchStaffPlace {
                top_y_pixels,
                bottom_y_pixels: top_y_pixels + space_pixels * f32::from(STAFF_LINES - 1),
                space_pixels,
            })
        } else {
            None
        };
        let area_bottom_y_pixels: f32 = match pitch_staff {
            Some(place) => place.bottom_y_pixels + AREA_BELOW_PITCH_STAFF_SPACES * space_pixels,
            None => staff_control.bottom_y_pixels + AREA_BELOW_CONTROL_SPACES * space_pixels,
        };
        StripGeometry {
            space_pixels,
            rhythm_space_pixels: space_pixels * RHYTHM_SPACE_FRACTION_OF_STAFF_SPACE,
            staff_left_pixels,
            staff_right_pixels,
            music_left_pixels,
            bar_width_pixels: (staff_right_pixels - music_left_pixels) / f32::from(BARS_IN_STRIP),
            chord_baseline_y_pixels,
            rhythm_line_y_pixels,
            pitch_staff,
            staff_control,
            area_top_y_pixels: chord_baseline_y_pixels
                - AREA_ABOVE_CHORD_BASELINE_SPACES * space_pixels,
            area_bottom_y_pixels,
            staff_line_thickness_pixels,
        }
    }

    /// Where bar `bar` (0 the first) starts.
    #[must_use]
    pub fn bar_left_pixels(&self, bar: u16) -> f32 {
        self.music_left_pixels + self.bar_width_pixels * f32::from(bar)
    }

    /// One em of the music font on the pitch staff: 4 staff spaces (SMuFL).
    #[must_use]
    pub fn music_em_pixels(&self) -> f32 {
        4.0 * self.space_pixels
    }

    /// One em of the music font on the rhythm line.
    #[must_use]
    pub fn rhythm_em_pixels(&self) -> f32 {
        4.0 * self.rhythm_space_pixels
    }
}

/// Lines for the part: rhythm line, pitch staff (when shown), barlines, the
/// control's border, and the playhead `playhead_fraction` of the way through
/// the 8 bars (0.0 to 1.0), drawn last.
#[must_use]
pub fn strip_lines(geometry: &StripGeometry, playhead_fraction: f32) -> Vec<LineInstance> {
    let mut lines: Vec<LineInstance> = Vec::new();
    let barline_thickness_pixels: f32 =
        whole_thickness_pixels(geometry.space_pixels * BARLINE_THICKNESS_SPACES);
    // The staff's opening line, then one barline at the end of each bar.
    let mut barline_xs_pixels: Vec<f32> = vec![geometry.staff_left_pixels];
    for bar in 1..=BARS_IN_STRIP {
        barline_xs_pixels.push(snapped_center_pixels(
            geometry.bar_left_pixels(bar),
            barline_thickness_pixels,
        ));
    }

    lines.push(horizontal_line(
        geometry.staff_left_pixels,
        geometry.staff_right_pixels,
        geometry.rhythm_line_y_pixels,
        geometry.staff_line_thickness_pixels,
        theme::STAFF_LINE,
    ));
    let rhythm_barline_half_pixels: f32 =
        RHYTHM_BARLINE_HALF_HEIGHT_RHYTHM_SPACES * geometry.rhythm_space_pixels;
    for x_pixels in &barline_xs_pixels {
        lines.push(vertical_line(
            *x_pixels,
            geometry.rhythm_line_y_pixels - rhythm_barline_half_pixels,
            geometry.rhythm_line_y_pixels + rhythm_barline_half_pixels,
            barline_thickness_pixels,
            theme::BARLINE,
        ));
    }

    if let Some(staff) = geometry.pitch_staff {
        for staff_line in 0..STAFF_LINES {
            lines.push(horizontal_line(
                geometry.staff_left_pixels,
                geometry.staff_right_pixels,
                staff.top_y_pixels + staff.space_pixels * f32::from(staff_line),
                geometry.staff_line_thickness_pixels,
                theme::STAFF_LINE,
            ));
        }
        for x_pixels in &barline_xs_pixels {
            lines.push(vertical_line(
                *x_pixels,
                staff.top_y_pixels,
                staff.bottom_y_pixels,
                barline_thickness_pixels,
                theme::BARLINE,
            ));
        }
    }

    // The control's border: edges on pixel boundaries (half-pixel centres
    // for a 1 px line).
    let control: ScreenRect = geometry.staff_control;
    let half_pixels: f32 = CONTROL_BORDER_THICKNESS_PIXELS / 2.0;
    let (left, top, right, bottom): (f32, f32, f32, f32) = (
        control.left_x_pixels + half_pixels,
        control.top_y_pixels + half_pixels,
        control.right_x_pixels - half_pixels,
        control.bottom_y_pixels - half_pixels,
    );
    for (start_x, start_y, end_x, end_y) in [
        (left - half_pixels, top, right + half_pixels, top),
        (left - half_pixels, bottom, right + half_pixels, bottom),
        (left, top, left, bottom),
        (right, top, right, bottom),
    ] {
        lines.push(LineInstance {
            start_x_pixels: start_x,
            start_y_pixels: start_y,
            end_x_pixels: end_x,
            end_y_pixels: end_y,
            thickness_pixels: CONTROL_BORDER_THICKNESS_PIXELS,
            color_linear_rgba: theme::CONTROL_BORDER,
        });
    }

    // The playhead moves smoothly, so only its thickness is whole pixels.
    let playhead_x_pixels: f32 = geometry.music_left_pixels
        + (geometry.staff_right_pixels - geometry.music_left_pixels) * playhead_fraction;
    lines.push(vertical_line(
        playhead_x_pixels,
        geometry.area_top_y_pixels,
        geometry.area_bottom_y_pixels,
        whole_thickness_pixels(geometry.space_pixels * PLAYHEAD_THICKNESS_SPACES),
        theme::PLAYHEAD,
    ));
    lines
}

/// The control's label, centred in it: what a click will do.
///
/// # Errors
///
/// When the atlas lacks a character of the label.
pub fn staff_control_label(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let label: &str = if geometry.pitch_staff.is_some() {
        "Hide staff"
    } else {
        "Show staff"
    };
    let control: ScreenRect = geometry.staff_control;
    let em_pixels: f32 = CONTROL_LABEL_EM_SPACES * geometry.space_pixels;
    let width_pixels: f32 = atlas.text_width_pixels(GlyphFont::DejaVuSans, label, em_pixels)?;
    let center_x_pixels: f32 = f32::midpoint(control.left_x_pixels, control.right_x_pixels);
    let center_y_pixels: f32 = f32::midpoint(control.top_y_pixels, control.bottom_y_pixels);
    atlas.text(
        GlyphFont::DejaVuSans,
        label,
        center_x_pixels - width_pixels / 2.0,
        center_y_pixels + TEXT_CAP_HEIGHT_EMS * em_pixels / 2.0,
        em_pixels,
        theme::CONTROL_TEXT,
        glyphs,
    )
}

/// A horizontal line from `left_pixels` to `right_pixels`, its centre
/// snapped so its edges fall on pixel boundaries.
#[must_use]
pub fn horizontal_line(
    left_pixels: f32,
    right_pixels: f32,
    y_pixels: f32,
    thickness_pixels: f32,
    color_linear_rgba: [f32; 4],
) -> LineInstance {
    let y_pixels: f32 = snapped_center_pixels(y_pixels, thickness_pixels);
    LineInstance {
        start_x_pixels: left_pixels,
        start_y_pixels: y_pixels,
        end_x_pixels: right_pixels,
        end_y_pixels: y_pixels,
        thickness_pixels,
        color_linear_rgba,
    }
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

/// A thickness of whole pixels, at least 1: with its edges snapped to pixel
/// boundaries such a line fills whole pixels, crisp; a thinner one would
/// only ever be a faint smear.
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

    /// Rhythm line 1 + its barlines 9, pitch staff 5 + its barlines 9,
    /// control border 4, playhead 1.
    const LINES_WITH_STAFF: usize = 1 + 9 + 5 + 9 + 4 + 1;
    const FIRST_PITCH_STAFF_LINE: usize = 1 + 9;

    #[test]
    fn the_part_has_its_lines_and_loses_the_staff_when_hidden() {
        let shown: Vec<LineInstance> = strip_lines(&StripGeometry::new(2560.0, 1440.0, true), 0.5);
        let hidden: Vec<LineInstance> =
            strip_lines(&StripGeometry::new(2560.0, 1440.0, false), 0.5);
        assert_eq!(shown.len(), LINES_WITH_STAFF);
        assert_eq!(hidden.len(), LINES_WITH_STAFF - 5 - 9);
    }

    #[test]
    fn hiding_the_staff_shrinks_the_area_and_keeps_the_control_in_place() {
        let shown: StripGeometry = StripGeometry::new(2560.0, 1440.0, true);
        let hidden: StripGeometry = StripGeometry::new(2560.0, 1440.0, false);
        assert!(hidden.area_bottom_y_pixels < shown.area_bottom_y_pixels);
        assert_eq!(hidden.staff_control, shown.staff_control);
        // The control sits between the rhythm line and the pitch staff.
        let staff: PitchStaffPlace = shown.pitch_staff.unwrap();
        assert!(shown.staff_control.top_y_pixels > shown.rhythm_line_y_pixels);
        assert!(shown.staff_control.bottom_y_pixels < staff.top_y_pixels);
    }

    #[test]
    fn playhead_runs_from_the_first_bar_to_the_last_barline() {
        let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0, true);
        let at_start: Vec<LineInstance> = strip_lines(&geometry, 0.0);
        let at_end: Vec<LineInstance> = strip_lines(&geometry, 1.0);
        let last_barline_x_pixels: f32 = at_start[9].start_x_pixels;
        let playhead: usize = LINES_WITH_STAFF - 1;
        assert!((at_start[playhead].start_x_pixels - geometry.music_left_pixels).abs() < 0.001);
        // Barlines are snapped to whole pixels, so up to half a pixel off.
        assert!((at_end[playhead].start_x_pixels - last_barline_x_pixels).abs() <= 0.5);
    }

    #[test]
    fn staff_lines_are_evenly_spaced_and_whole_pixels_at_every_window_height() {
        for height in 400_u16..=2160 {
            let lines: Vec<LineInstance> =
                strip_lines(&StripGeometry::new(2560.0, f32::from(height), true), 0.0);
            let staff: &[LineInstance] = &lines[FIRST_PITCH_STAFF_LINE..FIRST_PITCH_STAFF_LINE + 5];
            let first_gap_pixels: f32 = staff[1].start_y_pixels - staff[0].start_y_pixels;
            for pair in staff.windows(2) {
                let gap_pixels: f32 = pair[1].start_y_pixels - pair[0].start_y_pixels;
                assert!(
                    (gap_pixels - first_gap_pixels).abs() < 0.001,
                    "height {height}"
                );
            }
            for line in staff.iter().chain(std::iter::once(&lines[0])) {
                let top_edge_pixels: f32 = line.start_y_pixels - line.thickness_pixels / 2.0;
                assert!((top_edge_pixels - top_edge_pixels.round()).abs() < 0.001);
                assert!((line.thickness_pixels - line.thickness_pixels.round()).abs() < 0.001);
                assert!(line.thickness_pixels >= 1.0);
            }
        }
    }

    #[test]
    fn staff_steps_land_on_the_staff_lines() {
        let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0, true);
        let lines: Vec<LineInstance> = strip_lines(&geometry, 0.0);
        let staff: PitchStaffPlace = geometry.pitch_staff.unwrap();
        let bottom_line: &LineInstance = &lines[FIRST_PITCH_STAFF_LINE + 4];
        let top_line: &LineInstance = &lines[FIRST_PITCH_STAFF_LINE];
        assert!((staff.step_y_pixels(0) - bottom_line.start_y_pixels).abs() < 0.001);
        assert!((staff.step_y_pixels(8) - top_line.start_y_pixels).abs() < 0.001);
    }

    #[test]
    fn the_control_label_fits_inside_the_control() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        for shown in [true, false] {
            let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0, shown);
            let mut glyphs: Vec<GlyphInstance> = Vec::new();
            staff_control_label(&geometry, &atlas, &mut glyphs).unwrap();
            // "Hide staff" and "Show staff": 9 visible characters each.
            assert_eq!(glyphs.len(), 9);
            let border_pixels: f32 = atlas.spread_pixels
                * (CONTROL_LABEL_EM_SPACES * geometry.space_pixels)
                / atlas.pixels_per_em;
            for glyph in &glyphs {
                let ink_left_pixels: f32 = glyph.left_x_pixels + border_pixels;
                let ink_right_pixels: f32 =
                    glyph.left_x_pixels + glyph.width_pixels - border_pixels;
                assert!(ink_left_pixels > geometry.staff_control.left_x_pixels);
                assert!(ink_right_pixels < geometry.staff_control.right_x_pixels);
            }
        }
    }

    #[test]
    fn a_click_inside_the_control_hits_it() {
        let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0, true);
        let control: ScreenRect = geometry.staff_control;
        let middle_x_pixels: f32 = f32::midpoint(control.left_x_pixels, control.right_x_pixels);
        let middle_y_pixels: f32 = f32::midpoint(control.top_y_pixels, control.bottom_y_pixels);
        assert!(control.contains(middle_x_pixels, middle_y_pixels));
        assert!(!control.contains(control.right_x_pixels + 1.0, middle_y_pixels));
    }
}
