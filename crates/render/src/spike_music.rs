//! Spike content for one chord part (EFS-1.5): a chord symbol per bar, the
//! part's rhythm in slash notation on the rhythm line, and each chord's
//! voicing as stacked noteheads at its change point on the pitch staff.
//! Hard-coded here; the real layout will take these from the vocabulary
//! through the published stream.
//!
//! Positions come from Bravura's metadata (`Bravura.json`), in staff spaces:
//! glyph origins sit on a staff line, stems attach at the notehead's
//! anchors, beam and ledger-line sizes are its engraving defaults.

use crate::error::RenderError;
use crate::glyph_atlas::{GlyphAtlas, GlyphFont};
use crate::glyph_instance::GlyphInstance;
use crate::line_instance::LineInstance;
use crate::strip_layout::{
    BARS_IN_STRIP, PitchStaffPlace, STEM_THICKNESS_SPACES, StripGeometry, horizontal_line,
    snapped_center_pixels, vertical_line, whole_thickness_pixels,
};
use crate::theme;
use crate::wheel_instance::WheelDegree;

const G_CLEF: char = '\u{E050}';
const PERCUSSION_CLEF: char = '\u{E069}';
const TIME_SIGNATURE_4: char = '\u{E084}';
const NOTEHEAD_WHOLE: char = '\u{E0A2}';
const SLASH_BLACK: char = '\u{E100}';
const SLASH_WHITE_WHOLE: char = '\u{E102}';
const SLASH_WHITE_HALF: char = '\u{E103}';
const REST_WHOLE: char = '\u{E4E3}';
const REST_HALF: char = '\u{E4E4}';
const REST_QUARTER: char = '\u{E4E5}';
const REST_EIGHTH: char = '\u{E4E6}';
const FLAG_EIGHTH_UP: char = '\u{E240}';
const AUGMENTATION_DOT: char = '\u{E1E7}';
const ACCIDENTAL_SHARP: char = '\u{E262}';

/// Eighth notes per bar in the spike's 4/4; beams group by the quarter-note
/// beat, two eighths.
const EIGHTHS_PER_BAR: u8 = 8;
const EIGHTHS_PER_BEAT: u8 = 2;

/// Clefs and time signatures, in pitch staff spaces from the staff's left.
const CLEF_X_SPACES: f32 = 0.8;
const TIME_SIGNATURE_X_SPACES: f32 = 4.4;

/// Rhythm line, in rhythm staff spaces. Stem anchors `stemUpSE`: black
/// slash (1.46, 0.656), white half slash (3.12, 1.0).
const SLASH_BLACK_STEM_X_RHYTHM_SPACES: f32 = 1.46;
const SLASH_BLACK_STEM_Y_RHYTHM_SPACES: f32 = 0.656;
const SLASH_HALF_STEM_X_RHYTHM_SPACES: f32 = 3.12;
const SLASH_HALF_STEM_Y_RHYTHM_SPACES: f32 = 1.0;
/// Stems end this far above the line.
const SLASH_STEM_TOP_RHYTHM_SPACES: f32 = 3.5;
/// Bravura beamThickness.
const BEAM_THICKNESS_RHYTHM_SPACES: f32 = 0.5;
/// `flag8thUp`'s `stemUpNW` anchor: the stem ends 0.04 below the origin.
const FLAG_STEM_Y_RHYTHM_SPACES: f32 = 0.04;
/// The dot after a dotted value: a gap after the notehead, half a space up.
const DOT_GAP_RHYTHM_SPACES: f32 = 0.3;
const DOT_RAISE_RHYTHM_SPACES: f32 = 0.5;
/// Room after a barline before a bar's first slash, and before the next
/// barline after the last slot (an eighth's flag reaches past its slot),
/// in pitch staff spaces.
const RHYTHM_LEAD_IN_SPACES: f32 = 0.8;
const RHYTHM_TRAIL_SPACES: f32 = 1.2;

/// Pitch staff, in pitch staff spaces at full size: the sharps' gaps, and
/// ledger lines (Bravura legerLineThickness, legerLineExtension).
const ACCIDENTAL_GAP_SPACES: f32 = 0.25;
const ACCIDENTAL_COLUMN_GAP_SPACES: f32 = 0.15;
/// Sharps are about 2.8 spaces tall at full size: two closer than 6 steps
/// (3 spaces) would touch, so the lower one moves to the next column left.
const ACCIDENTAL_CLEARANCE_STEPS: i8 = 6;
const LEDGER_THICKNESS_SPACES: f32 = 0.16;
const LEDGER_EXTENSION_SPACES: f32 = 0.4;
/// Least room between a barline and a downbeat voicing's sharps, and
/// between one voicing and the next, in pitch staff spaces.
const VOICING_CLEARANCE_SPACES: f32 = 0.3;

/// Voicings are drawn at this fraction of full size everywhere (cue-size
/// noteheads), so a stack never changes size from bar to bar. It is the
/// largest that lets one change per beat fit with a column of sharps: at
/// 16:9 a beat is (11.4 − 0.8 − 1.2) spaces ÷ 4 beats = 2.35 spaces; a
/// stack reaches 0.6 × (1.69 + 0.4) spaces = 1.25 spaces right of its
/// slash (notehead and ledger line), the next one's sharps
/// 0.6 × (0.25 + 1.0) spaces = 0.75 spaces left of its own, and with the
/// 0.3 spaces between them that makes 2.3 spaces.
const VOICING_SCALE: f32 = 0.6;
/// The bottom and top staff lines' steps: below the one and above the
/// other, every even step is a ledger line.
const BOTTOM_LINE_STEP: i8 = 0;
const TOP_LINE_STEP: i8 = 8;

/// Chord symbols: one size everywhere, in pitch staff spaces to the em.
/// Capitals stand 0.729 em × 1.3 spaces/em = 0.95 spaces tall.
const CHORD_SYMBOL_EM_SPACES: f32 = 1.3;
/// The two rows' baselines, in pitch staff spaces below the part's chord
/// baseline (negative is above). The lower row is the usual one, close to
/// the music; the upper row's baseline stands a capital's height plus
/// 0.8 spaces above it: 0.8 − (0.95 + 0.8) = −0.95 spaces.
const LOWER_ROW_BASELINE_SPACES: f32 = 0.8;
const UPPER_ROW_BASELINE_SPACES: f32 = -0.95;
/// Least room between one chord symbol and the next on a row, in pitch
/// staff spaces.
const CHORD_SYMBOL_GAP_SPACES: f32 = 0.5;

/// A rhythm value on the rhythm line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlashValue {
    Whole,
    Half,
    DottedQuarter,
    Quarter,
    Eighth,
}

/// One note or rest of the part's rhythm in a bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RhythmEvent {
    /// Where it starts in the bar, in eighth notes from the downbeat.
    start_eighths: u8,
    value: SlashValue,
    rest: bool,
}

