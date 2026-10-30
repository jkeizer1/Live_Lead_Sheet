//! Greyscale images in the binary PGM format: a three-line text header,
//! then one byte per pixel, row by row from the top. Simple enough to need
//! no crate, and image viewers open it.

use std::path::Path;

/// Writes a greyscale image.
///
/// # Errors
///
/// When the pixel count does not match the size, or the file cannot be
/// written.
pub fn write_pgm(
    path: &Path,
    width_pixels: u32,
    height_pixels: u32,
    values: &[u8],
) -> Result<(), String> {
    let expected_pixels: u64 = u64::from(width_pixels) * u64::from(height_pixels);
    if values.len() as u64 != expected_pixels {
        return Err(format!(
            "Writing {}: {} pixel values for a {width_pixels} × {height_pixels} image \
             ({expected_pixels} pixels).",
            path.display(),
            values.len()
        ));
    }
    let mut bytes: Vec<u8> = format!("P5\n{width_pixels} {height_pixels}\n255\n").into_bytes();
    bytes.extend_from_slice(values);
    std::fs::write(path, bytes).map_err(|e| format!("Writing {}: {e}", path.display()))
}
