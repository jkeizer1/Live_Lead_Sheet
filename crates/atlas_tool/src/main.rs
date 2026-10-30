//! Builds the glyph atlas: reads the fonts in `assets/fonts`, computes each
//! glyph's distance field, packs them, and writes `assets/atlas/glyphs.pgm`
//! and `assets/atlas/glyphs.txt`.
//!
//! `cargo run --release -p atlas_tool`

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use atlas_tool::atlas::{AtlasEntry, PackedAtlas, pack, table_text};
use atlas_tool::distance_field::{DistanceField, FieldSettings, distance_field};
use atlas_tool::glyph_list::{AtlasFont, MUSIC_GLYPHS, text_characters};
use atlas_tool::outline::{FlatOutline, flatten_glyph, glyph_advance_font_units};
use atlas_tool::pgm::write_pgm;
use ttf_parser::Face;

/// One em is 4 staff spaces in SMuFL fonts; 96 px/em gives 24 px staff
/// spaces, the strip's size at 1440 px high. The shader scales from here.
const PIXELS_PER_EM: f64 = 96.0;
/// How far the fields measure from the outline, in atlas pixels; enough for
/// sharp edges well below and above the atlas size (checked by the
/// `check_glyphs` example).
const SPREAD_PIXELS: f64 = 8.0;
const ATLAS_WIDTH_PIXELS: u32 = 1024;

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
    let output_folder: PathBuf = project_root.join("assets/atlas");
    std::fs::create_dir_all(&output_folder)
        .map_err(|e| format!("Creating {}: {e}", output_folder.display()))?;

    let bravura_bytes: Vec<u8> = read_font(&fonts_folder, AtlasFont::Bravura)?;
    let dejavu_bytes: Vec<u8> = read_font(&fonts_folder, AtlasFont::DejaVuSans)?;
    let bravura: Face<'_> = parse_font(&bravura_bytes, AtlasFont::Bravura)?;
    let dejavu: Face<'_> = parse_font(&dejavu_bytes, AtlasFont::DejaVuSans)?;

    let mut entries: Vec<AtlasEntry> = Vec::new();
    for glyph in MUSIC_GLYPHS {
        entries.push(entry_for(
            &bravura,
            AtlasFont::Bravura,
            glyph.codepoint,
            glyph.smufl_name,
        )?);
    }
    for character in text_characters() {
        entries.push(entry_for(&dejavu, AtlasFont::DejaVuSans, character, "-")?);
    }

    let atlas: PackedAtlas = pack(&entries, ATLAS_WIDTH_PIXELS)?;
    let image_path: PathBuf = output_folder.join("glyphs.pgm");
    write_pgm(
        &image_path,
        atlas.width_pixels,
        atlas.height_pixels,
        &atlas.values,
    )?;
    let table_path: PathBuf = output_folder.join("glyphs.txt");
    let table: String = table_text(&entries, &atlas, PIXELS_PER_EM, SPREAD_PIXELS)?;
    std::fs::write(&table_path, table)
        .map_err(|e| format!("Writing {}: {e}", table_path.display()))?;
    println!(
        "{} glyphs in a {} × {} px atlas: {} and {}",
        entries.len(),
        atlas.width_pixels,
        atlas.height_pixels,
        image_path.display(),
        table_path.display()
    );
    Ok(())
}

/// One atlas entry. Whitespace has no outline and gets only its advance;
/// any other character without an outline is an error.
fn entry_for(
    face: &Face<'_>,
    font: AtlasFont,
    codepoint: char,
    name: &'static str,
) -> Result<AtlasEntry, String> {
    let font_units_per_em: f64 = f64::from(face.units_per_em());
    let font_name: &str = font.table_name();
    let (field, advance_font_units): (Option<DistanceField>, f64) = if codepoint.is_whitespace() {
        (None, glyph_advance_font_units(face, codepoint, font_name)?)
    } else {
        let outline: FlatOutline = flatten_glyph(face, codepoint, font_name)?;
        let settings: FieldSettings = FieldSettings {
            pixels_per_em: PIXELS_PER_EM,
            font_units_per_em,
            spread_pixels: SPREAD_PIXELS,
        };
        (
            Some(distance_field(&outline, settings)?),
            outline.advance_font_units,
        )
    };
    Ok(AtlasEntry {
        font,
        codepoint,
        name,
        field,
        // font units ÷ font units/em = em
        advance_ems: advance_font_units / font_units_per_em,
    })
}

fn read_font(fonts_folder: &Path, font: AtlasFont) -> Result<Vec<u8>, String> {
    let path: PathBuf = fonts_folder.join(font.file_path());
    std::fs::read(&path).map_err(|e| format!("Reading font {}: {e}", path.display()))
}

fn parse_font(bytes: &[u8], font: AtlasFont) -> Result<Face<'_>, String> {
    Face::parse(bytes, 0).map_err(|e| format!("Reading font {}: {e}", font.file_path()))
}