const fn hit(start_eighths: u8, value: SlashValue) -> RhythmEvent {
    RhythmEvent {
        start_eighths,
        value,
        rest: false,
    }
}

const fn rest(start_eighths: u8, value: SlashValue) -> RhythmEvent {
    RhythmEvent {
        start_eighths,
        value,
        rest: true,
    }
}

/// The part's rhythm, bar by bar: quarters, eighths, halves, a Charleston,
/// quarters with rests, a whole, a push, and a mix (EFS-7.2's kinds).
const SPIKE_RHYTHM: [&[RhythmEvent]; BARS_IN_STRIP as usize] = [
    &[
        hit(0, SlashValue::Quarter),
        hit(2, SlashValue::Quarter),
        hit(4, SlashValue::Quarter),
        hit(6, SlashValue::Quarter),
    ],
    &[
        hit(0, SlashValue::Eighth),
        hit(1, SlashValue::Eighth),
        hit(2, SlashValue::Eighth),
        hit(3, SlashValue::Eighth),
        hit(4, SlashValue::Eighth),
        hit(5, SlashValue::Eighth),
        hit(6, SlashValue::Eighth),
        hit(7, SlashValue::Eighth),
    ],
    &[hit(0, SlashValue::Half), hit(4, SlashValue::Half)],
    &[
        hit(0, SlashValue::DottedQuarter),
        hit(3, SlashValue::Eighth),
        hit(4, SlashValue::Half),
    ],
    &[
        hit(0, SlashValue::Quarter),
        rest(2, SlashValue::Quarter),
        hit(4, SlashValue::Quarter),
        rest(6, SlashValue::Quarter),
    ],
    &[hit(0, SlashValue::Whole)],
    &[
        hit(0, SlashValue::Eighth),
        hit(1, SlashValue::Eighth),
        hit(2, SlashValue::Quarter),
        hit(4, SlashValue::Eighth),
        hit(5, SlashValue::Eighth),
        rest(6, SlashValue::Eighth),
        hit(7, SlashValue::Eighth),
    ],
    &[
        hit(0, SlashValue::Quarter),
        hit(2, SlashValue::Eighth),
        hit(3, SlashValue::Eighth),
        hit(4, SlashValue::Half),
    ],
];

/// One note of a voicing. `staff_step`: half staff spaces up from the
/// bottom line (E4 0, C4 −2, G5 9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VoicingNote {
    staff_step: i8,
    sharp: bool,
}

const fn tone(staff_step: i8) -> VoicingNote {
    VoicingNote {
        staff_step,
        sharp: false,
    }
}

const fn sharp(staff_step: i8) -> VoicingNote {
    VoicingNote {
        staff_step,
        sharp: true,
    }
}

/// A chord of the spike, with its place on the wheel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpikeChord {
    /// Named as EFS-1.5a sets out.
    pub symbol: &'static str,
    /// Its degree in C major; `None` outside the key.
    pub degree: Option<WheelDegree>,
}

const fn chord(symbol: &'static str, degree: Option<WheelDegree>) -> SpikeChord {
    SpikeChord { symbol, degree }
}

/// A chord change: where in its bar it falls, the chord, and its voicing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChordChange {
    /// In eighth notes from the bar's downbeat.
    start_eighths: u8,
    chord: SpikeChord,
    voicing: &'static [VoicingNote],
}

const fn change(
    start_eighths: u8,
    chord: SpikeChord,
    voicing: &'static [VoicingNote],
) -> ChordChange {
    ChordChange {
        start_eighths,
        chord,
        voicing,
    }
}

const C: SpikeChord = chord("C", Some(WheelDegree::One));
const CM7: SpikeChord = chord("CM7", Some(WheelDegree::One));
const AM7: SpikeChord = chord("Am7", Some(WheelDegree::Six));
const DM7: SpikeChord = chord("Dm7", Some(WheelDegree::Two));
const G7: SpikeChord = chord("G7", Some(WheelDegree::Five));
const EM: SpikeChord = chord("Em", Some(WheelDegree::Three));
const F_SHARP_M7_FLAT_5: SpikeChord = chord("F♯m7♭5", None);
const B7: SpikeChord = chord("B7", None);
const A7: SpikeChord = chord("A7", None);

/// Close-position voicings.
const C_VOICING: &[VoicingNote] = &[tone(-2), tone(0), tone(2)]; // C4 E4 G4
const CM7_VOICING: &[VoicingNote] = &[tone(-2), tone(0), tone(2), tone(4)]; // C4 E4 G4 B4
const AM7_VOICING: &[VoicingNote] = &[tone(3), tone(5), tone(7), tone(9)]; // A4 C5 E5 G5
const DM7_VOICING: &[VoicingNote] = &[tone(-1), tone(1), tone(3), tone(5)]; // D4 F4 A4 C5
const G7_VOICING: &[VoicingNote] = &[tone(2), tone(4), tone(6), tone(8)]; // G4 B4 D5 F5
const EM_VOICING: &[VoicingNote] = &[tone(0), tone(2), tone(4)]; // E4 G4 B4
const F_SHARP_M7_FLAT_5_VOICING: &[VoicingNote] = &[sharp(1), tone(3), tone(5), tone(7)]; // F♯4 A4 C5 E5
const B7_VOICING: &[VoicingNote] = &[tone(-3), sharp(-1), sharp(1), tone(3)]; // B3 D♯4 F♯4 A4
const A7_VOICING: &[VoicingNote] = &[tone(3), sharp(5), tone(7), tone(9)]; // A4 C♯5 E5 G5

/// The chord changes, bar by bar, in C major. Several bars change more than
/// once: one chord per beat (bar 0), two sharp-laden chords a half note
/// apart (bar 2), a change on the off-beat (bar 3), two per bar (bar 7).
/// Each change falls on a note of `SPIKE_RHYTHM`, so no note crosses one.
/// F♯m7♭5, B7 and A7 lie outside the key: the wheel names them in its
/// centre.
const SPIKE_CHANGES: [&[ChordChange]; BARS_IN_STRIP as usize] = [
    &[
        change(0, C, C_VOICING),
        change(2, AM7, AM7_VOICING),
        change(4, DM7, DM7_VOICING),
        change(6, G7, G7_VOICING),
    ],
    &[change(0, CM7, CM7_VOICING)],
    &[
        change(0, F_SHARP_M7_FLAT_5, F_SHARP_M7_FLAT_5_VOICING),
        change(4, B7, B7_VOICING),
    ],
    &[change(0, EM, EM_VOICING), change(3, A7, A7_VOICING)],
    &[change(0, DM7, DM7_VOICING)],
    &[change(0, G7, G7_VOICING)],
    &[change(0, C, C_VOICING)],
    &[change(0, DM7, DM7_VOICING), change(4, G7, G7_VOICING)],
];

