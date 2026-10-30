//! Draws the spike frame's lines and glyphs on the CPU into a greyscale
//! image, to check the layout without a window or GPU. The wheel's wedges
//! are left out (its labels are drawn); brightness stands for colour.
//!
//! `cargo run --release -p render --example layout_preview -- [WIDTH HEIGHT]`
//!
//! Writes `scratch/layout_preview.pgm` (default 2560 × 1440).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use render::glyph_atlas::GlyphAtlas;
use render::glyph_instance::GlyphInstance;
use render::line_instance::LineInstance;
use render::spike_music::spike_music;
use render::strip_layout::{StripGeometry, strip_lines};
use render::wheel_instance::WheelInstance;
use render::wheel_layout::{wheel_labels, wheel_layout};

/// The playhead in the preview: a third of the way through the strip.
const PREVIEW_PLAYHEAD_FRACTION: f32 = 0.33;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (width_pixels, height_pixels): (u32, u32) = match arguments.as_slice() {
        [] => (2560, 1440),
        [width, height] => (
            width
                .parse()
                .map_err(|_| format!("Width {width:?} is not a whole number."))?,
            height
                .parse()
                .map_err(|_| format!("Height {height:?} is not a whole number."))?,
        ),
        _ => return Err("Give no size, or WIDTH HEIGHT in pixels.".to_string()),
    };
    // Preview sizes are far below 2^24, so the conversion to f32 is exact.
    #[allow(clippy::cast_precision_loss)]
    let (width, height): (f32, f32) = (width_pixels as f32, height_pixels as f32);

    let atlas: GlyphAtlas = GlyphAtlas::embedded().map_err(|e| e.to_string())?;
    let geometry: StripGeometry = StripGeometry::new(width, height);
    let mut lines: Vec<LineInstance> = strip_lines(&geometry, PREVIEW_PLAYHEAD_FRACTION);
    let mut glyphs: Vec<GlyphInstance> = Vec::new();
    spike_music(&geometry, &atlas, &mut lines, &mut glyphs).map_err(|e| e.to_string())?;
    let wheel: WheelInstance = wheel_layout(width, height, 0, true);
    wheel_labels(&wheel, &atlas, &mut glyphs).map_err(|e| e.to_string())?;

    let mut image: Image = Image::new(width_pixels, height_pixels)?;
    for line in &lines {
        image.draw_line(line);
    }
    for glyph in &glyphs {
        image.draw_glyph(glyph, &atlas);
    }
    let path: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scratch/layout_preview.pgm");
    image.write_pgm(&path)?;
    println!(
        "{} lines, {} glyphs: wrote {}",
        lines.len(),
        glyphs.len(),
        path.display()
    );
    Ok(())
}

struct Image {
    width_pixels: u32,
    height_pixels: u32,
    /// Brightness 0 to 1 per pixel, row by row from the top.
    brightness: Vec<f32>,
}

impl Image {
    fn new(width_pixels: u32, height_pixels: u32) -> Result<Image, String> {
        let pixels: usize = usize::try_from(u64::from(width_pixels) * u64::from(height_pixels))
            .map_err(|e| e.to_string())?;
        Ok(Image {
            width_pixels,
            height_pixels,
            brightness: vec![0.0; pixels],
        })
    }

    /// Blends `amount` (0 to 1) of `brightness` into the pixel, if inside.
    fn blend(&mut self, column: i64, row: i64, brightness: f32, amount: f32) {
        let (Ok(column), Ok(row)): (Result<u32, _>, Result<u32, _>) =
            (u32::try_from(column), u32::try_from(row))
        else {
            return;
        };
        if column >= self.width_pixels || row >= self.height_pixels {
            return;
        }
        let Ok(index): Result<usize, _> =
            usize::try_from(u64::from(row) * u64::from(self.width_pixels) + u64::from(column))
        else {
            return;
        };
        let old: f32 = self.brightness[index];
        self.brightness[index] = old + (brightness - old) * amount;
    }

    /// Lines here are horizontal or vertical (staff, barlines, stems,
    /// playhead), so a filled rectangle is enough for the preview.
    fn draw_line(&mut self, line: &LineInstance) {
        let half: f32 = line.thickness_pixels / 2.0;
        let left: f32 = line.start_x_pixels.min(line.end_x_pixels);
        let right: f32 = line.start_x_pixels.max(line.end_x_pixels);
        let top: f32 = line.start_y_pixels.min(line.end_y_pixels);
        let bottom: f32 = line.start_y_pixels.max(line.end_y_pixels);
        let horizontal: bool = (line.start_y_pixels - line.end_y_pixels).abs() < 0.001;
        let (x0, x1, y0, y1): (f32, f32, f32, f32) = if horizontal {
            (left, right, top - half, top + half)
        } else {
            (left - half, left + half, top, bottom)
        };
        let shade: f32 = luminance(line.color_linear_rgba);
        for row in pixel_range(y0, y1) {
            for column in pixel_range(x0, x1) {
                self.blend(column, row, shade, 1.0);
            }
        }
    }

