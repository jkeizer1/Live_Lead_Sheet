//! Spike layout of the key wheel: where it sits in the window. The real
//! layout will take the lead part's chords from the published stream.

use crate::error::RenderError;
use crate::glyph_atlas::{GlyphAtlas, GlyphFont};
use crate::glyph_instance::GlyphInstance;
use crate::theme;
use crate::wheel_instance::{WEDGES_IN_WHEEL, WedgeState, WheelInstance};

/// Outer radius as a fraction of window height.
const OUTER_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.18;

/// Inner radius as a fraction of the outer radius; the hole holds a
/// chord outside the key (EFS-1.10).
const INNER_RADIUS_FRACTION_OF_OUTER: f32 = 0.45;

/// Centre, as fractions of window width and height: below the staff.
const CENTER_X_FRACTION_OF_WIDTH: f32 = 0.5;
const CENTER_Y_FRACTION_OF_HEIGHT: f32 = 0.78;

/// Spike labels for C major, in ring order (EFS-1.10, EFS-1.5a): Roman
/// numerals inside the wedges, chord names just outside.
const SPIKE_NUMERALS: [&str; WEDGES_IN_WHEEL] = ["I", "IV", "vii°", "iii", "vi", "ii", "V"];
const SPIKE_CHORD_NAMES: [&str; WEDGES_IN_WHEEL] = ["C", "F", "B°", "Em", "Am", "Dm", "G"];

/// Label sizes as fractions of the outer radius.
const NUMERAL_EM_FRACTION_OF_OUTER: f32 = 0.2;
const CHORD_NAME_EM_FRACTION_OF_OUTER: f32 = 0.16;
/// Gap between the ring and a chord name, as a fraction of the outer radius.
const CHORD_NAME_GAP_FRACTION_OF_OUTER: f32 = 0.06;

/// DejaVu Sans capital height: 1493 font units ÷ 2048 font units/em. Half
/// of it below a label's centre puts the baseline so capitals are centred.
const TEXT_CAP_HEIGHT_EMS: f32 = 0.729;

/// The angle between neighbouring wedges: 2π ÷ 7, in radians.
const WEDGE_ANGLE_RADIANS: f32 = std::f32::consts::TAU / 7.0;

/// The wheel with wedge `current_wedge` filled and the one after it
/// outlined. Wedge numbers wrap around the ring, so the next wedge after
/// the last is the first.
#[must_use]
pub fn wheel_layout(
    width_pixels: f32,
    height_pixels: f32,
    current_wedge: usize,
    color_by_function: bool,
) -> WheelInstance {
    let outer_radius_pixels: f32 = height_pixels * OUTER_RADIUS_FRACTION_OF_HEIGHT;
    let mut wedge_states: [WedgeState; WEDGES_IN_WHEEL] = [WedgeState::Idle; WEDGES_IN_WHEEL];
    wedge_states[current_wedge % WEDGES_IN_WHEEL] = WedgeState::Current;
    wedge_states[(current_wedge + 1) % WEDGES_IN_WHEEL] = WedgeState::Next;
    WheelInstance {
        center_x_pixels: width_pixels * CENTER_X_FRACTION_OF_WIDTH,
        center_y_pixels: height_pixels * CENTER_Y_FRACTION_OF_HEIGHT,
        inner_radius_pixels: outer_radius_pixels * INNER_RADIUS_FRACTION_OF_OUTER,
        outer_radius_pixels,
        wedge_states,
        color_by_function,
    }
}

