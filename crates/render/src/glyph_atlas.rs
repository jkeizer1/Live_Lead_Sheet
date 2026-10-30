//! The glyph atlas built offline by `atlas_tool`: one distance-field image
//! and a table saying where each glyph is. Both are compiled into the
//! program, so the render crate reads no files.

use std::collections::HashMap;

use crate::error::RenderError;
use crate::glyph_instance::GlyphInstance;

const TABLE_TEXT: &str = include_str!("../../../assets/atlas/glyphs.txt");
const IMAGE_BYTES: &[u8] = include_bytes!("../../../assets/atlas/glyphs.pgm");

/// A font in the atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GlyphFont {
    /// Music symbols (SMuFL). One em is 4 staff spaces.
    Bravura,
    /// Chord symbols, numerals and labels.
    DejaVuSans,
}

impl GlyphFont {
    fn from_table_name(name: &str) -> Option<GlyphFont> {
        match name {
            "bravura" => Some(GlyphFont::Bravura),
            "dejavu_sans" => Some(GlyphFont::DejaVuSans),
            _ => None,
        }
    }

    fn table_name(self) -> &'static str {
        match self {
            GlyphFont::Bravura => "bravura",
            GlyphFont::DejaVuSans => "dejavu_sans",
        }
    }
}

/// One glyph's entry in the table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasGlyph {
    /// The field in the image, in atlas pixels; all zero for a glyph with
    /// nothing to draw (the space).
    pub x_pixels: u32,
    pub y_pixels: u32,
    pub width_pixels: u32,
    pub height_pixels: u32,
    /// The field's top-left corner from the glyph's origin, x right, y up.
    pub left_ems: f32,
    pub top_ems: f32,
    /// How far the pen moves on after this glyph.
    pub advance_ems: f32,
}

pub struct GlyphAtlas {
    /// Atlas pixels to one em: the size the fields were computed at.
    pub pixels_per_em: f32,
    /// How far the fields measure from an outline, in atlas pixels.
    pub spread_pixels: f32,
    pub width_pixels: u32,
    pub height_pixels: u32,
    /// One byte per pixel, row by row from the top.
    pub image: Vec<u8>,
    glyphs: HashMap<(GlyphFont, char), AtlasGlyph>,
}

impl GlyphAtlas {
    /// The atlas compiled into the program.
    ///
    /// # Errors
    ///
    /// When the table or image is malformed: rebuild them with
    /// `cargo run --release -p atlas_tool`.
    pub fn embedded() -> Result<GlyphAtlas, RenderError> {
        GlyphAtlas::parse(TABLE_TEXT, IMAGE_BYTES)
    }

