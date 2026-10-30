//! Checks the distance fields on a few glyphs before the atlas format is
//! settled: a notehead, the G clef, a sharp (Bravura) and the letter C
//! (DejaVu Sans). Writes two images to `scratch/atlas_check/`:
//!
//! - `fields.pgm` — the raw distance fields side by side (grey at the edge,
//!   white inside, black outside);
//! - `shapes_4x.pgm` — the glyphs redrawn from those fields at 4 times
//!   their size, as the glyph shader will; edges should stay sharp.
//!
//! `cargo run --release -p atlas_tool --example check_glyphs`

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use atlas_tool::distance_field::{DistanceField, FieldSettings, distance_field};
use atlas_tool::glyph_list::AtlasFont;
use atlas_tool::outline::{FlatOutline, flatten_glyph};
use atlas_tool::pgm::write_pgm;
use ttf_parser::Face;

/// One em is 4 staff spaces in SMuFL fonts; 96 px/em gives 24 px staff
/// spaces, the size the strip uses at 1440 px high.
const PIXELS_PER_EM: f64 = 96.0;
const SPREAD_PIXELS: f64 = 8.0;
/// Space between glyphs in the check image.
const GAP_PIXELS: u32 = 8;
/// Enlargement for the redrawn shapes.
const SCALE_UP: u32 = 4;

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
    let project_root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fonts_folder: PathBuf = project_root.join("assets/fonts");
    let output_folder: PathBuf = project_root.join("scratch/atlas_check");
    std::fs::create_dir_all(&output_folder)
        .map_err(|e| format!("Creating {}: {e}", output_folder.display()))?;

    let bravura_bytes: Vec<u8> = read_font(&fonts_folder, AtlasFont::Bravura)?;
    let dejavu_bytes: Vec<u8> = read_font(&fonts_folder, AtlasFont::DejaVuSans)?;
    let bravura: Face<'_> = Face::parse(&bravura_bytes, 0)
        .map_err(|e| format!("Reading {}: {e}", AtlasFont::Bravura.file_path()))?;
    let dejavu: Face<'_> = Face::parse(&dejavu_bytes, 0)
        .map_err(|e| format!("Reading {}: {e}", AtlasFont::DejaVuSans.file_path()))?;

    let checks: [(&Face<'_>, &str, char); 4] = [
        (&bravura, "Bravura noteheadBlack", '\u{E0A4}'),
        (&bravura, "Bravura gClef", '\u{E050}'),
        (&bravura, "Bravura accidentalSharp", '\u{E262}'),
        (&dejavu, "DejaVu Sans", 'C'),
    ];
    let mut fields: Vec<DistanceField> = Vec::new();
    for (face, font_name, character) in checks {
        let outline: FlatOutline = flatten_glyph(face, character, font_name)?;
        let settings: FieldSettings = FieldSettings {
            pixels_per_em: PIXELS_PER_EM,
            font_units_per_em: f64::from(face.units_per_em()),
            spread_pixels: SPREAD_PIXELS,
        };
        let field: DistanceField = distance_field(&outline, settings)?;
        println!(
            "{font_name} U+{:04X}: {} segments, field {} × {} px",
            u32::from(character),
            outline.segments.len(),
            field.width_pixels,
            field.height_pixels
        );
        fields.push(field);
    }

    let (width_pixels, height_pixels, values): (u32, u32, Vec<u8>) = side_by_side(&fields, 1)?;
    let fields_path: PathBuf = output_folder.join("fields.pgm");
    write_pgm(&fields_path, width_pixels, height_pixels, &values)?;
    let (width_pixels, height_pixels, values): (u32, u32, Vec<u8>) =
        side_by_side(&fields, SCALE_UP)?;
    let shapes_path: PathBuf = output_folder.join("shapes_4x.pgm");
    write_pgm(&shapes_path, width_pixels, height_pixels, &values)?;
    println!(
        "Wrote {} and {}",
        fields_path.display(),
        shapes_path.display()
    );
    Ok(())
}

