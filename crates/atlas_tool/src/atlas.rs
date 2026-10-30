//! Packs the glyphs' distance fields into one image and writes the glyph
//! table that says where each one is.

use std::fmt::Write as _;

use crate::distance_field::DistanceField;
use crate::glyph_list::AtlasFont;

/// Empty pixels between neighbouring fields, so sampling between pixels at
/// one field's edge never picks up its neighbour.
const GAP_PIXELS: u32 = 2;

/// One glyph to place in the atlas.
#[derive(Clone, Debug, PartialEq)]
pub struct AtlasEntry {
    pub font: AtlasFont,
    pub codepoint: char,
    /// The SMuFL name for music glyphs; `-` for text characters, which are
    /// known by their codepoint.
    pub name: &'static str,
    /// `None` for a character with no ink, such as the space: it has an
    /// advance and nothing to draw.
    pub field: Option<DistanceField>,
    /// How far the pen moves on after this glyph, in ems.
    pub advance_ems: f64,
}

/// Where a field landed in the atlas, top-left corner, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub x_pixels: u32,
    pub y_pixels: u32,
}

/// The packed image and, for each entry in the same order, where its field
/// is (`None` for an entry without a field).
#[derive(Clone, Debug, PartialEq)]
pub struct PackedAtlas {
    pub width_pixels: u32,
    pub height_pixels: u32,
    /// One byte per pixel, row by row from the top; 0 (far outside) where
    /// no glyph is.
    pub values: Vec<u8>,
    pub placements: Vec<Option<Placement>>,
}

/// Packs the fields in rows ("shelves"): tallest first, left to right, a new
/// row when the next one does not fit the width. Simple, and for about 140
/// glyphs it wastes little space.
///
/// # Errors
///
/// When a field is wider than the atlas.
pub fn pack(entries: &[AtlasEntry], width_pixels: u32) -> Result<PackedAtlas, String> {
    let mut order: Vec<usize> = (0..entries.len()).collect();
    // Tallest first; entries without a field go last and take no space.
    order.sort_by_key(|&index| {
        std::cmp::Reverse(
            entries[index]
                .field
                .as_ref()
                .map_or(0, |field| field.height_pixels),
        )
    });

    let mut placements: Vec<Option<Placement>> = vec![None; entries.len()];
    let mut cursor_x_pixels: u32 = GAP_PIXELS;
    let mut shelf_top_pixels: u32 = GAP_PIXELS;
    let mut shelf_height_pixels: u32 = 0;
    for index in order {
        let Some(field) = entries[index].field.as_ref() else {
            continue;
        };
        if field.width_pixels + 2 * GAP_PIXELS > width_pixels {
            return Err(format!(
                "Glyph {} U+{:04X} is {} px wide; the atlas is {width_pixels} px wide.",
                entries[index].font.table_name(),
                u32::from(entries[index].codepoint),
                field.width_pixels
            ));
        }
        if cursor_x_pixels + field.width_pixels + GAP_PIXELS > width_pixels {
            shelf_top_pixels += shelf_height_pixels + GAP_PIXELS;
            cursor_x_pixels = GAP_PIXELS;
            shelf_height_pixels = 0;
        }
        placements[index] = Some(Placement {
            x_pixels: cursor_x_pixels,
            y_pixels: shelf_top_pixels,
        });
        cursor_x_pixels += field.width_pixels + GAP_PIXELS;
        shelf_height_pixels = shelf_height_pixels.max(field.height_pixels);
    }
    let height_pixels: u32 = shelf_top_pixels + shelf_height_pixels + GAP_PIXELS;

    let mut values: Vec<u8> = vec![0; pixel_index(0, height_pixels, width_pixels)?];
    for (entry, placement) in entries.iter().zip(&placements) {
        let (Some(field), Some(placement)) = (entry.field.as_ref(), placement) else {
            continue;
        };
        for row in 0..field.height_pixels {
            let from: usize = pixel_index(0, row, field.width_pixels)?;
            let to: usize =
                pixel_index(placement.x_pixels, placement.y_pixels + row, width_pixels)?;
            let row_length: usize =
                usize::try_from(field.width_pixels).map_err(|e| e.to_string())?;
            values[to..to + row_length].copy_from_slice(&field.values[from..from + row_length]);
        }
    }
    Ok(PackedAtlas {
        width_pixels,
        height_pixels,
        values,
        placements,
    })
}