    /// Reads a table and its PGM image.
    ///
    /// # Errors
    ///
    /// When a line of the table cannot be read, a glyph lies outside the
    /// image, or the image does not match the table's size.
    pub fn parse(table_text: &str, image_bytes: &[u8]) -> Result<GlyphAtlas, RenderError> {
        let mut pixels_per_em: Option<f32> = None;
        let mut spread_pixels: Option<f32> = None;
        let mut width_pixels: Option<u32> = None;
        let mut height_pixels: Option<u32> = None;
        let mut glyphs: HashMap<(GlyphFont, char), AtlasGlyph> = HashMap::new();
        for (index, line) in table_text.lines().enumerate() {
            let line_number: usize = index + 1;
            let bad = |reason: &str| RenderError::AtlasUnreadable {
                line_number,
                reason: format!("{reason}: {line:?}"),
            };
            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields.as_slice() {
                [] => {}
                [first, ..] if first.starts_with('#') => {}
                ["pixels_per_em", value] => {
                    pixels_per_em = Some(value.parse().map_err(|_| bad("not a number"))?);
                }
                ["spread_pixels", value] => {
                    spread_pixels = Some(value.parse().map_err(|_| bad("not a number"))?);
                }
                ["atlas_width_pixels", value] => {
                    width_pixels = Some(value.parse().map_err(|_| bad("not a whole number"))?);
                }
                ["atlas_height_pixels", value] => {
                    height_pixels = Some(value.parse().map_err(|_| bad("not a whole number"))?);
                }
                [
                    "glyph",
                    font,
                    codepoint,
                    _name,
                    x,
                    y,
                    width,
                    height,
                    left,
                    top,
                    advance,
                ] => {
                    let font: GlyphFont =
                        GlyphFont::from_table_name(font).ok_or_else(|| bad("unknown font"))?;
                    let codepoint: char = u32::from_str_radix(codepoint, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or_else(|| bad("not a hexadecimal character code"))?;
                    let whole = |text: &str| -> Result<u32, RenderError> {
                        text.parse().map_err(|_| bad("not a whole number"))
                    };
                    let number = |text: &str| -> Result<f32, RenderError> {
                        text.parse().map_err(|_| bad("not a number"))
                    };
                    let glyph: AtlasGlyph = AtlasGlyph {
                        x_pixels: whole(x)?,
                        y_pixels: whole(y)?,
                        width_pixels: whole(width)?,
                        height_pixels: whole(height)?,
                        left_ems: number(left)?,
                        top_ems: number(top)?,
                        advance_ems: number(advance)?,
                    };
                    if glyphs.insert((font, codepoint), glyph).is_some() {
                        return Err(bad("glyph listed twice"));
                    }
                }
                _ => return Err(bad("not a line the atlas table has")),
            }
        }
        let missing = |what: &str| RenderError::AtlasUnreadable {
            line_number: 0,
            reason: format!("the table has no {what} line"),
        };
        let pixels_per_em: f32 = pixels_per_em.ok_or_else(|| missing("pixels_per_em"))?;
        let spread_pixels: f32 = spread_pixels.ok_or_else(|| missing("spread_pixels"))?;
        let width_pixels: u32 = width_pixels.ok_or_else(|| missing("atlas_width_pixels"))?;
        let height_pixels: u32 = height_pixels.ok_or_else(|| missing("atlas_height_pixels"))?;
        for ((font, codepoint), glyph) in &glyphs {
            if glyph.x_pixels + glyph.width_pixels > width_pixels
                || glyph.y_pixels + glyph.height_pixels > height_pixels
            {
                return Err(RenderError::AtlasUnreadable {
                    line_number: 0,
                    reason: format!(
                        "glyph {} U+{:04X} lies outside the {width_pixels} × {height_pixels} \
                         pixel image",
                        font.table_name(),
                        u32::from(*codepoint)
                    ),
                });
            }
        }
        let image: Vec<u8> = pgm_pixels(image_bytes, width_pixels, height_pixels)?;
        Ok(GlyphAtlas {
            pixels_per_em,
            spread_pixels,
            width_pixels,
            height_pixels,
            image,
            glyphs,
        })
    }

    /// A glyph's table entry.
    ///
    /// # Errors
    ///
    /// When the atlas has no such glyph: add it to `atlas_tool`'s glyph list
    /// and rebuild the atlas.
    pub fn glyph(&self, font: GlyphFont, codepoint: char) -> Result<&AtlasGlyph, RenderError> {
        self.glyphs
            .get(&(font, codepoint))
            .ok_or(RenderError::GlyphMissing {
                font: font.table_name(),
                codepoint,
            })
    }

    /// The record that draws one glyph with its origin at the given point,
    /// at `em_pixels` screen pixels to the em. `None` for a glyph with
    /// nothing to draw (the space).
    ///
    /// # Errors
    ///
    /// When the atlas has no such glyph.
    pub fn place(
        &self,
        font: GlyphFont,
        codepoint: char,
        origin_x_pixels: f32,
        origin_y_pixels: f32,
        em_pixels: f32,
        color_linear_rgba: [f32; 4],
    ) -> Result<Option<GlyphInstance>, RenderError> {
        let glyph: &AtlasGlyph = self.glyph(font, codepoint)?;
        if glyph.width_pixels == 0 || glyph.height_pixels == 0 {
            return Ok(None);
        }
        // screen px/em ÷ atlas px/em = screen px per atlas px
        let scale: f32 = em_pixels / self.pixels_per_em;
        // Atlas sizes are far below 2^24, so these conversions are exact.
        #[allow(clippy::cast_precision_loss)]
        let (x, y, width, height, atlas_width, atlas_height): (
            f32,
            f32,
            f32,
            f32,
            f32,
            f32,
        ) = (
            glyph.x_pixels as f32,
            glyph.y_pixels as f32,
            glyph.width_pixels as f32,
            glyph.height_pixels as f32,
            self.width_pixels as f32,
            self.height_pixels as f32,
        );
        Ok(Some(GlyphInstance {
            left_x_pixels: origin_x_pixels + glyph.left_ems * em_pixels,
            // The table's y runs up; the screen's runs down.
            top_y_pixels: origin_y_pixels - glyph.top_ems * em_pixels,
            width_pixels: width * scale,
            height_pixels: height * scale,
            atlas_left: x / atlas_width,
            atlas_top: y / atlas_height,
            atlas_right: (x + width) / atlas_width,
            atlas_bottom: (y + height) / atlas_height,
            color_linear_rgba,
            // The field spans ±spread atlas px over values 0 to 1, so one
            // unit of value is 2 × spread atlas px = 2 × spread × scale
            // screen px.
            distance_scale_pixels: 2.0 * self.spread_pixels * scale,
        }))
    }

    /// Lays out `text` from `origin_x_pixels` along the baseline, glyph after
    /// glyph by their advances, appending the records to `out`.
    ///
    /// # Errors
    ///
    /// When the atlas lacks one of the characters.
    #[allow(clippy::too_many_arguments)] // each is a separate, named quantity
    pub fn text(
        &self,
        font: GlyphFont,
        text: &str,
        origin_x_pixels: f32,
        baseline_y_pixels: f32,
        em_pixels: f32,
        color_linear_rgba: [f32; 4],
        out: &mut Vec<GlyphInstance>,
    ) -> Result<(), RenderError> {
        let mut pen_x_pixels: f32 = origin_x_pixels;
        for character in text.chars() {
            if let Some(instance) = self.place(
                font,
                character,
                pen_x_pixels,
                baseline_y_pixels,
                em_pixels,
                color_linear_rgba,
            )? {
                out.push(instance);
            }
            pen_x_pixels += self.glyph(font, character)?.advance_ems * em_pixels;
        }
        Ok(())
    }

    /// How wide `text` is, by its advances, at `em_pixels` to the em.
    ///
    /// # Errors
    ///
    /// When the atlas lacks one of the characters.
    pub fn text_width_pixels(
        &self,
        font: GlyphFont,
        text: &str,
        em_pixels: f32,
    ) -> Result<f32, RenderError> {
        let mut width_ems: f32 = 0.0;
        for character in text.chars() {
            width_ems += self.glyph(font, character)?.advance_ems;
        }
        Ok(width_ems * em_pixels)
    }
}

/// The pixel bytes of a binary PGM image ("P5", one byte per pixel), checked
/// against the size the table states.
fn pgm_pixels(bytes: &[u8], width_pixels: u32, height_pixels: u32) -> Result<Vec<u8>, RenderError> {
    let bad = |reason: String| RenderError::AtlasUnreadable {
        line_number: 0,
        reason: format!("glyphs.pgm: {reason}"),
    };
    let header: String = format!("P5\n{width_pixels} {height_pixels}\n255\n");
    let Some(pixels) = bytes.strip_prefix(header.as_bytes()) else {
        return Err(bad(format!(
            "does not start with the header for a {width_pixels} × {height_pixels} image"
        )));
    };
    let expected_pixels: u64 = u64::from(width_pixels) * u64::from(height_pixels);
    if pixels.len() as u64 != expected_pixels {
        return Err(bad(format!(
            "{} pixels after the header; {expected_pixels} expected",
            pixels.len()
        )));
    }
    Ok(pixels.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "# test\n\
        pixels_per_em 100\n\
        spread_pixels 8\n\
        atlas_width_pixels 4\n\
        atlas_height_pixels 2\n\
        glyph dejavu_sans 0043 - 1 0 2 2 -0.1 0.8 0.7\n\
        glyph dejavu_sans 0020 - 0 0 0 0 0 0 0.3\n";

    fn image() -> Vec<u8> {
        let mut bytes: Vec<u8> = b"P5\n4 2\n255\n".to_vec();
        bytes.extend_from_slice(&[0, 10, 20, 0, 0, 30, 40, 0]);
        bytes
    }

    #[test]
    fn the_embedded_atlas_reads_and_has_the_glyphs_the_layout_uses() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        assert!(atlas.glyph(GlyphFont::Bravura, '\u{E050}').is_ok(), "gClef");
        assert!(
            atlas.glyph(GlyphFont::Bravura, '\u{E0A4}').is_ok(),
            "noteheadBlack"
        );
        assert!(atlas.glyph(GlyphFont::DejaVuSans, '♭').is_ok());
        assert!(atlas.glyph(GlyphFont::DejaVuSans, '°').is_ok());
    }

    #[test]
    fn a_glyph_is_placed_from_its_origin_at_the_requested_size() {
        let atlas: GlyphAtlas = GlyphAtlas::parse(TABLE, &image()).unwrap();
        // 50 px/em on screen: half the atlas's 100 px/em.
        let glyph: GlyphInstance = atlas
            .place(GlyphFont::DejaVuSans, 'C', 200.0, 300.0, 50.0, [1.0; 4])
            .unwrap()
            .unwrap();
        // left: 200 px + (−0.1 em × 50 px/em) = 195 px
        assert!((glyph.left_x_pixels - 195.0).abs() < 1e-4);
        // top: 300 px − (0.8 em × 50 px/em) = 260 px
        assert!((glyph.top_y_pixels - 260.0).abs() < 1e-4);
        // 2 atlas px × 0.5 = 1 screen px
        assert!((glyph.width_pixels - 1.0).abs() < 1e-4);
        assert!((glyph.atlas_left - 0.25).abs() < 1e-6);
        assert!((glyph.atlas_right - 0.75).abs() < 1e-6);
        // 2 × 8 atlas px × 0.5 = 8 screen px per unit of value
        assert!((glyph.distance_scale_pixels - 8.0).abs() < 1e-4);
    }

    #[test]
    fn a_space_draws_nothing_but_advances_the_pen() {
        let atlas: GlyphAtlas = GlyphAtlas::parse(TABLE, &image()).unwrap();
        let mut out: Vec<GlyphInstance> = Vec::new();
        atlas
            .text(
                GlyphFont::DejaVuSans,
                "C C",
                0.0,
                0.0,
                100.0,
                [1.0; 4],
                &mut out,
            )
            .unwrap();
        assert_eq!(out.len(), 2);
        // second C: 0.7 em + 0.3 em = 1 em = 100 px further on
        assert!((out[1].left_x_pixels - out[0].left_x_pixels - 100.0).abs() < 1e-3);
        let width: f32 = atlas
            .text_width_pixels(GlyphFont::DejaVuSans, "C C", 100.0)
            .unwrap();
        assert!((width - 170.0).abs() < 1e-3);
    }

    #[test]
    fn a_missing_glyph_is_reported() {
        let atlas: GlyphAtlas = GlyphAtlas::parse(TABLE, &image()).unwrap();
        assert!(atlas.glyph(GlyphFont::DejaVuSans, 'Q').is_err());
    }

    #[test]
    fn an_image_of_the_wrong_size_is_refused() {
        let mut bytes: Vec<u8> = image();
        bytes.pop();
        assert!(GlyphAtlas::parse(TABLE, &bytes).is_err());
    }
}
