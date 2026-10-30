//! Spike layout of the key wheel: where it sits in the window. The real
//! layout will take the lead part's chords from the published stream.

use crate::error::RenderError;
use crate::glyph_atlas::{GlyphAtlas, GlyphFont};
use crate::glyph_instance::GlyphInstance;
use crate::theme;
use crate::wheel_instance::{WEDGES_IN_WHEEL, WheelDegree, WheelInstance};

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
/// A chord outside the key, in the centre: it must fit the hole (0.9 of the
/// outer radius across) even as F♯m7♭5.
const CENTER_CHORD_EM_FRACTION_OF_OUTER: f32 = 0.2;
/// Gap between the ring and a chord name, as a fraction of the outer radius.
const CHORD_NAME_GAP_FRACTION_OF_OUTER: f32 = 0.06;

/// DejaVu Sans capital height: 1493 font units ÷ 2048 font units/em. Half
/// of it below a label's centre puts the baseline so capitals are centred.
const TEXT_CAP_HEIGHT_EMS: f32 = 0.729;

/// The angle between neighbouring wedges: 2π ÷ 7, in radians.
const WEDGE_ANGLE_RADIANS: f32 = std::f32::consts::TAU / 7.0;

/// The wheel following the lead part's chords (EFS-1.10): the current
/// chord's wedge brightened and outlined. `None` for a chord outside the
/// key: it has no wedge (`wheel_labels` names it in the centre).
#[must_use]
pub fn wheel_layout(
    width_pixels: f32,
    height_pixels: f32,
    current: Option<WheelDegree>,
    color_by_function: bool,
) -> WheelInstance {
    let outer_radius_pixels: f32 = height_pixels * OUTER_RADIUS_FRACTION_OF_HEIGHT;
    WheelInstance {
        center_x_pixels: width_pixels * CENTER_X_FRACTION_OF_WIDTH,
        center_y_pixels: height_pixels * CENTER_Y_FRACTION_OF_HEIGHT,
        inner_radius_pixels: outer_radius_pixels * INNER_RADIUS_FRACTION_OF_OUTER,
        outer_radius_pixels,
        current,
        color_by_function,
    }
}

/// Appends the wheel's labels: each wedge's numeral at the middle of the
/// ring, its chord name just outside it, and `outside_key_chord` (the
/// current chord, when it is outside the key) by name in the centre.
///
/// # Errors
///
/// When the atlas lacks a character of a label.
pub fn wheel_labels(
    wheel: &WheelInstance,
    atlas: &GlyphAtlas,
    outside_key_chord: Option<&str>,
    out: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let outer_pixels: f32 = wheel.outer_radius_pixels;
    if let Some(chord_symbol) = outside_key_chord {
        centered_label(
            atlas,
            chord_symbol,
            (wheel.center_x_pixels, wheel.center_y_pixels),
            outer_pixels * CENTER_CHORD_EM_FRACTION_OF_OUTER,
            theme::TEXT,
            out,
        )?;
    }
    let numeral_em_pixels: f32 = outer_pixels * NUMERAL_EM_FRACTION_OF_OUTER;
    let name_em_pixels: f32 = outer_pixels * CHORD_NAME_EM_FRACTION_OF_OUTER;
    let middle_radius_pixels: f32 = f32::midpoint(wheel.inner_radius_pixels, outer_pixels);
    for (wedge, (numeral, name)) in SPIKE_NUMERALS.iter().zip(SPIKE_CHORD_NAMES).enumerate() {
        // Wedge 0 is centred straight up; the rest follow clockwise.
        #[allow(clippy::cast_precision_loss)] // wedge is 0 to 6
        let angle_radians: f32 = WEDGE_ANGLE_RADIANS * wedge as f32;
        // Screen y runs down, so "up" is −cos.
        let (across, down): (f32, f32) = (angle_radians.sin(), -angle_radians.cos());
        // Dark on the current wedge's bright fill; light elsewhere.
        let numeral_color: [f32; 4] = if wheel.current.map(WheelDegree::wedge) == Some(wedge) {
            theme::WHEEL_CURRENT_NUMERAL
        } else {
            theme::TEXT
        };
        centered_label(
            atlas,
            numeral,
            (
                wheel.center_x_pixels + across * middle_radius_pixels,
                wheel.center_y_pixels + down * middle_radius_pixels,
            ),
            numeral_em_pixels,
            numeral_color,
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
            (
                wheel.center_x_pixels + across * name_radius_pixels,
                wheel.center_y_pixels + down * name_radius_pixels,
            ),
            name_em_pixels,
            theme::TEXT,
            out,
        )?;
    }
    Ok(())
}

