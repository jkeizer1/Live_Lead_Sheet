//! A glyph's outline as straight segments, in font units (the font's own
//! grid, `units_per_em` to the em, y up).

use ttf_parser::{Face, GlyphId, OutlineBuilder, Rect};

/// Straight pieces per curve. At 96 px/em a long Bravura curve spans about
/// 500 font units = 48 px; 16 pieces of 3 px each stray from the true
/// curve by well under 0.1 px, which the distance field cannot show.
const PIECES_PER_CURVE: u16 = 16;

/// One straight piece of an outline, in font units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub start_x_font_units: f64,
    pub start_y_font_units: f64,
    pub end_x_font_units: f64,
    pub end_y_font_units: f64,
}

/// The smallest rectangle around a glyph's ink, in font units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphBounds {
    pub left_font_units: f64,
    pub bottom_font_units: f64,
    pub right_font_units: f64,
    pub top_font_units: f64,
}

/// A glyph's closed outlines, flattened.
#[derive(Clone, Debug, PartialEq)]
pub struct FlatOutline {
    pub segments: Vec<Segment>,
    pub bounds: GlyphBounds,
    /// How far the pen moves on after this glyph, in font units.
    pub advance_font_units: f64,
}

/// Reads `character`'s outline from `face` and flattens it. `font_name` is
/// for the error message.
///
/// # Errors
///
/// When the font has no glyph for `character`, or the glyph has no outline
/// (a space, for example: it is placed by its advance alone).
pub fn flatten_glyph(
    face: &Face<'_>,
    character: char,
    font_name: &str,
) -> Result<FlatOutline, String> {
    let glyph_id: GlyphId = face.glyph_index(character).ok_or_else(|| {
        format!(
            "{font_name} has no glyph for {character:?} (U+{:04X}).",
            u32::from(character)
        )
    })?;
    let mut collector: SegmentCollector = SegmentCollector::default();
    let rect: Rect = face
        .outline_glyph(glyph_id, &mut collector)
        .ok_or_else(|| {
            format!(
                "{font_name}'s glyph for {character:?} (U+{:04X}) has no outline.",
                u32::from(character)
            )
        })?;
    let advance_font_units: f64 = glyph_advance_font_units(face, character, font_name)?;
    Ok(FlatOutline {
        segments: collector.segments,
        bounds: GlyphBounds {
            left_font_units: f64::from(rect.x_min),
            bottom_font_units: f64::from(rect.y_min),
            right_font_units: f64::from(rect.x_max),
            top_font_units: f64::from(rect.y_max),
        },
        advance_font_units,
    })
}

/// How far the pen moves on after `character`, in font units. Used alone
/// for characters with no outline, such as the space.
///
/// # Errors
///
/// When the font has no glyph for `character`, or the glyph has no advance.
pub fn glyph_advance_font_units(
    face: &Face<'_>,
    character: char,
    font_name: &str,
) -> Result<f64, String> {
    let glyph_id: GlyphId = face.glyph_index(character).ok_or_else(|| {
        format!(
            "{font_name} has no glyph for {character:?} (U+{:04X}).",
            u32::from(character)
        )
    })?;
    // A glyph without an advance would be placed on top of the next one.
    let advance_font_units: u16 = face.glyph_hor_advance(glyph_id).ok_or_else(|| {
        format!(
            "{font_name}'s glyph for {character:?} (U+{:04X}) has no horizontal advance.",
            u32::from(character)
        )
    })?;
    Ok(f64::from(advance_font_units))
}

/// Receives the outline from `ttf-parser`, one drawing command at a time,
/// and keeps it as straight segments.
#[derive(Default)]
struct SegmentCollector {
    segments: Vec<Segment>,
    /// Where the pen is.
    pen_x_font_units: f64,
    pen_y_font_units: f64,
    /// Where the current contour started, to close it.
    contour_start_x_font_units: f64,
    contour_start_y_font_units: f64,
}

