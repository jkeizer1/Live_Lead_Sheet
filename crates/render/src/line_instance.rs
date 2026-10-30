//! One record for the line shader: a straight line with a thickness and a
//! colour. Staff lines, barlines, stems and the playhead are all this.

/// A line, in window pixels measured from the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineInstance {
    pub start_x_pixels: f32,
    pub start_y_pixels: f32,
    pub end_x_pixels: f32,
    pub end_y_pixels: f32,
    pub thickness_pixels: f32,
    /// Linear-light red, green, blue, alpha (see `theme`).
    pub color_linear_rgba: [f32; 4],
}

/// Floats in one record as the shader reads it: start and end (4), colour
/// (4), thickness (1).
const FLOATS_PER_INSTANCE: usize = 9;

/// Bytes in one record as the shader reads it (4 bytes per float).
pub const BYTES_PER_INSTANCE: usize = FLOATS_PER_INSTANCE * 4;

impl LineInstance {
    /// Appends this record's bytes, in the order the shader's vertex layout
    /// expects. Native byte order: the GPU is on the same machine.
    pub fn write_bytes(&self, bytes: &mut Vec<u8>) {
        let floats: [f32; FLOATS_PER_INSTANCE] = [
            self.start_x_pixels,
            self.start_y_pixels,
            self.end_x_pixels,
            self.end_y_pixels,
            self.color_linear_rgba[0],
            self.color_linear_rgba[1],
            self.color_linear_rgba[2],
            self.color_linear_rgba[3],
            self.thickness_pixels,
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
        let line: LineInstance = LineInstance {
            start_x_pixels: 1.0,
            start_y_pixels: 2.0,
            end_x_pixels: 3.0,
            end_y_pixels: 4.0,
            thickness_pixels: 5.0,
            color_linear_rgba: [0.1, 0.2, 0.3, 1.0],
        };
        let mut bytes: Vec<u8> = Vec::new();
        line.write_bytes(&mut bytes);
        assert_eq!(bytes.len(), BYTES_PER_INSTANCE);
    }
}
