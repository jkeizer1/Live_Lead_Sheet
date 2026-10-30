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

/// Pitch staff, in pitch staff spaces: where a voicing's noteheads go after
/// the barline (room for two columns of sharps), the sharps' gaps, and
/// ledger lines (Bravura legerLineThickness, legerLineExtension).
const VOICING_HEAD_X_SPACES: f32 = 3.2;
const ACCIDENTAL_GAP_SPACES: f32 = 0.25;
const ACCIDENTAL_COLUMN_GAP_SPACES: f32 = 0.15;
/// Sharps are about 2.8 spaces tall: two closer than 6 steps (3 spaces)
/// would touch, so the lower one moves to the next column left.
const ACCIDENTAL_CLEARANCE_STEPS: i8 = 6;
const LEDGER_THICKNESS_SPACES: f32 = 0.16;
const LEDGER_EXTENSION_SPACES: f32 = 0.4;
/// The bottom and top staff lines' steps: below the one and above the
/// other, every even step is a ledger line.
const BOTTOM_LINE_STEP: i8 = 0;
const TOP_LINE_STEP: i8 = 8;

/// Chord symbols: size, in pitch staff spaces, and inset from the barline.
const CHORD_SYMBOL_EM_SPACES: f32 = 2.4;
const CHORD_SYMBOL_INSET_SPACES: f32 = 0.5;

/// A rhythm value on the rhythm line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlashValue {
    Whole,
    Half,
    DottedQuarter,
    Quarter,
    Eighth,
}

/// One pulse or rest of the part's rhythm in a bar.
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

/// One chord per bar, in C major. F♯m7♭5 and B7 lie outside the key: the
/// wheel names them in its centre.
pub const SPIKE_CHORDS: [SpikeChord; BARS_IN_STRIP as usize] = [
    chord("C", Some(WheelDegree::One)),
    chord("Am7", Some(WheelDegree::Six)),
    chord("Dm7", Some(WheelDegree::Two)),
    chord("G7", Some(WheelDegree::Five)),
    chord("CM7", Some(WheelDegree::One)),
    chord("F♯m7♭5", None),
    chord("B7", None),
    chord("Em", Some(WheelDegree::Three)),
];

/// Each chord voiced in close position.
const SPIKE_VOICINGS: [&[VoicingNote]; BARS_IN_STRIP as usize] = [
    &[tone(-2), tone(0), tone(2)],             // C4 E4 G4
    &[tone(3), tone(5), tone(7), tone(9)],     // A4 C5 E5 G5
    &[tone(-1), tone(1), tone(3), tone(5)],    // D4 F4 A4 C5
    &[tone(2), tone(4), tone(6), tone(8)],     // G4 B4 D5 F5
    &[tone(-2), tone(0), tone(2), tone(4)],    // C4 E4 G4 B4
    &[sharp(1), tone(3), tone(5), tone(7)],    // F♯4 A4 C5 E5
    &[tone(-3), sharp(-1), sharp(1), tone(3)], // B3 D♯4 F♯4 A4
    &[tone(0), tone(2), tone(4)],              // E4 G4 B4
];

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