/// The chord sounding `playhead_fraction` of the way through the 8 bars
/// (0.0 to 1.0): the last change at or before that point. `None` before
/// the first change, when no chord sounds.
#[must_use]
pub fn chord_at(playhead_fraction: f32) -> Option<SpikeChord> {
    let eighths_in_strip: u16 = BARS_IN_STRIP * u16::from(EIGHTHS_PER_BAR);
    // fraction × 64 eighths, rounded down to the eighth the playhead is
    // in; min keeps it on the last eighth at exactly 1.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let playhead_eighths: u16 = ((playhead_fraction.clamp(0.0, 1.0) * f32::from(eighths_in_strip))
        .floor() as u16)
        .min(eighths_in_strip - 1);
    let mut sounding: Option<SpikeChord> = None;
    for (bar, changes) in (0..BARS_IN_STRIP).zip(SPIKE_CHANGES) {
        for change in changes {
            let change_eighths: u16 =
                bar * u16::from(EIGHTHS_PER_BAR) + u16::from(change.start_eighths);
            if change_eighths <= playhead_eighths {
                sounding = Some(change.chord);
            }
        }
    }
    sounding
}

/// Appends the spike's music for the part: chord symbols, the rhythm line's
/// clef, time signature and slashes, and, when the pitch staff is shown,
/// its clef, time signature and voicings.
///
/// # Errors
///
/// When the atlas lacks a glyph the layout uses.
pub fn spike_music(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    lines: &mut Vec<LineInstance>,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    chord_symbols(geometry, atlas, glyphs)?;
    rhythm_line(geometry, atlas, lines, glyphs)?;
    if let Some(staff) = geometry.pitch_staff {
        pitch_staff(geometry, &staff, atlas, lines, glyphs)?;
    }
    Ok(())
}

/// Which row a chord symbol stands on: the lower, or, in a bar too crowded
/// for one row, the lower and upper taken in turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChordSymbolRow {
    Lower,
    Upper,
}

fn chord_symbols(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    for (bar, (changes, rows)) in (0..BARS_IN_STRIP).zip(
        SPIKE_CHANGES
            .iter()
            .zip(chord_symbol_rows(geometry, atlas)?),
    ) {
        for (change, row) in changes.iter().zip(rows) {
            chord_symbol(geometry, atlas, bar, change, row, glyphs)?;
        }
    }
    Ok(())
}

/// Each change's row, bar by bar. A bar whose symbols all fit on the lower
/// row, after whatever stands there already, uses it. A crowded bar
/// alternates, starting on the lower row where its first symbol fits
/// there; after a crowded bar it carries on the alternation.
fn chord_symbol_rows(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
) -> Result<Vec<Vec<ChordSymbolRow>>, RenderError> {
    let gap_pixels: f32 = CHORD_SYMBOL_GAP_SPACES * geometry.space_pixels;
    let mut rows: Vec<Vec<ChordSymbolRow>> = Vec::new();
    // The right edge of the last symbol on the lower row so far.
    let mut lower_right_pixels: f32 = f32::NEG_INFINITY;
    // The row of the last symbol of the bar before, if that bar alternated.
    let mut last_alternated_row: Option<ChordSymbolRow> = None;
    for (bar, changes) in (0..BARS_IN_STRIP).zip(SPIKE_CHANGES) {
        let mut extents_pixels: Vec<(f32, f32)> = Vec::new();
        for change in changes {
            extents_pixels.push(chord_symbol_extent_pixels(geometry, atlas, bar, change)?);
        }
        let mut right_pixels: f32 = lower_right_pixels;
        let mut fits_on_lower_row: bool = true;
        for (left_pixels, symbol_right_pixels) in &extents_pixels {
            fits_on_lower_row &= *left_pixels >= right_pixels + gap_pixels;
            right_pixels = *symbol_right_pixels;
        }
        if fits_on_lower_row {
            rows.push(vec![ChordSymbolRow::Lower; changes.len()]);
            lower_right_pixels = right_pixels;
            last_alternated_row = None;
            continue;
        }
        let first_fits_on_lower_row: bool = extents_pixels
            .first()
            .is_some_and(|(left_pixels, _)| *left_pixels >= lower_right_pixels + gap_pixels);
        let mut row: ChordSymbolRow = match last_alternated_row {
            Some(ChordSymbolRow::Upper) => ChordSymbolRow::Lower,
            None if first_fits_on_lower_row => ChordSymbolRow::Lower,
            Some(ChordSymbolRow::Lower) | None => ChordSymbolRow::Upper,
        };
        let mut bar_rows: Vec<ChordSymbolRow> = Vec::new();
        for (_left_pixels, symbol_right_pixels) in &extents_pixels {
            bar_rows.push(row);
            if row == ChordSymbolRow::Lower {
                lower_right_pixels = *symbol_right_pixels;
            }
            last_alternated_row = Some(row);
            row = match row {
                ChordSymbolRow::Lower => ChordSymbolRow::Upper,
                ChordSymbolRow::Upper => ChordSymbolRow::Lower,
            };
        }
        rows.push(bar_rows);
    }
    Ok(rows)
}

/// A chord symbol's left and right ink edges: it starts on its note's
/// stem; its width is its advances.
fn chord_symbol_extent_pixels(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    bar: u16,
    change: &ChordChange,
) -> Result<(f32, f32), RenderError> {
    let left_pixels: f32 = stem_left_x_pixels(geometry, bar, change.start_eighths);
    let width_pixels: f32 = atlas.text_width_pixels(
        GlyphFont::DejaVuSans,
        change.chord.symbol,
        CHORD_SYMBOL_EM_SPACES * geometry.space_pixels,
    )?;
    Ok((left_pixels, left_pixels + width_pixels))
}

/// One change's chord symbol, its first letter's ink starting on the left
/// edge of its note's stem (EFS-1.5).
fn chord_symbol(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    bar: u16,
    change: &ChordChange,
    row: ChordSymbolRow,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let em_pixels: f32 = CHORD_SYMBOL_EM_SPACES * geometry.space_pixels;
    let baseline_offset_spaces: f32 = match row {
        ChordSymbolRow::Lower => LOWER_ROW_BASELINE_SPACES,
        ChordSymbolRow::Upper => UPPER_ROW_BASELINE_SPACES,
    };
    // The ink starts the field's border plus the field's offset right of
    // the origin.
    let ink_offset_pixels: f32 = match change.chord.symbol.chars().next() {
        Some(first) => {
            atlas.glyph(GlyphFont::DejaVuSans, first)?.left_ems * em_pixels
                + atlas.spread_pixels * em_pixels / atlas.pixels_per_em
        }
        None => 0.0, // an empty symbol draws nothing
    };
    atlas.text(
        GlyphFont::DejaVuSans,
        change.chord.symbol,
        stem_left_x_pixels(geometry, bar, change.start_eighths) - ink_offset_pixels,
        geometry.chord_baseline_y_pixels + baseline_offset_spaces * geometry.space_pixels,
        em_pixels,
        theme::TEXT,
        glyphs,
    )
}

