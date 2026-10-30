//! A signed distance field: for each pixel, how far its centre is from the
//! glyph's outline, positive inside. The glyph shader draws the edge where
//! the field crosses the middle value, so the glyph stays crisp when scaled.

use crate::outline::{FlatOutline, Segment};

/// The value stored for a pixel exactly on the outline: the middle of 0 to
/// 255. Values above are inside the glyph.
pub const EDGE_VALUE: u8 = 128;

/// One glyph's distance field, row by row from the top.
#[derive(Clone, Debug, PartialEq)]
pub struct DistanceField {
    pub width_pixels: u32,
    pub height_pixels: u32,
    /// One byte per pixel: 0 is `spread_pixels` or more outside, 255 is
    /// `spread_pixels` or more inside, `EDGE_VALUE` on the outline.
    pub values: Vec<u8>,
    /// The field's top-left corner relative to the glyph's origin (on the
    /// baseline, at the pen position), in pixels, x right, y up.
    pub left_from_origin_pixels: f64,
    pub top_from_origin_pixels: f64,
}

/// Size and range of the fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldSettings {
    /// Pixels to one em of the font.
    pub pixels_per_em: f64,
    /// The font's own units to one em (1000 for Bravura, 2048 for DejaVu).
    pub font_units_per_em: f64,
    /// How far from the outline the field measures, in pixels; also the
    /// empty border kept around the glyph.
    pub spread_pixels: f64,
}

/// Computes `outline`'s distance field.
///
/// # Errors
///
/// When the glyph is too large for the field's size in pixels.
pub fn distance_field(
    outline: &FlatOutline,
    settings: FieldSettings,
) -> Result<DistanceField, String> {
    // px/em ÷ font units/em = px per font unit
    let pixels_per_font_unit: f64 = settings.pixels_per_em / settings.font_units_per_em;
    let spread_font_units: f64 = settings.spread_pixels / pixels_per_font_unit;
    let left_font_units: f64 = outline.bounds.left_font_units - spread_font_units;
    let top_font_units: f64 = outline.bounds.top_font_units + spread_font_units;
    let ink_width_pixels: f64 =
        (outline.bounds.right_font_units - outline.bounds.left_font_units) * pixels_per_font_unit;
    let ink_height_pixels: f64 =
        (outline.bounds.top_font_units - outline.bounds.bottom_font_units) * pixels_per_font_unit;
    let width_pixels: u32 = whole_pixels(ink_width_pixels + 2.0 * settings.spread_pixels)?;
    let height_pixels: u32 = whole_pixels(ink_height_pixels + 2.0 * settings.spread_pixels)?;

    let mut values: Vec<u8> = Vec::with_capacity(
        usize::try_from(u64::from(width_pixels) * u64::from(height_pixels))
            .map_err(|e| e.to_string())?,
    );
    for row in 0..height_pixels {
        // Pixel centres: half a pixel in from the cell's edges. Image rows
        // run down; font y runs up.
        let y_font_units: f64 = top_font_units - (f64::from(row) + 0.5) / pixels_per_font_unit;
        for column in 0..width_pixels {
            let x_font_units: f64 =
                left_font_units + (f64::from(column) + 0.5) / pixels_per_font_unit;
            let distance_pixels: f64 =
                nearest_distance_font_units(&outline.segments, x_font_units, y_font_units)
                    * pixels_per_font_unit;
            let signed_distance_pixels: f64 =
                if is_inside(&outline.segments, x_font_units, y_font_units) {
                    distance_pixels
                } else {
                    -distance_pixels
                };
            values.push(encode(signed_distance_pixels, settings.spread_pixels));
        }
    }
    Ok(DistanceField {
        width_pixels,
        height_pixels,
        values,
        left_from_origin_pixels: left_font_units * pixels_per_font_unit,
        top_from_origin_pixels: top_font_units * pixels_per_font_unit,
    })
}

/// Maps −spread … +spread px onto 0 … 255, with the outline at about 128.
fn encode(signed_distance_pixels: f64, spread_pixels: f64) -> u8 {
    let fraction: f64 = (0.5 + signed_distance_pixels / (2.0 * spread_pixels)).clamp(0.0, 1.0);
    // 0.0 ≤ fraction × 255 ≤ 255, so the rounded value fits a u8.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value: u8 = (fraction * 255.0).round() as u8;
    value
}

/// Rounds a size up to whole pixels.
fn whole_pixels(size_pixels: f64) -> Result<u32, String> {
    let rounded_pixels: f64 = size_pixels.ceil();
    if !(0.0..=f64::from(u16::MAX)).contains(&rounded_pixels) {
        return Err(format!(
            "A glyph field of {size_pixels} pixels is outside 0 to {} pixels.",
            u16::MAX
        ));
    }
    // Checked just above: a whole number from 0 to 65535.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole: u32 = rounded_pixels as u32;
    Ok(whole)
}