fn chord_symbols(
    geometry: &StripGeometry,
    atlas: &GlyphAtlas,
    glyphs: &mut Vec<GlyphInstance>,
) -> Result<(), RenderError> {
    let em_pixels: f32 = CHORD_SYMBOL_EM_SPACES * geometry.space_pixels;
    for (bar, chord) in (0..BARS_IN_STRIP).zip(SPIKE_CHORDS) {
        atlas.text(
            GlyphFont::DejaVuSans,
            chord.symbol,
            geometry.bar_left_pixels(bar) + CHORD_SYMBOL_INSET_SPACES * geometry.space_pixels,
            geometry.chord_baseline_y_pixels,
            em_pixels,
            theme::TEXT,
            glyphs,
        )?;
    }
    Ok(())
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
    let lead_in_pixels: f32 = RHYTHM_LEAD_IN_SPACES * geometry.space_pixels;
    let bar_left_pixels: f32 = geometry.bar_left_pixels(bar);
    let usable_pixels: f32 =
        geometry.bar_width_pixels - lead_in_pixels - RHYTHM_TRAIL_SPACES * geometry.space_pixels;
    // Each stem's centre, by event, so beams can join them.
    let mut stem_xs_pixels: Vec<Option<f32>> = vec![None; events.len()];
    for (index, event) in events.iter().enumerate() {
        // Spaced in proportion to time: start ÷ 8 eighths per bar.
        let x_pixels: f32 = bar_left_pixels
            + lead_in_pixels
            + usable_pixels * f32::from(event.start_eighths) / f32::from(EIGHTHS_PER_BAR);
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
        // The stem's right edge on the anchor; y runs down on screen.
        let stem_x_pixels: f32 = snapped_center_pixels(
            x_pixels + anchor_x_spaces * space_pixels - stem_thickness_pixels / 2.0,
            stem_thickness_pixels,
        );
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

    let head_width_pixels: f32 = advance_pixels(atlas, NOTEHEAD_WHOLE, em_pixels)?;
    let sharp_width_pixels: f32 = advance_pixels(atlas, ACCIDENTAL_SHARP, em_pixels)?;
    let ledger_thickness_pixels: f32 =
        whole_thickness_pixels(LEDGER_THICKNESS_SPACES * space_pixels);
    for (bar, voicing) in (0..BARS_IN_STRIP).zip(SPIKE_VOICINGS) {
        // Each chord changes on its bar's downbeat, so its voicing stands
        // there: stacked noteheads, only at the change (EFS-1.5).
        let head_x_pixels: f32 =
            geometry.bar_left_pixels(bar) + VOICING_HEAD_X_SPACES * space_pixels;
        let mut notes: Vec<VoicingNote> = voicing.to_vec();
        notes.sort_by_key(|note| std::cmp::Reverse(note.staff_step));
        // Accidental columns, nearest the noteheads first: the lowest step
        // placed so far in each.
        let mut column_lowest_steps: Vec<i8> = Vec::new();
        for note in &notes {
            let y_pixels: f32 = staff.step_y_pixels(note.staff_step);
            place_music(
                atlas,
                NOTEHEAD_WHOLE,
                (head_x_pixels, y_pixels),
                em_pixels,
                glyphs,
            )?;
            if !note.sharp {
                continue;
            }
            let column: usize = if let Some(column) = column_lowest_steps
                .iter()
                .position(|lowest| lowest - note.staff_step >= ACCIDENTAL_CLEARANCE_STEPS)
            {
                column_lowest_steps[column] = note.staff_step;
                column
            } else {
                column_lowest_steps.push(note.staff_step);
                column_lowest_steps.len() - 1
            };
            // A chord has a handful of columns at most, so this is exact.
            #[allow(clippy::cast_precision_loss)]
            let column_number: f32 = column as f32;
            let x_pixels: f32 = head_x_pixels
                - ACCIDENTAL_GAP_SPACES * space_pixels
                - (column_number + 1.0) * sharp_width_pixels
                - column_number * ACCIDENTAL_COLUMN_GAP_SPACES * space_pixels;
            place_music(
                atlas,
                ACCIDENTAL_SHARP,
                (x_pixels, y_pixels),
                em_pixels,
                glyphs,
            )?;
        }
        // Ledger lines for notes beyond the staff, once per chord.
        let ledger_left_pixels: f32 = head_x_pixels - LEDGER_EXTENSION_SPACES * space_pixels;
        let ledger_right_pixels: f32 =
            head_x_pixels + head_width_pixels + LEDGER_EXTENSION_SPACES * space_pixels;
        for step in ledger_steps(&notes) {
            lines.push(horizontal_line(
                ledger_left_pixels,
                ledger_right_pixels,
                staff.step_y_pixels(step),
                ledger_thickness_pixels,
                theme::NOTE,
            ));
        }
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
        // symbol characters 22; on the rhythm line 36 (slashes, rests, a
        // dot and flags, by bar 4, 8, 2, 5, 4, 1, 8, 4); voicing noteheads
        // 30 and sharps 3.
        assert_eq!(glyphs.len(), 3 + 3 + 22 + 36 + 30 + 3);
        // Stems 29, beams 7, ledger lines 3 (C4 in bars 0 and 4, and the
        // C4 line under B3 in bar 6).
        assert_eq!(lines.len(), 29 + 7 + 3);
    }

    #[test]
    fn hiding_the_staff_leaves_the_symbols_and_rhythm_line() {
        let (lines, glyphs) = laid_out(false, 2560.0, 1440.0);
        assert_eq!(glyphs.len(), 3 + 22 + 36);
        assert_eq!(lines.len(), 29 + 7);
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