/// The left edge of the stem of the note starting `start_eighths` into bar
/// `bar`. A note without a stem (a whole note), a rest, or a change
/// between notes takes the place a black slash's stem would have there.
fn stem_left_x_pixels(geometry: &StripGeometry, bar: u16, start_eighths: u8) -> f32 {
    let value: Option<SlashValue> = SPIKE_RHYTHM[usize::from(bar)]
        .iter()
        .find(|event| event.start_eighths == start_eighths && !event.rest)
        .map(|event| event.value);
    let anchor_x_rhythm_spaces: f32 = if value == Some(SlashValue::Half) {
        SLASH_HALF_STEM_X_RHYTHM_SPACES
    } else {
        SLASH_BLACK_STEM_X_RHYTHM_SPACES
    };
    let stem_thickness_pixels: f32 =
        whole_thickness_pixels(geometry.space_pixels * STEM_THICKNESS_SPACES);
    stem_center_x_pixels(
        geometry,
        time_x_pixels(geometry, bar, start_eighths),
        anchor_x_rhythm_spaces,
    ) - stem_thickness_pixels / 2.0
}

/// A stem's centre for a slash at `slash_x_pixels`: its right edge on the
/// slash's stem anchor, snapped to whole pixels.
fn stem_center_x_pixels(
    geometry: &StripGeometry,
    slash_x_pixels: f32,
    anchor_x_rhythm_spaces: f32,
) -> f32 {
    let stem_thickness_pixels: f32 =
        whole_thickness_pixels(geometry.space_pixels * STEM_THICKNESS_SPACES);
    snapped_center_pixels(
        slash_x_pixels + anchor_x_rhythm_spaces * geometry.rhythm_space_pixels
            - stem_thickness_pixels / 2.0,
        stem_thickness_pixels,
    )
}

/// Where a slash starting `start_eighths` into bar `bar` stands: after the
/// lead-in, spaced in proportion to time, start ÷ 8 eighths per bar.
fn time_x_pixels(geometry: &StripGeometry, bar: u16, start_eighths: u8) -> f32 {
    let lead_in_pixels: f32 = RHYTHM_LEAD_IN_SPACES * geometry.space_pixels;
    let usable_pixels: f32 =
        geometry.bar_width_pixels - lead_in_pixels - RHYTHM_TRAIL_SPACES * geometry.space_pixels;
    geometry.bar_left_pixels(bar)
        + lead_in_pixels
        + usable_pixels * f32::from(start_eighths) / f32::from(EIGHTHS_PER_BAR)
}

/// Places one Bravura glyph with its origin at the point, at `em_pixels`
/// to the em.
fn place_music(
    atlas: &GlyphAtlas,
    codepoint: char,
    origin: (f32, f32),
    em_pixels: f32,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let (x_pixels, y_pixels): (f32, f32) = origin;
    if let Some(glyph) = atlas.place(
        GlyphFont::Bravura,
        codepoint,
        x_pixels,
        y_pixels,
        em_pixels,
        theme::NOTE,
    )? {
        glyphs.push(glyph);
    }
    Ok(())
}

fn advance_pixels(atlas: &GlyphAtlas, codepoint: char, em_pixels: f32) -> Result<f32, RenderError> {
    Ok(atlas.glyph(GlyphFont::Bravura, codepoint)?.advance_ems * em_pixels)
}

fn rhythm_line(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    lines: &mut Vec<LineInstance>,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let line_y_pixels: f32 = geometry.rhythm_line_y_pixels;
    let space_pixels: f32 = geometry.rhythm_space_pixels;
    let em_pixels: f32 = geometry.rhythm_em_pixels();

    // The percussion clef is centred on the line; the time signature's
    // digits are centred one space above and below it.
    place_music(
        atlas,
        PERCUSSION_CLEF,
        (
            geometry.staff_left_pixels + CLEF_X_SPACES * geometry.space_pixels,
            line_y_pixels,
        ),
        em_pixels,
        glyphs,
    )?;
    let time_x_pixels: f32 =
        geometry.staff_left_pixels + TIME_SIGNATURE_X_SPACES * geometry.space_pixels;
    for digit_y_pixels in [line_y_pixels - space_pixels, line_y_pixels + space_pixels] {
        place_music(
            atlas,
            TIME_SIGNATURE_4,
            (time_x_pixels, digit_y_pixels),
            em_pixels,
            glyphs,
        )?;
    }

    for (bar, events) in (0..BARS_IN_STRIP).zip(SPIKE_RHYTHM) {
        rhythm_bar(geometry, atlas, bar, events, lines, glyphs)?;
    }
    Ok(())
}

/// One bar of the rhythm line: slashes with their stems, dots and flags,
/// rests, and the beams joining eighth notes within a beat.
fn rhythm_bar(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    bar: u16,
    events: &[RhythmEvent],
    lines: &mut Vec<LineInstance>,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let line_y_pixels: f32 = geometry.rhythm_line_y_pixels;
    let space_pixels: f32 = geometry.rhythm_space_pixels;
    let em_pixels: f32 = geometry.rhythm_em_pixels();
    let stem_thickness_pixels: f32 =
        whole_thickness_pixels(geometry.space_pixels * STEM_THICKNESS_SPACES);
    let stem_top_y_pixels: f32 = line_y_pixels - SLASH_STEM_TOP_RHYTHM_SPACES * space_pixels;
    let bar_left_pixels: f32 = geometry.bar_left_pixels(bar);
    // Each stem's centre, by event, so beams can join them.
    let mut stem_xs_pixels: Vec<Option<f32>> = vec![None; events.len()];
    for (index, event) in events.iter().enumerate() {
        let x_pixels: f32 = time_x_pixels(geometry, bar, event.start_eighths);
        if event.rest {
            rhythm_rest(geometry, atlas, *event, bar_left_pixels, x_pixels, glyphs)?;
            continue;
        }
        let (head, stem_anchor_spaces): (char, Option<(f32, f32)>) = match event.value {
            SlashValue::Whole => (SLASH_WHITE_WHOLE, None),
            SlashValue::Half => (
                SLASH_WHITE_HALF,
                Some((
                    SLASH_HALF_STEM_X_RHYTHM_SPACES,
                    SLASH_HALF_STEM_Y_RHYTHM_SPACES,
                )),
            ),
            SlashValue::DottedQuarter | SlashValue::Quarter | SlashValue::Eighth => (
                SLASH_BLACK,
                Some((
                    SLASH_BLACK_STEM_X_RHYTHM_SPACES,
                    SLASH_BLACK_STEM_Y_RHYTHM_SPACES,
                )),
            ),
        };
        place_music(atlas, head, (x_pixels, line_y_pixels), em_pixels, glyphs)?;
        if event.value == SlashValue::DottedQuarter {
            let dot_x_pixels: f32 = x_pixels
                + advance_pixels(atlas, head, em_pixels)?
                + DOT_GAP_RHYTHM_SPACES * space_pixels;
            place_music(
                atlas,
                AUGMENTATION_DOT,
                (
                    dot_x_pixels,
                    line_y_pixels - DOT_RAISE_RHYTHM_SPACES * space_pixels,
                ),
                em_pixels,
                glyphs,
            )?;
        }
        let Some((anchor_x_spaces, anchor_y_spaces)) = stem_anchor_spaces else {
            continue;
        };
        // y runs down on screen.
        let stem_x_pixels: f32 = stem_center_x_pixels(geometry, x_pixels, anchor_x_spaces);
        lines.push(vertical_line(
            stem_x_pixels,
            stem_top_y_pixels,
            line_y_pixels - anchor_y_spaces * space_pixels,
            stem_thickness_pixels,
            theme::NOTE,
        ));
        stem_xs_pixels[index] = Some(stem_x_pixels);
        if event.value == SlashValue::Eighth && !is_beamed(events, index) {
            place_music(
                atlas,
                FLAG_EIGHTH_UP,
                (
                    stem_x_pixels - stem_thickness_pixels / 2.0,
                    stem_top_y_pixels - FLAG_STEM_Y_RHYTHM_SPACES * space_pixels,
                ),
                em_pixels,
                glyphs,
            )?;
        }
    }
    beams(
        events,
        &stem_xs_pixels,
        stem_top_y_pixels,
        stem_thickness_pixels,
        BEAM_THICKNESS_RHYTHM_SPACES * space_pixels,
        lines,
    );
    Ok(())
}

