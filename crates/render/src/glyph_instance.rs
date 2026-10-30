//! One record for the glyph shader: a rectangle on screen, the part of the
//! atlas to draw in it, and a colour. Made by `GlyphAtlas::place`.

/// A glyph, in window pixels measured from the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphInstance {
    pub left_x_pixels: f32,
    pub top_y_pixels: f32,
    pub width_pixels: f32,
    pub height_pixels: f32,
    /// The field's rectangle in the atlas, as fractions of its width and
    /// height (0 to 1).
    pub atlas_left: f32,
    pub atlas_top: f32,
    pub atlas_right: f32,
    pub atlas_bottom: f32,
    /// Linear-light red, green, blue, alpha (see `theme`).
    pub color_linear_rgba: [f32; 4],
    /// Screen pixels per unit of field value: turns a sampled value back
    /// into a distance from the outline on screen.
    pub distance_scale_pixels: f32,
}

/// Floats in one record as the shader reads it: rectangle (4), atlas
/// rectangle (4), colour (4), distance scale (1).
const FLOATS_PER_INSTANCE: usize = 13;

/// Bytes in one record as the shader reads it (4 bytes per float).
pub const BYTES_PER_INSTANCE: usize = FLOATS_PER_INSTANCE * 4;

impl GlyphInstance {
    /// Appends this record's bytes, in the order the shader's vertex layout
    /// expects. Native byte order: the GPU is on the same machine.
    pub fn write_bytes(&self, bytes: &mut Vec<u8>) {
        let floats: [f32; FLOATS_PER_INSTANCE] = [
            self.left_x_pixels,
            self.top_y_pixels,
            self.width_pixels,
            self.height_pixels,
            self.atlas_left,
            self.atlas_top,
            self.atlas_right,
            self.atlas_bottom,
            self.color_linear_rgba[0],
            self.color_linear_rgba[1],
            self.color_linear_rgba[2],
            self.color_linear_rgba[3],
            self.distance_scale_pixels,
        ];
        for float in floats {
            bytes.extend_from_slice(&float.to_ne_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_is_the_declared_number_of_bytes() {
        let glyph: GlyphInstance = GlyphInstance {
            left_x_pixels: 1.0,
            top_y_pixels: 2.0,
            width_pixels: 3.0,
            height_pixels: 4.0,
            atlas_left: 0.1,
            atlas_top: 0.2,
            atlas_right: 0.3,
            atlas_bottom: 0.4,
            color_linear_rgba: [0.5, 0.5, 0.5, 1.0],
            distance_scale_pixels: 8.0,
        };
        let mut bytes: Vec<u8> = Vec::new();
        glyph.write_bytes(&mut bytes);
        assert_eq!(bytes.len(), BYTES_PER_INSTANCE);
    }
}