    /// As the glyph shader: sample the field, turn it into a distance in
    /// screen pixels, cover by how far inside the edge the pixel is.
    fn draw_glyph(&mut self, glyph: &GlyphInstance, atlas: &GlyphAtlas) {
        let shade: f32 = luminance(glyph.color_linear_rgba);
        let x0: f32 = glyph.left_x_pixels;
        let y0: f32 = glyph.top_y_pixels;
        for row in pixel_range(y0, y0 + glyph.height_pixels) {
            for column in pixel_range(x0, x0 + glyph.width_pixels) {
                // Pixel centre as a fraction across the glyph's rectangle.
                #[allow(clippy::cast_precision_loss)]
                let (across, down): (f32, f32) = (
                    ((column as f32 + 0.5) - x0) / glyph.width_pixels,
                    ((row as f32 + 0.5) - y0) / glyph.height_pixels,
                );
                let u: f32 = glyph.atlas_left + (glyph.atlas_right - glyph.atlas_left) * across;
                let v: f32 = glyph.atlas_top + (glyph.atlas_bottom - glyph.atlas_top) * down;
                let value: f32 = sample(atlas, u, v);
                let distance_pixels: f32 = (value - 0.5) * glyph.distance_scale_pixels;
                let coverage: f32 = (distance_pixels + 0.5).clamp(0.0, 1.0);
                self.blend(column, row, shade, coverage);
            }
        }
    }

    fn write_pgm(&self, path: &Path) -> Result<(), String> {
        let mut bytes: Vec<u8> =
            format!("P5\n{} {}\n255\n", self.width_pixels, self.height_pixels).into_bytes();
        // Background: the theme's dark grey, sRGB 24 of 255.
        for brightness in &self.brightness {
            let srgb: f32 = to_srgb(*brightness).max(24.0 / 255.0);
            // 0 ≤ srgb × 255 ≤ 255.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            bytes.push((srgb * 255.0).round() as u8);
        }
        std::fs::write(path, bytes).map_err(|e| format!("Writing {}: {e}", path.display()))
    }
}

/// Whole pixels whose centres lie between `from` and `to`.
fn pixel_range(from: f32, to: f32) -> std::ops::Range<i64> {
    // Window coordinates are far inside i64's range.
    #[allow(clippy::cast_possible_truncation)]
    let range: std::ops::Range<i64> = ((from - 0.5).ceil() as i64)..((to - 0.5).ceil() as i64);
    range
}

/// Bilinear sample of the atlas at (u, v), 0 to 1 across and down; 0 to 1.
fn sample(atlas: &GlyphAtlas, u: f32, v: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let (width, height): (f32, f32) = (atlas.width_pixels as f32, atlas.height_pixels as f32);
    let x: f32 = (u * width - 0.5).clamp(0.0, width - 1.0);
    let y: f32 = (v * height - 0.5).clamp(0.0, height - 1.0);
    // Clamped to the image just above.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (left, top): (usize, usize) = (x.floor() as usize, y.floor() as usize);
    // u32 to usize widens on the 64-bit machines this runs on.
    let width_whole: usize = atlas.width_pixels as usize;
    let height_whole: usize = atlas.height_pixels as usize;
    let right: usize = (left + 1).min(width_whole - 1);
    let bottom: usize = (top + 1).min(height_whole - 1);
    #[allow(clippy::cast_precision_loss)]
    let (across, down): (f32, f32) = (x - left as f32, y - top as f32);
    let at = |column: usize, row: usize| -> f32 {
        f32::from(atlas.image[row * width_whole + column]) / 255.0
    };
    let upper: f32 = at(left, top) * (1.0 - across) + at(right, top) * across;
    let lower: f32 = at(left, bottom) * (1.0 - across) + at(right, bottom) * across;
    upper * (1.0 - down) + lower * down
}

/// Brightness of a linear-light colour (Rec. 709 weights).
fn luminance(color_linear_rgba: [f32; 4]) -> f32 {
    0.2126 * color_linear_rgba[0] + 0.7152 * color_linear_rgba[1] + 0.0722 * color_linear_rgba[2]
}

fn to_srgb(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}