/// A beam joins each beamed pair of eighths, hanging from the stems' tops.
fn beams(
    events: &[RhythmEvent],
    stem_xs_pixels: &[Option<f32>],
    stem_top_y_pixels: f32,
    stem_thickness_pixels: f32,
    beam_thickness_pixels: f32,
    lines: &mut Vec<LineInstance>,
) {
    let beam_y_pixels: f32 = stem_top_y_pixels + beam_thickness_pixels / 2.0;
    for index in 0..events.len() {
        if !starts_beam(events, index) {
            continue;
        }
        if let (Some(Some(first_x_pixels)), Some(Some(second_x_pixels))) =
            (stem_xs_pixels.get(index), stem_xs_pixels.get(index + 1))
        {
            lines.push(LineInstance {
                start_x_pixels: first_x_pixels - stem_thickness_pixels / 2.0,
                start_y_pixels: beam_y_pixels,
                end_x_pixels: second_x_pixels + stem_thickness_pixels / 2.0,
                end_y_pixels: beam_y_pixels,
                thickness_pixels: beam_thickness_pixels,
                color_linear_rgba: theme::NOTE,
            });
        }
    }
}

/// Whether event `index` is an eighth note on a beat that the next event, an
/// eighth note on the off-beat, shares: the two are beamed.
fn starts_beam(events: &[RhythmEvent], index: usize) -> bool {
    let is_eighth_note = |event: &RhythmEvent| event.value == SlashValue::Eighth && !event.rest;
    match (events.get(index), events.get(index + 1)) {
        (Some(first), Some(second)) => {
            is_eighth_note(first)
                && is_eighth_note(second)
                && first.start_eighths % EIGHTHS_PER_BEAT == 0
                && second.start_eighths == first.start_eighths + 1
        }
        _ => false,
    }
}

fn is_beamed(events: &[RhythmEvent], index: usize) -> bool {
    starts_beam(events, index) || (index > 0 && starts_beam(events, index - 1))
}

/// A rest on the rhythm line: quarter and eighth rests centred on the line,
/// the half rest sitting on it, the whole rest hanging from it in the
/// bar's middle. A dotted value's rest takes a dot.
fn rhythm_rest(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    event: RhythmEvent,
    bar_left_pixels: f32,
    x_pixels: f32,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let em_pixels: f32 = geometry.rhythm_em_pixels();
    let line_y_pixels: f32 = geometry.rhythm_line_y_pixels;
    let glyph: char = match event.value {
        SlashValue::Whole => REST_WHOLE,
        SlashValue::Half => REST_HALF,
        SlashValue::DottedQuarter | SlashValue::Quarter => REST_QUARTER,
        SlashValue::Eighth => REST_EIGHTH,
    };
    let x_pixels: f32 = if event.value == SlashValue::Whole {
        bar_left_pixels + geometry.bar_width_pixels / 2.0
            - advance_pixels(atlas, glyph, em_pixels)? / 2.0
    } else {
        x_pixels
    };
    place_music(atlas, glyph, (x_pixels, line_y_pixels), em_pixels, glyphs)?;
    if event.value == SlashValue::DottedQuarter {
        let dot_x_pixels: f32 = x_pixels
            + advance_pixels(atlas, glyph, em_pixels)?
            + DOT_GAP_RHYTHM_SPACES * geometry.rhythm_space_pixels;
        place_music(
            atlas,
            AUGMENTATION_DOT,
            (
                dot_x_pixels,
                line_y_pixels - DOT_RAISE_RHYTHM_SPACES * geometry.rhythm_space_pixels,
            ),
            em_pixels,
            glyphs,
        )?;
    }
    Ok(())
}

fn pitch_staff(
    geometry: &StripGeometry,
    staff: &PitchStaffPlace,
    atlas: &GlyphAtlas,
    lines: &mut Vec<LineInstance>,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let space_pixels: f32 = staff.space_pixels;
    let em_pixels: f32 = geometry.music_em_pixels();
    // The G clef's origin is on the G line (step 2); time signature digits
    // are centred on the second and fourth lines.
    place_music(
        atlas,
        G_CLEF,
        (
            geometry.staff_left_pixels + CLEF_X_SPACES * space_pixels,
            staff.step_y_pixels(2),
        ),
        em_pixels,
        glyphs,
    )?;
    let time_x_pixels: f32 = geometry.staff_left_pixels + TIME_SIGNATURE_X_SPACES * space_pixels;
    for digit_step in [6, 2] {
        place_music(
            atlas,
            TIME_SIGNATURE_4,
            (time_x_pixels, staff.step_y_pixels(digit_step)),
            em_pixels,
            glyphs,
        )?;
    }

    for (bar, changes) in (0..BARS_IN_STRIP).zip(SPIKE_CHANGES) {
        for change in changes {
            let place: VoicingPlace = voicing_place(geometry, atlas, bar, change)?;
            voicing(geometry, staff, atlas, &place, lines, glyphs)?;
        }
    }
    Ok(())
}

/// Where one voicing goes: its notes in order, their sharps' columns, and
/// its noteheads' left edge.
#[derive(Clone, Debug, PartialEq)]
struct VoicingPlace {
    /// Highest first.
    notes: Vec<VoicingNote>,
    /// Each note's sharp column, nearest the noteheads 0; `None` without a
    /// sharp.
    sharp_columns: Vec<Option<usize>>,
    head_x_pixels: f32,
}