fn read_font(fonts_folder: &Path, font: AtlasFont) -> Result<Vec<u8>, String> {
    let path: PathBuf = fonts_folder.join(font.file_path());
    std::fs::read(&path).map_err(|e| format!("Reading font {}: {e}", path.display()))
}

/// Lays the fields out in a row, top-aligned. At `scale` 1 the raw field
/// values are copied; above 1 each glyph is redrawn from its field.
fn side_by_side(fields: &[DistanceField], scale: u32) -> Result<(u32, u32, Vec<u8>), String> {
    let width_pixels: u32 = fields
        .iter()
        .map(|field| field.width_pixels * scale + GAP_PIXELS)
        .sum::<u32>()
        + GAP_PIXELS;
    let height_pixels: u32 = fields
        .iter()
        .map(|field| field.height_pixels * scale)
        .max()
        .ok_or("No glyphs to lay out.")?
        + 2 * GAP_PIXELS;
    let mut values: Vec<u8> = vec![0; index(0, height_pixels, width_pixels)?];
    let mut left_pixels: u32 = GAP_PIXELS;
    for field in fields {
        for row in 0..field.height_pixels * scale {
            for column in 0..field.width_pixels * scale {
                let value: u8 = if scale == 1 {
                    field.values[index(column, row, field.width_pixels)?]
                } else {
                    redrawn(field, column, row, scale)?
                };
                values[index(left_pixels + column, GAP_PIXELS + row, width_pixels)?] = value;
            }
        }
        left_pixels += field.width_pixels * scale + GAP_PIXELS;
    }
    Ok((width_pixels, height_pixels, values))
}

/// What the glyph shader will do: sample the field between its pixels
/// (bilinear), turn the value back into a distance in output pixels, and
/// cover the pixel by how far inside the edge it is, over one pixel.
fn redrawn(field: &DistanceField, column: u32, row: u32, scale: u32) -> Result<u8, String> {
    // Output pixel centre, in field pixels.
    let x: f64 = (f64::from(column) + 0.5) / f64::from(scale) - 0.5;
    let y: f64 = (f64::from(row) + 0.5) / f64::from(scale) - 0.5;
    let value: f64 = bilinear(field, x, y)?;
    // value/255 − 0.5 spans ±0.5 for ±spread field px; × 2 × spread gives
    // field px; × scale gives output px.
    let distance_output_pixels: f64 =
        (value / 255.0 - 0.5) * 2.0 * SPREAD_PIXELS * f64::from(scale);
    let coverage: f64 = (distance_output_pixels + 0.5).clamp(0.0, 1.0);
    // 0 ≤ coverage × 255 ≤ 255.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let shade: u8 = (coverage * 255.0).round() as u8;
    Ok(shade)
}

fn bilinear(field: &DistanceField, x: f64, y: f64) -> Result<f64, String> {
    let largest_column: f64 = f64::from(field.width_pixels - 1);
    let largest_row: f64 = f64::from(field.height_pixels - 1);
    let x: f64 = x.clamp(0.0, largest_column);
    let y: f64 = y.clamp(0.0, largest_row);
    // Clamped to the field just above, so these are valid pixel numbers.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (left, top): (u32, u32) = (x.floor() as u32, y.floor() as u32);
    let right: u32 = (left + 1).min(field.width_pixels - 1);
    let bottom: u32 = (top + 1).min(field.height_pixels - 1);
    let across: f64 = x - f64::from(left);
    let down: f64 = y - f64::from(top);
    let at = |column: u32, row: u32| -> Result<f64, String> {
        Ok(f64::from(
            field.values[index(column, row, field.width_pixels)?],
        ))
    };
    let upper: f64 = at(left, top)? * (1.0 - across) + at(right, top)? * across;
    let lower: f64 = at(left, bottom)? * (1.0 - across) + at(right, bottom)? * across;
    Ok(upper * (1.0 - down) + lower * down)
}

fn index(column: u32, row: u32, width_pixels: u32) -> Result<usize, String> {
    usize::try_from(u64::from(row) * u64::from(width_pixels) + u64::from(column))
        .map_err(|e| format!("Pixel ({column}, {row}): {e}"))
}