/// The glyph table as text: a header with the scale, then one line per
/// glyph. Sizes and offsets are in ems so layout can draw at any size.
///
/// # Errors
///
/// When `atlas` was not packed from `entries`.
pub fn table_text(
    entries: &[AtlasEntry],
    atlas: &PackedAtlas,
    pixels_per_em: f64,
    spread_pixels: f64,
) -> Result<String, String> {
    if entries.len() != atlas.placements.len() {
        return Err(format!(
            "The glyph table has {} entries but the atlas placed {}.",
            entries.len(),
            atlas.placements.len()
        ));
    }
    let mut text: String = String::new();
    // Writing to a String cannot fail; `let _` is safe for `write!` here.
    let _ = writeln!(
        text,
        "# Live Lead Sheet glyph atlas. Written by atlas_tool; do not edit."
    );
    let _ = writeln!(
        text,
        "# Image: glyphs.pgm. Fonts: Bravura (SIL OFL 1.1), DejaVu Sans."
    );
    let _ = writeln!(text, "pixels_per_em {pixels_per_em}");
    let _ = writeln!(text, "spread_pixels {spread_pixels}");
    let _ = writeln!(text, "atlas_width_pixels {}", atlas.width_pixels);
    let _ = writeln!(text, "atlas_height_pixels {}", atlas.height_pixels);
    let _ = writeln!(
        text,
        "# glyph font codepoint name x_pixels y_pixels width_pixels height_pixels \
         left_ems top_ems advance_ems"
    );
    let _ = writeln!(
        text,
        "# x, y, width, height: the field in the image. left, top: the field's \
         top-left corner from the glyph's origin, x right, y up. No field: 0 0 0 0 0 0."
    );
    for (entry, placement) in entries.iter().zip(&atlas.placements) {
        let (x_pixels, y_pixels, width_pixels, height_pixels, left_ems, top_ems): (
            u32,
            u32,
            u32,
            u32,
            f64,
            f64,
        ) = match (entry.field.as_ref(), placement) {
            (Some(field), Some(placement)) => (
                placement.x_pixels,
                placement.y_pixels,
                field.width_pixels,
                field.height_pixels,
                // px ÷ px/em = em
                field.left_from_origin_pixels / pixels_per_em,
                field.top_from_origin_pixels / pixels_per_em,
            ),
            (None, None) => (0, 0, 0, 0, 0.0, 0.0),
            _ => {
                return Err(format!(
                    "Glyph {} U+{:04X}: has a field but no place in the atlas, or the reverse.",
                    entry.font.table_name(),
                    u32::from(entry.codepoint)
                ));
            }
        };
        let _ = writeln!(
            text,
            "glyph {} {:04X} {} {x_pixels} {y_pixels} {width_pixels} {height_pixels} \
             {left_ems:.6} {top_ems:.6} {:.6}",
            entry.font.table_name(),
            u32::from(entry.codepoint),
            entry.name,
            entry.advance_ems
        );
    }
    Ok(text)
}

fn pixel_index(column: u32, row: u32, width_pixels: u32) -> Result<usize, String> {
    usize::try_from(u64::from(row) * u64::from(width_pixels) + u64::from(column))
        .map_err(|e| format!("Atlas pixel ({column}, {row}): {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(codepoint: char, width_pixels: u32, height_pixels: u32, value: u8) -> AtlasEntry {
        let pixels: usize = usize::try_from(width_pixels * height_pixels).unwrap();
        AtlasEntry {
            font: AtlasFont::DejaVuSans,
            codepoint,
            name: "-",
            field: Some(DistanceField {
                width_pixels,
                height_pixels,
                values: vec![value; pixels],
                left_from_origin_pixels: -2.0,
                top_from_origin_pixels: 10.0,
            }),
            advance_ems: 0.5,
        }
    }

    #[test]
    fn fields_do_not_overlap_and_keep_their_pixels() {
        let entries: Vec<AtlasEntry> = vec![
            entry('a', 30, 20, 11),
            entry('b', 30, 40, 22),
            entry('c', 30, 10, 33),
            AtlasEntry {
                font: AtlasFont::DejaVuSans,
                codepoint: ' ',
                name: "-",
                field: None,
                advance_ems: 0.3,
            },
        ];
        // 70 px wide: two 30 px fields per row (2 + 30 + 2 + 30 + 2 = 66).
        let atlas: PackedAtlas = pack(&entries, 70).unwrap();
        assert_eq!(atlas.placements[3], None);
        for (entry, placement) in entries.iter().zip(&atlas.placements).take(3) {
            let field: &DistanceField = entry.field.as_ref().unwrap();
            let placement: Placement = placement.unwrap();
            // Every pixel of the field arrived intact.
            for row in 0..field.height_pixels {
                for column in 0..field.width_pixels {
                    let at: usize = pixel_index(
                        placement.x_pixels + column,
                        placement.y_pixels + row,
                        atlas.width_pixels,
                    )
                    .unwrap();
                    assert_eq!(atlas.values[at], field.values[0]);
                }
            }
        }
        // Tallest (b) first at the top left, a beside it, c on the next row.
        assert_eq!(
            atlas.placements[1],
            Some(Placement {
                x_pixels: 2,
                y_pixels: 2
            })
        );
        assert_eq!(
            atlas.placements[0],
            Some(Placement {
                x_pixels: 34,
                y_pixels: 2
            })
        );
        assert_eq!(
            atlas.placements[2],
            Some(Placement {
                x_pixels: 2,
                y_pixels: 44
            })
        );
        assert_eq!(atlas.height_pixels, 44 + 10 + 2);
    }

    #[test]
    fn a_field_wider_than_the_atlas_is_refused() {
        assert!(pack(&[entry('w', 100, 10, 1)], 50).is_err());
    }

    #[test]
    fn table_lists_every_glyph_in_ems() {
        let entries: Vec<AtlasEntry> = vec![entry('C', 30, 20, 1)];
        let atlas: PackedAtlas = pack(&entries, 64).unwrap();
        let text: String = table_text(&entries, &atlas, 100.0, 8.0).unwrap();
        // left −2 px ÷ 100 px/em = −0.02 em; top 10 px ÷ 100 px/em = 0.1 em.
        assert!(text.contains("glyph dejavu_sans 0043 - 2 2 30 20 -0.020000 0.100000 0.500000"));
        assert!(text.contains("pixels_per_em 100\n"));
    }
}