/// A voicing's noteheads stand under their note's slash, sharps to the
/// left. On the downbeat, sharps that would come too close to the barline
/// push the noteheads right.
fn voicing_place(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    bar: u16,
    change: &ChordChange,
) -> Result<VoicingPlace, RenderError> {
    let scale: f32 = VOICING_SCALE;
    let space_pixels: f32 = geometry.space_pixels;
    let em_pixels: f32 = scale * geometry.music_em_pixels();
    let mut notes: Vec<VoicingNote> = change.voicing.to_vec();
    notes.sort_by_key(|note| std::cmp::Reverse(note.staff_step));
    // Accidental columns, nearest the noteheads first: the lowest step
    // placed so far in each. Smaller sharps may stand closer.
    let clearance_steps: f32 = f32::from(ACCIDENTAL_CLEARANCE_STEPS) * scale;
    let mut column_lowest_steps: Vec<i8> = Vec::new();
    let mut sharp_columns: Vec<Option<usize>> = Vec::new();
    for note in &notes {
        if !note.sharp {
            sharp_columns.push(None);
            continue;
        }
        let column: usize = if let Some(column) = column_lowest_steps
            .iter()
            .position(|lowest| f32::from(lowest - note.staff_step) >= clearance_steps)
        {
            column_lowest_steps[column] = note.staff_step;
            column
        } else {
            column_lowest_steps.push(note.staff_step);
            column_lowest_steps.len() - 1
        };
        sharp_columns.push(Some(column));
    }

    // How far the voicing reaches left of its noteheads: its sharps (gap,
    // columns, gaps between columns) or its ledger lines.
    let ledger_extension_pixels: f32 = if ledger_steps(&notes).is_empty() {
        0.0
    } else {
        scale * LEDGER_EXTENSION_SPACES * space_pixels
    };
    // A chord has a handful of columns at most, so this is exact.
    #[allow(clippy::cast_precision_loss)]
    let columns: f32 = column_lowest_steps.len() as f32;
    let sharps_reach_pixels: f32 = if columns > 0.0 {
        scale * ACCIDENTAL_GAP_SPACES * space_pixels
            + columns * advance_pixels(atlas, ACCIDENTAL_SHARP, em_pixels)?
            + (columns - 1.0) * scale * ACCIDENTAL_COLUMN_GAP_SPACES * space_pixels
    } else {
        0.0
    };
    let left_reach_pixels: f32 = sharps_reach_pixels.max(ledger_extension_pixels);

    let slash_x_pixels: f32 = time_x_pixels(geometry, bar, change.start_eighths);
    let head_x_pixels: f32 = if change.start_eighths == 0 {
        slash_x_pixels.max(
            geometry.bar_left_pixels(bar)
                + VOICING_CLEARANCE_SPACES * space_pixels
                + left_reach_pixels,
        )
    } else {
        slash_x_pixels
    };
    Ok(VoicingPlace {
        notes,
        sharp_columns,
        head_x_pixels,
    })
}

/// One voicing: stacked noteheads, only at the change (EFS-1.5).
fn voicing(
    geometry: &StripGeometry,
    staff: &PitchStaffPlace,
    atlas: &GlyphAtlas,
    place: &VoicingPlace,
    lines: &mut Vec<LineInstance>,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let space_pixels: f32 = staff.space_pixels;
    let em_pixels: f32 = VOICING_SCALE * geometry.music_em_pixels();
    let head_x_pixels: f32 = place.head_x_pixels;
    let sharp_width_pixels: f32 = advance_pixels(atlas, ACCIDENTAL_SHARP, em_pixels)?;
    for (note, sharp_column) in place.notes.iter().zip(&place.sharp_columns) {
        let y_pixels: f32 = staff.step_y_pixels(note.staff_step);
        place_music(
            atlas,
            NOTEHEAD_WHOLE,
            (head_x_pixels, y_pixels),
            em_pixels,
            glyphs,
        )?;
        let Some(column) = sharp_column else {
            continue;
        };
        // A chord has a handful of columns at most, so this is exact.
        #[allow(clippy::cast_precision_loss)]
        let column_number: f32 = *column as f32;
        let x_pixels: f32 = head_x_pixels
            - VOICING_SCALE * ACCIDENTAL_GAP_SPACES * space_pixels
            - (column_number + 1.0) * sharp_width_pixels
            - column_number * VOICING_SCALE * ACCIDENTAL_COLUMN_GAP_SPACES * space_pixels;
        place_music(
            atlas,
            ACCIDENTAL_SHARP,
            (x_pixels, y_pixels),
            em_pixels,
            glyphs,
        )?;
    }
    // Ledger lines for notes beyond the staff, once per chord; as thick as
    // at full size, since they continue the staff.
    let ledger_thickness_pixels: f32 =
        whole_thickness_pixels(LEDGER_THICKNESS_SPACES * space_pixels);
    let ledger_extension_pixels: f32 = VOICING_SCALE * LEDGER_EXTENSION_SPACES * space_pixels;
    let ledger_left_pixels: f32 = head_x_pixels - ledger_extension_pixels;
    let ledger_right_pixels: f32 =
        head_x_pixels + advance_pixels(atlas, NOTEHEAD_WHOLE, em_pixels)? + ledger_extension_pixels;
    for step in ledger_steps(&place.notes) {
        lines.push(horizontal_line(
            ledger_left_pixels,
            ledger_right_pixels,
            staff.step_y_pixels(step),
            ledger_thickness_pixels,
            theme::NOTE,
        ));
    }
    Ok(())
}