/// Lays out `text` with its capitals centred on the given point.
fn centered_label(
    atlas: &GlyphAtlas,
    text: &str,
    center_pixels: (f32, f32),
    em_pixels: f32,
    color_linear_rgba: [f32; 4],
    out: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let (center_x_pixels, center_y_pixels): (f32, f32) = center_pixels;
    let width_pixels: f32 = atlas.text_width_pixels(GlyphFont::DejaVuSans, text, em_pixels)?;
    atlas.text(
        GlyphFont::DejaVuSans,
        text,
        center_x_pixels - width_pixels / 2.0,
        center_y_pixels + TEXT_CAP_HEIGHT_EMS * em_pixels / 2.0,
        em_pixels,
        color_linear_rgba,
        out,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheel_marks_the_current_chord() {
        let wheel: WheelInstance = wheel_layout(2560.0, 1440.0, Some(WheelDegree::Six), true);
        assert_eq!(wheel.current, Some(WheelDegree::Six));
    }

    #[test]
    fn a_chord_outside_the_key_is_named_inside_the_hole() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        let wheel: WheelInstance = wheel_layout(2560.0, 1440.0, None, true);
        let mut with_center: Vec<GlyphInstance> = Vec::new();
        wheel_labels(&wheel, &atlas, Some("F♯m7♭5"), &mut with_center).unwrap();
        let mut without: Vec<GlyphInstance> = Vec::new();
        wheel_labels(&wheel, &atlas, None, &mut without).unwrap();
        assert_eq!(with_center.len(), without.len() + 6);
        let em_pixels: f32 = wheel.outer_radius_pixels * CENTER_CHORD_EM_FRACTION_OF_OUTER;
        let width_pixels: f32 = atlas
            .text_width_pixels(GlyphFont::DejaVuSans, "F♯m7♭5", em_pixels)
            .unwrap();
        assert!(width_pixels < 2.0 * wheel.inner_radius_pixels);
    }

    #[test]
    fn the_wheel_fits_inside_the_window() {
        for height in 400_u16..=2160 {
            let height_pixels: f32 = f32::from(height);
            let wheel: WheelInstance = wheel_layout(height_pixels * 1.5, height_pixels, None, true);
            assert!(wheel.center_y_pixels + wheel.outer_radius_pixels < height_pixels);
        }
    }

    #[test]
    fn the_labels_fit_inside_the_window() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        for (width, height) in [(2560.0, 1440.0), (1280.0, 720.0), (3840.0, 2160.0)] {
            let wheel: WheelInstance = wheel_layout(width, height, None, true);
            let mut labels: Vec<GlyphInstance> = Vec::new();
            wheel_labels(&wheel, &atlas, None, &mut labels).unwrap();
            // Numerals: 15 characters; names: 11 characters.
            assert_eq!(labels.len(), 15 + 11);
            for glyph in &labels {
                assert!(glyph.top_y_pixels >= 0.0);
                assert!(glyph.top_y_pixels + glyph.height_pixels <= height);
            }
        }
    }

    #[test]
    fn the_current_numeral_is_dark_and_the_others_light() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        // IV is current; its numeral is two characters.
        let wheel: WheelInstance = wheel_layout(2560.0, 1440.0, Some(WheelDegree::Four), true);
        let mut labels: Vec<GlyphInstance> = Vec::new();
        wheel_labels(&wheel, &atlas, None, &mut labels).unwrap();
        let dark: usize = labels
            .iter()
            .filter(|glyph| glyph.color_linear_rgba == theme::WHEEL_CURRENT_NUMERAL)
            .count();
        assert_eq!(dark, 2);
    }
}
