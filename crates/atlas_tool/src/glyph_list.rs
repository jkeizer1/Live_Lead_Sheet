//! Which glyphs go into the atlas, from which font.

/// A font the atlas is built from. Files are in `assets/fonts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasFont {
    /// Music symbols (SMuFL), by Steinberg; SIL Open Font Licence.
    Bravura,
    /// Chord symbols, numerals and labels.
    DejaVuSans,
}

impl AtlasFont {
    /// The font file, relative to `assets/fonts`.
    #[must_use]
    pub fn file_path(self) -> &'static str {
        match self {
            AtlasFont::Bravura => "bravura/Bravura.otf",
            AtlasFont::DejaVuSans => "dejavu/DejaVuSans.ttf",
        }
    }

    /// The font's name in the glyph table.
    #[must_use]
    pub fn table_name(self) -> &'static str {
        match self {
            AtlasFont::Bravura => "bravura",
            AtlasFont::DejaVuSans => "dejavu_sans",
        }
    }
}

/// A music glyph, named as in the SMuFL specification so it can be looked
/// up in `Bravura.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusicGlyph {
    pub smufl_name: &'static str,
    pub codepoint: char,
}

/// The music glyphs the lead sheet draws: clefs, noteheads (round and
/// slash), rests, flags, accidentals, the augmentation dot and
/// time-signature digits.
pub const MUSIC_GLYPHS: [MusicGlyph; 36] = [
    MusicGlyph {
        smufl_name: "gClef",
        codepoint: '\u{E050}',
    },
    MusicGlyph {
        smufl_name: "fClef",
        codepoint: '\u{E062}',
    },
    MusicGlyph {
        smufl_name: "unpitchedPercussionClef1",
        codepoint: '\u{E069}',
    },
    MusicGlyph {
        smufl_name: "noteheadWhole",
        codepoint: '\u{E0A2}',
    },
    MusicGlyph {
        smufl_name: "noteheadHalf",
        codepoint: '\u{E0A3}',
    },
    MusicGlyph {
        smufl_name: "noteheadBlack",
        codepoint: '\u{E0A4}',
    },
    MusicGlyph {
        smufl_name: "noteheadSlashVerticalEnds",
        codepoint: '\u{E100}',
    },
    MusicGlyph {
        smufl_name: "noteheadSlashWhiteWhole",
        codepoint: '\u{E102}',
    },
    MusicGlyph {
        smufl_name: "noteheadSlashWhiteHalf",
        codepoint: '\u{E103}',
    },
    MusicGlyph {
        smufl_name: "restWhole",
        codepoint: '\u{E4E3}',
    },
    MusicGlyph {
        smufl_name: "restHalf",
        codepoint: '\u{E4E4}',
    },
    MusicGlyph {
        smufl_name: "restQuarter",
        codepoint: '\u{E4E5}',
    },
    MusicGlyph {
        smufl_name: "rest8th",
        codepoint: '\u{E4E6}',
    },
    MusicGlyph {
        smufl_name: "rest16th",
        codepoint: '\u{E4E7}',
    },
    MusicGlyph {
        smufl_name: "flag8thUp",
        codepoint: '\u{E240}',
    },
    MusicGlyph {
        smufl_name: "flag8thDown",
        codepoint: '\u{E241}',
    },
    MusicGlyph {
        smufl_name: "flag16thUp",
        codepoint: '\u{E242}',
    },
    MusicGlyph {
        smufl_name: "flag16thDown",
        codepoint: '\u{E243}',
    },
    MusicGlyph {
        smufl_name: "accidentalFlat",
        codepoint: '\u{E260}',
    },
    MusicGlyph {
        smufl_name: "accidentalNatural",
        codepoint: '\u{E261}',
    },
    MusicGlyph {
        smufl_name: "accidentalSharp",
        codepoint: '\u{E262}',
    },
    MusicGlyph {
        smufl_name: "accidentalDoubleSharp",
        codepoint: '\u{E263}',
    },
    MusicGlyph {
        smufl_name: "accidentalDoubleFlat",
        codepoint: '\u{E264}',
    },
    MusicGlyph {
        smufl_name: "augmentationDot",
        codepoint: '\u{E1E7}',
    },
    MusicGlyph {
        smufl_name: "timeSig0",
        codepoint: '\u{E080}',
    },
    MusicGlyph {
        smufl_name: "timeSig1",
        codepoint: '\u{E081}',
    },
    MusicGlyph {
        smufl_name: "timeSig2",
        codepoint: '\u{E082}',
    },
    MusicGlyph {
        smufl_name: "timeSig3",
        codepoint: '\u{E083}',
    },
    MusicGlyph {
        smufl_name: "timeSig4",
        codepoint: '\u{E084}',
    },
    MusicGlyph {
        smufl_name: "timeSig5",
        codepoint: '\u{E085}',
    },
    MusicGlyph {
        smufl_name: "timeSig6",
        codepoint: '\u{E086}',
    },
    MusicGlyph {
        smufl_name: "timeSig7",
        codepoint: '\u{E087}',
    },
    MusicGlyph {
        smufl_name: "timeSig8",
        codepoint: '\u{E088}',
    },
    MusicGlyph {
        smufl_name: "timeSig9",
        codepoint: '\u{E089}',
    },
    MusicGlyph {
        smufl_name: "timeSigCommon",
        codepoint: '\u{E08A}',
    },
    MusicGlyph {
        smufl_name: "timeSigCutCommon",
        codepoint: '\u{E08B}',
    },
];

/// Text characters beyond printable ASCII: sharp, flat, natural, degree
/// (diminished), o-slash (half-diminished), delta and triangle (major
/// seventh, in lead sheets that use them).
const TEXT_EXTRA_CHARACTERS: [char; 7] = ['♯', '♭', '♮', '°', 'ø', 'Δ', '△'];

/// Every text character the atlas holds: printable ASCII (space to tilde)
/// and the extras above.
#[must_use]
pub fn text_characters() -> Vec<char> {
    let mut characters: Vec<char> = (' '..='~').collect();
    characters.extend_from_slice(&TEXT_EXTRA_CHARACTERS);
    characters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_characters_are_printable_ascii_and_the_extras() {
        let characters: Vec<char> = text_characters();
        // 95 printable ASCII characters (0x20 to 0x7E) and 7 extras.
        assert_eq!(characters.len(), 95 + 7);
        assert!(characters.contains(&'C'));
        assert!(characters.contains(&'♭'));
    }

    #[test]
    fn music_glyphs_have_distinct_names_and_codepoints() {
        for (index, glyph) in MUSIC_GLYPHS.iter().enumerate() {
            for other in &MUSIC_GLYPHS[index + 1..] {
                assert_ne!(glyph.smufl_name, other.smufl_name);
                assert_ne!(glyph.codepoint, other.codepoint);
            }
        }
    }
}