/// The ledger lines a chord needs: every line step beyond the staff, out to
/// its lowest and highest notes.
fn ledger_steps(notes: &[VoicingNote]) -> Vec<i8> {
    let mut steps: Vec<i8> = Vec::new();
    for note in notes {
        let mut step: i8 = BOTTOM_LINE_STEP - 2;
        while step >= note.staff_step {
            steps.push(step);
            step -= 2;
        }
        let mut step: i8 = TOP_LINE_STEP + 2;
        while step <= note.staff_step {
            steps.push(step);
            step += 2;
        }
    }
    // Notes of one chord share their ledger lines.
    steps.sort_unstable();
    steps.dedup();
    steps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip_layout::ScreenRect;

    fn laid_out(shown: bool, width: f32, height: f32) -> (Vec<LineInstance>, Vec<GlyphInstance>) {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        let geometry: StripGeometry = StripGeometry::new(width, height, shown);
        let mut lines: Vec<LineInstance> = Vec::new();
        let mut glyphs: Vec<GlyphInstance> = Vec::new();
        spike_music(&geometry, &atlas, &mut lines, &mut glyphs).unwrap();
        (lines, glyphs)
    }

    #[test]
    fn every_symbol_is_laid_out() {
        let (lines, glyphs) = laid_out(true, 2560.0, 1440.0);
        // Rhythm line: clef 1, time 2; pitch staff: clef 1, time 2; chord
        // symbol characters 35 (14 changes); on the rhythm line 36
        // (slashes, rests, a dot and flags, by bar 4, 8, 2, 5, 4, 1, 8, 4);
        // voicing noteheads 53 and sharps 4.
        assert_eq!(glyphs.len(), 3 + 3 + 35 + 36 + 53 + 4);
        // Stems 29, beams 7, ledger lines 4 (C4 in bars 0, 1 and 6, and the
        // C4 line under B3 in bar 2).
        assert_eq!(lines.len(), 29 + 7 + 4);
    }

    #[test]
    fn hiding_the_staff_leaves_the_symbols_and_rhythm_line() {
        let (lines, glyphs) = laid_out(false, 2560.0, 1440.0);
        assert_eq!(glyphs.len(), 3 + 35 + 36);
        assert_eq!(lines.len(), 29 + 7);
    }

    #[test]
    fn the_wheel_follows_changes_inside_a_bar() {
        // Bar 0 changes every beat: 64 eighths in the strip, so beat 2
        // (eighth 2) is at 2 ÷ 64.
        let symbol_at = |eighths: f32| chord_at(eighths / 64.0).map(|chord| chord.symbol);
        assert_eq!(symbol_at(0.0), Some("C"));
        assert_eq!(symbol_at(1.9), Some("C"));
        assert_eq!(symbol_at(2.0), Some("Am7"));
        assert_eq!(symbol_at(7.5), Some("G7"));
        // Bar 3 changes on the off-beat after beat 2: eighth 24 + 3.
        assert_eq!(symbol_at(26.5), Some("Em"));
        assert_eq!(symbol_at(27.0), Some("A7"));
        assert_eq!(symbol_at(64.0), Some("G7"));
    }

    /// The ink's box, in pixels, of glyphs laid out at `em_pixels` to the
    /// em: a glyph's rectangle includes the field's empty border.
    fn ink_box(atlas: &GlyphAtlas, em_pixels: f32, glyphs: &[GlyphInstance]) -> ScreenRect {
        let border_pixels: f32 = atlas.spread_pixels * em_pixels / atlas.pixels_per_em;
        let mut ink: ScreenRect = ScreenRect {
            left_x_pixels: f32::INFINITY,
            right_x_pixels: f32::NEG_INFINITY,
            top_y_pixels: f32::INFINITY,
            bottom_y_pixels: f32::NEG_INFINITY,
        };
        for glyph in glyphs {
            ink.left_x_pixels = ink.left_x_pixels.min(glyph.left_x_pixels + border_pixels);
            ink.right_x_pixels = ink
                .right_x_pixels
                .max(glyph.left_x_pixels + glyph.width_pixels - border_pixels);
            ink.top_y_pixels = ink.top_y_pixels.min(glyph.top_y_pixels + border_pixels);
            ink.bottom_y_pixels = ink
                .bottom_y_pixels
                .max(glyph.top_y_pixels + glyph.height_pixels - border_pixels);
        }
        ink
    }

    const SIZES: [(f32, f32); 3] = [(2560.0, 1440.0), (1280.0, 720.0), (3840.0, 2160.0)];

    #[test]
    fn a_crowded_bar_alternates_rows_starting_low_and_others_use_the_lower_row() {
        use ChordSymbolRow::{Lower, Upper};
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        for (width, height) in SIZES {
            let geometry: StripGeometry = StripGeometry::new(width, height, true);
            let rows: Vec<Vec<ChordSymbolRow>> = chord_symbol_rows(&geometry, &atlas).unwrap();
            // Bar 0 changes every beat; in bar 2 F♯m7♭5 is too wide to
            // share the row with B7. Every other bar fits on the lower row,
            // bar 3 too: B7 above leaves room for its Em below.
            assert_eq!(rows[0], vec![Lower, Upper, Lower, Upper]);
            assert_eq!(rows[2], vec![Lower, Upper]);
            for bar in [1, 3, 4, 5, 6, 7] {
                assert!(rows[bar].iter().all(|row| *row == Lower), "{rows:?}");
            }
        }
    }

    #[test]
    fn a_chord_symbol_starts_on_its_notes_stem() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0, true);
        let em_pixels: f32 = CHORD_SYMBOL_EM_SPACES * geometry.space_pixels;
        // Bar 0, beat 2 (a quarter's stem); bar 2, beat 3 (a half's stem).
        for (bar, index) in [(0_u16, 1_usize), (2, 1)] {
            let change: &ChordChange = &SPIKE_CHANGES[usize::from(bar)][index];
            let mut glyphs: Vec<GlyphInstance> = Vec::new();
            chord_symbol(
                &geometry,
                &atlas,
                bar,
                change,
                ChordSymbolRow::Lower,
                &mut glyphs,
            )
            .unwrap();
            let ink: ScreenRect = ink_box(&atlas, em_pixels, &glyphs[..1]);
            let stem_left_pixels: f32 = stem_left_x_pixels(&geometry, bar, change.start_eighths);
            assert!(
                (ink.left_x_pixels - stem_left_pixels).abs() < 0.01,
                "bar {bar}: ink at {} px, stem at {stem_left_pixels} px",
                ink.left_x_pixels
            );
        }
    }

    /// No two chord symbols touch, anywhere in the strip, and the rows stay
    /// inside the part's area above the rhythm line's stems. Every collision
    /// is listed, with the overlap in staff spaces.
    #[test]
    fn chord_symbols_never_collide() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        let mut collisions: Vec<String> = Vec::new();
        for (width, height) in SIZES {
            let geometry: StripGeometry = StripGeometry::new(width, height, true);
            let space_pixels: f32 = geometry.space_pixels;
            let rows: Vec<Vec<ChordSymbolRow>> = chord_symbol_rows(&geometry, &atlas).unwrap();
            let mut boxes: Vec<(String, ScreenRect)> = Vec::new();
            for (bar, (changes, bar_rows)) in
                (0..BARS_IN_STRIP).zip(SPIKE_CHANGES.iter().zip(&rows))
            {
                for (change, row) in changes.iter().zip(bar_rows) {
                    let mut glyphs: Vec<GlyphInstance> = Vec::new();
                    chord_symbol(&geometry, &atlas, bar, change, *row, &mut glyphs).unwrap();
                    let ink: ScreenRect =
                        ink_box(&atlas, CHORD_SYMBOL_EM_SPACES * space_pixels, &glyphs);
                    let name: String = format!("bar {bar} {}", change.chord.symbol);
                    let stems_top_pixels: f32 = geometry.rhythm_line_y_pixels
                        - SLASH_STEM_TOP_RHYTHM_SPACES * geometry.rhythm_space_pixels;
                    if ink.top_y_pixels < geometry.area_top_y_pixels
                        || ink.bottom_y_pixels > stems_top_pixels
                    {
                        collisions.push(format!("{width} × {height}: {name} leaves its row"));
                    }
                    boxes.push((name, ink));
                }
            }
            for (index, (name, ink)) in boxes.iter().enumerate() {
                for (other_name, other) in &boxes[index + 1..] {
                    let across_spaces: f32 = (ink.right_x_pixels.min(other.right_x_pixels)
                        - ink.left_x_pixels.max(other.left_x_pixels))
                        / space_pixels;
                    let down_spaces: f32 = (ink.bottom_y_pixels.min(other.bottom_y_pixels)
                        - ink.top_y_pixels.max(other.top_y_pixels))
                        / space_pixels;
                    if across_spaces > 0.0 && down_spaces > 0.0 {
                        collisions.push(format!(
                            "{width} × {height}: {name} and {other_name} overlap by \
                             {across_spaces:.2} spaces across"
                        ));
                    }
                }
            }
        }
        assert!(collisions.is_empty(), "\n{}", collisions.join("\n"));
    }

    /// Each voicing, with its sharps and ledger lines, clears the one before
    /// it and stays inside its bar: the spike's bars, and a bar of A7 on
    /// every beat, each with a sharp. Every collision is listed, with the
    /// overlap in staff spaces.
    #[test]
    fn voicings_never_collide() {
        const A7_EVERY_BEAT: &[ChordChange] = &[
            change(0, A7, A7_VOICING),
            change(2, A7, A7_VOICING),
            change(4, A7, A7_VOICING),
            change(6, A7, A7_VOICING),
        ];
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        let mut collisions: Vec<String> = Vec::new();
        for (width, height) in SIZES {
            let geometry: StripGeometry = StripGeometry::new(width, height, true);
            let staff: PitchStaffPlace = geometry.pitch_staff.unwrap();
            let space_pixels: f32 = geometry.space_pixels;
            let bars = (0..BARS_IN_STRIP)
                .zip(SPIKE_CHANGES)
                .chain(std::iter::once((0, A7_EVERY_BEAT)));
            for (bar, changes) in bars {
                // The barlines are the limits on either side.
                let mut extents: Vec<(f32, f32)> = vec![(0.0, geometry.bar_left_pixels(bar))];
                for change in changes {
                    let place: VoicingPlace =
                        voicing_place(&geometry, &atlas, bar, change).unwrap();
                    let mut lines: Vec<LineInstance> = Vec::new();
                    let mut glyphs: Vec<GlyphInstance> = Vec::new();
                    voicing(&geometry, &staff, &atlas, &place, &mut lines, &mut glyphs).unwrap();
                    let ink: ScreenRect =
                        ink_box(&atlas, VOICING_SCALE * geometry.music_em_pixels(), &glyphs);
                    let (mut left_pixels, mut right_pixels): (f32, f32) =
                        (ink.left_x_pixels, ink.right_x_pixels);
                    for line in &lines {
                        left_pixels = left_pixels.min(line.start_x_pixels);
                        right_pixels = right_pixels.max(line.end_x_pixels);
                    }
                    extents.push((left_pixels, right_pixels));
                }
                extents.push((geometry.bar_left_pixels(bar + 1), 0.0));
                for (index, pair) in extents.windows(2).enumerate() {
                    let overlap_spaces: f32 = (pair[0].1 - pair[1].0) / space_pixels;
                    if overlap_spaces > 0.0 {
                        collisions.push(format!(
                            "{width} × {height}, bar {bar}: between items {index} and {} \
                             (0 and last are barlines) by {overlap_spaces:.2} spaces",
                            index + 1
                        ));
                    }
                }
            }
        }
        assert!(collisions.is_empty(), "\n{}", collisions.join("\n"));
    }

    #[test]
    fn the_part_is_laid_out_inside_the_window() {
        for (width, height) in [(2560.0, 1440.0), (1280.0, 720.0), (3840.0, 2160.0)] {
            let (_lines, glyphs) = laid_out(true, width, height);
            for glyph in &glyphs {
                assert!(glyph.left_x_pixels >= 0.0 && glyph.top_y_pixels >= 0.0);
                assert!(glyph.left_x_pixels + glyph.width_pixels <= width);
                assert!(glyph.top_y_pixels + glyph.height_pixels <= height);
            }
        }
    }

    #[test]
    fn no_rhythm_symbol_crosses_a_barline() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        for (width, height) in [(2560.0, 1440.0), (1280.0, 720.0), (3840.0, 2160.0)] {
            let geometry: StripGeometry = StripGeometry::new(width, height, false);
            let (_lines, glyphs) = laid_out(false, width, height);
            // A glyph's rectangle includes the field's empty border; compare
            // the ink inside it.
            let border_pixels: f32 =
                atlas.spread_pixels * geometry.rhythm_em_pixels() / atlas.pixels_per_em;
            let rhythm_glyphs = glyphs
                .iter()
                .filter(|glyph| glyph.top_y_pixels > geometry.chord_baseline_y_pixels);
            for glyph in rhythm_glyphs {
                let ink_left_pixels: f32 = glyph.left_x_pixels + border_pixels;
                let ink_right_pixels: f32 =
                    glyph.left_x_pixels + glyph.width_pixels - border_pixels;
                for bar in 0..=BARS_IN_STRIP {
                    let barline_x_pixels: f32 = geometry.bar_left_pixels(bar);
                    assert!(
                        !(ink_left_pixels..=ink_right_pixels).contains(&barline_x_pixels),
                        "a symbol at {ink_left_pixels} to {ink_right_pixels} px crosses \
                         barline {bar} at {barline_x_pixels} px ({width} × {height})"
                    );
                }
            }
        }
    }

    #[test]
    fn eighths_on_a_beat_are_beamed_and_a_lone_eighth_is_flagged() {
        let bar_1: &[RhythmEvent] = SPIKE_RHYTHM[1];
        assert!(starts_beam(bar_1, 0) && !starts_beam(bar_1, 1));
        assert!(is_beamed(bar_1, 1));
        // Bar 3: the eighth on the off-beat after a dotted quarter.
        assert!(!is_beamed(SPIKE_RHYTHM[3], 1));
        // Bar 6: a rest on the beat leaves the off-beat eighth alone.
        assert!(!is_beamed(SPIKE_RHYTHM[6], 6));
    }

    #[test]
    fn ledger_lines_reach_out_to_the_note() {
        assert_eq!(ledger_steps(&[tone(-3), tone(3)]), vec![-2]);
        assert_eq!(ledger_steps(&[tone(-4), tone(-2)]), vec![-4, -2]);
        assert_eq!(ledger_steps(&[tone(11)]), vec![10]);
        assert_eq!(ledger_steps(&[tone(-1), tone(9)]), Vec::<i8>::new());
    }
}