/// Appends the wheel's labels: each wedge's numeral at the middle of the
/// ring, its chord name just outside it.
///
/// # Errors
///
/// When the atlas lacks a character of a label.
pub fn wheel_labels(
    wheel: &WheelInstance,
    atlas: &GlyphAtlas,
    out: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let outer_pixels: f32 = wheel.outer_radius_pixels;
    let numeral_em_pixels: f32 = outer_pixels * NUMERAL_EM_FRACTION_OF_OUTER;
    let name_em_pixels: f32 = outer_pixels * CHORD_NAME_EM_FRACTION_OF_OUTER;
    let middle_radius_pixels: f32 = f32::midpoint(wheel.inner_radius_pixels, outer_pixels);
    for (wedge, (numeral, name)) in SPIKE_NUMERALS.iter().zip(SPIKE_CHORD_NAMES).enumerate() {
        // Wedge 0 is centred straight up; the rest follow clockwise.
        #[allow(clippy::cast_precision_loss)] // wedge is 0 to 6
        let angle_radians: f32 = WEDGE_ANGLE_RADIANS * wedge as f32;
        // Screen y runs down, so "up" is −cos.
        let (across, down): (f32, f32) = (angle_radians.sin(), -angle_radians.cos());
        centered_label(
            atlas,
            numeral,
            wheel.center_x_pixels + across * middle_radius_pixels,
            wheel.center_y_pixels + down * middle_radius_pixels,
            numeral_em_pixels,
            out,
        )?;
        // Far enough out that the name's box clears the ring in this
        // direction: half its width sideways, half its height up or down.
        let name_width_pixels: f32 =
            atlas.text_width_pixels(GlyphFont::DejaVuSans, name, name_em_pixels)?;
        let name_height_pixels: f32 = TEXT_CAP_HEIGHT_EMS * name_em_pixels;
        let name_radius_pixels: f32 = outer_pixels
            + outer_pixels * CHORD_NAME_GAP_FRACTION_OF_OUTER
            + across.abs() * name_width_pixels / 2.0
            + down.abs() * name_height_pixels / 2.0;
        centered_label(
            atlas,
            name,
            wheel.center_x_pixels + across * name_radius_pixels,
            wheel.center_y_pixels + down * name_radius_pixels,
            name_em_pixels,
            out,
        )?;
    }
    Ok(())
}

/// Lays out `text` with its capitals centred on the given point.
fn centered_label(
    atlas: &GlyphAtlas,
    text: &str,
    center_x_pixels: f32,
    center_y_pixels: f32,
    em_pixels: f32,
    out: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let width_pixels: f32 = atlas.text_width_pixels(GlyphFont::DejaVuSans, text, em_pixels)?;
    atlas.text(
        GlyphFont::DejaVuSans,
        text,
        center_x_pixels - width_pixels / 2.0,
        center_y_pixels + TEXT_CAP_HEIGHT_EMS * em_pixels / 2.0,
        em_pixels,
        theme::TEXT,
        out,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_wedge_is_current_and_the_next_is_outlined() {
        let wheel: WheelInstance = wheel_layout(2560.0, 1440.0, 2, true);
        assert_eq!(wheel.wedge_states[2], WedgeState::Current);
        assert_eq!(wheel.wedge_states[3], WedgeState::Next);
        let idle: usize = wheel
            .wedge_states
            .iter()
            .filter(|state| **state == WedgeState::Idle)
            .count();
        assert_eq!(idle, 5);
    }

    #[test]
    fn the_wedge_after_the_last_is_the_first() {
        let wheel: WheelInstance = wheel_layout(2560.0, 1440.0, 6, true);
        assert_eq!(wheel.wedge_states[6], WedgeState::Current);
        assert_eq!(wheel.wedge_states[0], WedgeState::Next);
    }

    #[test]
    fn the_wheel_fits_inside_the_window() {
        for height in 400_u16..=2160 {
            let height_pixels: f32 = f32::from(height);
            let wheel: WheelInstance = wheel_layout(height_pixels * 1.5, height_pixels, 0, true);
            assert!(wheel.center_y_pixels + wheel.outer_radius_pixels < height_pixels);
        }
    }

    #[test]
    fn the_labels_fit_inside_the_window() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        for (width, height) in [(2560.0, 1440.0), (1280.0, 720.0), (3840.0, 2160.0)] {
            let wheel: WheelInstance = wheel_layout(width, height, 0, true);
            let mut labels: Vec<GlyphInstance> = Vec::new();
            wheel_labels(&wheel, &atlas, &mut labels).unwrap();
            // Numerals: 15 characters; names: 11 characters.
            assert_eq!(labels.len(), 15 + 11);
            for glyph in &labels {
                assert!(glyph.top_y_pixels >= 0.0);
                assert!(glyph.top_y_pixels + glyph.height_pixels <= height);
            }
        }
    }
}