/// The distance from a point to the nearest segment, in font units.
fn nearest_distance_font_units(segments: &[Segment], x_font_units: f64, y_font_units: f64) -> f64 {
    let mut nearest_squared: f64 = f64::INFINITY;
    for segment in segments {
        let along_x: f64 = segment.end_x_font_units - segment.start_x_font_units;
        let along_y: f64 = segment.end_y_font_units - segment.start_y_font_units;
        let to_point_x: f64 = x_font_units - segment.start_x_font_units;
        let to_point_y: f64 = y_font_units - segment.start_y_font_units;
        let length_squared: f64 = along_x * along_x + along_y * along_y;
        // Where along the segment (0 start, 1 end) the closest point is. A
        // zero-length segment's closest point is its start.
        let fraction: f64 = if length_squared > 0.0 {
            ((to_point_x * along_x + to_point_y * along_y) / length_squared).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let gap_x: f64 = to_point_x - fraction * along_x;
        let gap_y: f64 = to_point_y - fraction * along_y;
        nearest_squared = nearest_squared.min(gap_x * gap_x + gap_y * gap_y);
    }
    nearest_squared.sqrt()
}

/// Whether a point is inside the outline, by the non-zero winding rule
/// that TrueType and CFF fonts both use: count how the outline winds around
/// the point; anything but zero is inside.
fn is_inside(segments: &[Segment], x_font_units: f64, y_font_units: f64) -> bool {
    let mut winding: i32 = 0;
    for segment in segments {
        let start_y: f64 = segment.start_y_font_units;
        let end_y: f64 = segment.end_y_font_units;
        // Which side of the segment's line the point is on: positive left.
        let side: f64 = (segment.end_x_font_units - segment.start_x_font_units)
            * (y_font_units - start_y)
            - (x_font_units - segment.start_x_font_units) * (end_y - start_y);
        if start_y <= y_font_units && end_y > y_font_units && side > 0.0 {
            winding += 1;
        } else if start_y > y_font_units && end_y <= y_font_units && side < 0.0 {
            winding -= 1;
        }
    }
    winding != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline::GlyphBounds;

    /// A 100 × 100 font-unit square, anticlockwise.
    fn square() -> FlatOutline {
        let corners: [(f64, f64); 4] = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
        let mut segments: Vec<Segment> = Vec::new();
        for index in 0..4 {
            let (start_x, start_y): (f64, f64) = corners[index];
            let (end_x, end_y): (f64, f64) = corners[(index + 1) % 4];
            segments.push(Segment {
                start_x_font_units: start_x,
                start_y_font_units: start_y,
                end_x_font_units: end_x,
                end_y_font_units: end_y,
            });
        }
        FlatOutline {
            segments,
            bounds: GlyphBounds {
                left_font_units: 0.0,
                bottom_font_units: 0.0,
                right_font_units: 100.0,
                top_font_units: 100.0,
            },
            advance_font_units: 100.0,
        }
    }

    #[test]
    fn centre_is_inside_and_far_corner_is_outside() {
        assert!(is_inside(&square().segments, 50.0, 50.0));
        assert!(!is_inside(&square().segments, 150.0, 50.0));
    }

    #[test]
    fn field_is_full_inside_empty_outside_and_mid_at_the_edge() {
        // 100 font units/em at 100 px/em: 1 px per font unit, 4 px spread.
        let settings: FieldSettings = FieldSettings {
            pixels_per_em: 100.0,
            font_units_per_em: 100.0,
            spread_pixels: 4.0,
        };
        let field: DistanceField = distance_field(&square(), settings).unwrap();
        // 100 px of ink plus 4 px of border on each side.
        assert_eq!(field.width_pixels, 108);
        assert_eq!(field.height_pixels, 108);
        let value_at = |column: u32, row: u32| -> u8 {
            field.values[usize::try_from(row * field.width_pixels + column).unwrap()]
        };
        assert_eq!(value_at(54, 54), 255, "centre, far inside");
        // 3.5 px outside, 4 px spread: (0.5 − 3.5 px / 8 px) × 255 = 15.9.
        assert_eq!(value_at(0, 54), 16, "border, 3.5 px outside");
        // The pixel centred 0.5 px outside the edge, then 0.5 px inside.
        let outside: u8 = value_at(3, 54);
        let inside: u8 = value_at(4, 54);
        assert!(
            outside < EDGE_VALUE && inside >= EDGE_VALUE,
            "{outside} {inside}"
        );
    }
}