impl SegmentCollector {
    fn line_to_point(&mut self, x_font_units: f64, y_font_units: f64) {
        self.segments.push(Segment {
            start_x_font_units: self.pen_x_font_units,
            start_y_font_units: self.pen_y_font_units,
            end_x_font_units: x_font_units,
            end_y_font_units: y_font_units,
        });
        self.pen_x_font_units = x_font_units;
        self.pen_y_font_units = y_font_units;
    }
}

impl OutlineBuilder for SegmentCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.pen_x_font_units = f64::from(x);
        self.pen_y_font_units = f64::from(y);
        self.contour_start_x_font_units = f64::from(x);
        self.contour_start_y_font_units = f64::from(y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.line_to_point(f64::from(x), f64::from(y));
    }

    /// A quadratic Bézier curve (TrueType fonts), from the pen through
    /// control point 1 to the end point.
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let start_x: f64 = self.pen_x_font_units;
        let start_y: f64 = self.pen_y_font_units;
        for piece in 1..=PIECES_PER_CURVE {
            let t: f64 = f64::from(piece) / f64::from(PIECES_PER_CURVE);
            let u: f64 = 1.0 - t;
            // B(t) = u² P0 + 2 u t P1 + t² P2
            let point_x: f64 = u * u * start_x + 2.0 * u * t * f64::from(x1) + t * t * f64::from(x);
            let point_y: f64 = u * u * start_y + 2.0 * u * t * f64::from(y1) + t * t * f64::from(y);
            self.line_to_point(point_x, point_y);
        }
    }

    /// A cubic Bézier curve (CFF fonts such as Bravura), from the pen
    /// through control points 1 and 2 to the end point.
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let start_x: f64 = self.pen_x_font_units;
        let start_y: f64 = self.pen_y_font_units;
        for piece in 1..=PIECES_PER_CURVE {
            let t: f64 = f64::from(piece) / f64::from(PIECES_PER_CURVE);
            let u: f64 = 1.0 - t;
            // B(t) = u³ P0 + 3 u² t P1 + 3 u t² P2 + t³ P3
            let point_x: f64 = u * u * u * start_x
                + 3.0 * u * u * t * f64::from(x1)
                + 3.0 * u * t * t * f64::from(x2)
                + t * t * t * f64::from(x);
            let point_y: f64 = u * u * u * start_y
                + 3.0 * u * u * t * f64::from(y1)
                + 3.0 * u * t * t * f64::from(y2)
                + t * t * t * f64::from(y);
            self.line_to_point(point_x, point_y);
        }
    }

    fn close(&mut self) {
        // Fonts may end a contour away from its start; the closing segment
        // makes it a closed shape either way. A zero-length closing segment
        // is harmless: it has no effect on distances or winding.
        self.line_to_point(
            self.contour_start_x_font_units,
            self.contour_start_y_font_units,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_square_is_four_segments_back_to_its_start() {
        let mut collector: SegmentCollector = SegmentCollector::default();
        collector.move_to(0.0, 0.0);
        collector.line_to(10.0, 0.0);
        collector.line_to(10.0, 10.0);
        collector.line_to(0.0, 10.0);
        collector.close();
        assert_eq!(collector.segments.len(), 4);
        let last: Segment = collector.segments[3];
        assert!(last.end_x_font_units.abs() < 1e-9 && last.end_y_font_units.abs() < 1e-9);
    }

    #[test]
    fn a_curve_ends_exactly_at_its_end_point() {
        let mut collector: SegmentCollector = SegmentCollector::default();
        collector.move_to(0.0, 0.0);
        collector.curve_to(0.0, 50.0, 100.0, 50.0, 100.0, 0.0);
        assert_eq!(collector.segments.len(), usize::from(PIECES_PER_CURVE));
        let last: Segment = collector.segments[collector.segments.len() - 1];
        assert!((last.end_x_font_units - 100.0).abs() < 1e-9);
        assert!(last.end_y_font_units.abs() < 1e-9);
    }
}
