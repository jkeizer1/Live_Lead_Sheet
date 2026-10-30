//! Spike content for the strip: a clef, a time signature, eight bars of
//! notes and rests, and a chord symbol per bar, hard-coded. The real layout
//! will take these from the vocabulary through the published stream.
//!
//! Positions on the staff come from Bravura's metadata (`Bravura.json`), in
//! staff spaces: glyph origins sit on a staff line, stems attach at the
//! notehead's anchors.

use crate::error::RenderError;
use crate::glyph_atlas::{GlyphAtlas, GlyphFont};
use crate::glyph_instance::GlyphInstance;
use crate::line_instance::LineInstance;
use crate::strip_layout::{
    BARS_IN_STRIP, STEM_THICKNESS_SPACES, StripGeometry, snapped_center_pixels, vertical_line,
    whole_thickness_pixels,
};
use crate::theme;

const G_CLEF: char = '\u{E050}';
const TIME_SIGNATURE_4: char = '\u{E084}';
const NOTEHEAD_WHOLE: char = '\u{E0A2}';
const NOTEHEAD_HALF: char = '\u{E0A3}';
const NOTEHEAD_BLACK: char = '\u{E0A4}';
const REST_WHOLE: char = '\u{E4E3}';
const REST_HALF: char = '\u{E4E4}';
const REST_QUARTER: char = '\u{E4E5}';
const ACCIDENTAL_SHARP: char = '\u{E262}';

/// Quarter notes per bar in the spike's 4/4.
const QUARTER_NOTES_PER_BAR: u16 = 4;

/// Clef and time signature, in staff spaces from the staff's left end.
const CLEF_X_SPACES: f32 = 0.8;
const TIME_SIGNATURE_X_SPACES: f32 = 4.4;

/// Stem anchors of `noteheadBlack` and `noteheadHalf`, in staff spaces from
/// the notehead's origin, y up: `stemUpSE` (1.18, 0.168) and `stemDownNW`
/// (0, −0.168).
const STEM_UP_X_SPACES: f32 = 1.18;
const STEM_ATTACH_Y_SPACES: f32 = 0.168;
/// A stem reaches an octave (3.5 staff spaces) from the notehead's centre.
const STEM_LENGTH_SPACES: f32 = 3.5;
/// Notes from the middle line (step 4) up have their stems down.
const MIDDLE_LINE_STEP: i8 = 4;
/// Room after a barline before a bar's first note, in staff spaces, so an
/// accidental on the first beat clears the barline.
const BAR_LEAD_IN_SPACES: f32 = 1.0;
/// Gap between a sharp and its notehead, in staff spaces.
const ACCIDENTAL_GAP_SPACES: f32 = 0.25;

/// Chord symbols: size and height above the top staff line.
const CHORD_SYMBOL_EM_SPACES: f32 = 2.4;
const CHORD_SYMBOL_BASELINE_ABOVE_STAFF_SPACES: f32 = 2.2;
const CHORD_SYMBOL_INSET_SPACES: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpikeDuration {
    Quarter,
    Half,
    Whole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpikeSymbol {
    /// `staff_step`: half staff spaces up from the bottom line (E4 is 0).
    Note {
        staff_step: i8,
        duration: SpikeDuration,
        sharp: bool,
    },
    Rest {
        duration: SpikeDuration,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SpikeEvent {
    bar: u16,
    /// Quarter note within the bar, 0 to 3.
    beat: u16,
    symbol: SpikeSymbol,
}

const fn note(bar: u16, beat: u16, staff_step: i8, duration: SpikeDuration) -> SpikeEvent {
    SpikeEvent {
        bar,
        beat,
        symbol: SpikeSymbol::Note {
            staff_step,
            duration,
            sharp: false,
        },
    }
}

const fn sharp_note(bar: u16, beat: u16, staff_step: i8, duration: SpikeDuration) -> SpikeEvent {
    SpikeEvent {
        bar,
        beat,
        symbol: SpikeSymbol::Note {
            staff_step,
            duration,
            sharp: true,
        },
    }
}

const fn rest(bar: u16, beat: u16, duration: SpikeDuration) -> SpikeEvent {
    SpikeEvent {
        bar,
        beat,
        symbol: SpikeSymbol::Rest { duration },
    }
}

/// Steps: E4 0, F4 1, G4 2, A4 3, B4 4, C5 5, D5 6, E5 7.
const SPIKE_EVENTS: [SpikeEvent; 23] = [
    note(0, 0, 2, SpikeDuration::Quarter),
    note(0, 1, 5, SpikeDuration::Quarter),
    note(0, 2, 7, SpikeDuration::Quarter),
    note(0, 3, 5, SpikeDuration::Quarter),
    note(1, 0, 3, SpikeDuration::Half),
    note(1, 2, 5, SpikeDuration::Half),
    note(2, 0, 1, SpikeDuration::Quarter),
    note(2, 1, 3, SpikeDuration::Quarter),
    rest(2, 2, SpikeDuration::Quarter),
    note(2, 3, 6, SpikeDuration::Quarter),
    // Bar 3: G, B and D as one whole-note chord.
    note(3, 0, 2, SpikeDuration::Whole),
    note(3, 0, 4, SpikeDuration::Whole),
    note(3, 0, 6, SpikeDuration::Whole),
    note(4, 0, 7, SpikeDuration::Quarter),
    note(4, 1, 6, SpikeDuration::Quarter),
    note(4, 2, 5, SpikeDuration::Quarter),
    note(4, 3, 4, SpikeDuration::Quarter),
    sharp_note(5, 0, 1, SpikeDuration::Half),
    rest(5, 2, SpikeDuration::Half),
    sharp_note(6, 0, 6, SpikeDuration::Quarter),
    note(6, 1, 4, SpikeDuration::Quarter),
    note(6, 2, 3, SpikeDuration::Half),
    rest(7, 0, SpikeDuration::Whole),
];

/// One chord symbol per bar, named as EFS-1.5a sets out.
const SPIKE_CHORD_SYMBOLS: [&str; BARS_IN_STRIP as usize] =
    ["C", "Am7", "Dm7", "G7", "CM7", "F♯m7♭5", "B7", "Em"];

/// Appends the spike's music to the strip: glyphs for clef, time signature,
/// noteheads, rests, sharps and chord symbols, and lines for the stems.
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
    let space_pixels: f32 = geometry.space_pixels;
    let music_em_pixels: f32 = geometry.music_em_pixels();
    let mut place = |codepoint: char, x_pixels: f32, y_pixels: f32| -> Result<(), RenderError> {
        if let Some(glyph) = atlas.place(
            GlyphFont::Bravura,
            codepoint,
            x_pixels,
            y_pixels,
            music_em_pixels,
            theme::NOTE,
        )? {
            glyphs.push(glyph);
        }
        Ok(())
    };

    // The G clef's origin is on the G line (step 2); time signature digits
    // are centred on their origin, on the second and fourth lines.
    place(
        G_CLEF,
        geometry.staff_left_pixels + CLEF_X_SPACES * space_pixels,
        geometry.step_y_pixels(2),
    )?;
    let time_x_pixels: f32 = geometry.staff_left_pixels + TIME_SIGNATURE_X_SPACES * space_pixels;
    place(TIME_SIGNATURE_4, time_x_pixels, geometry.step_y_pixels(6))?;
    place(TIME_SIGNATURE_4, time_x_pixels, geometry.step_y_pixels(2))?;

    let stem_thickness_pixels: f32 = whole_thickness_pixels(space_pixels * STEM_THICKNESS_SPACES);
    for event in SPIKE_EVENTS {
        let center_x_pixels: f32 = event_center_x_pixels(geometry, event);
        match event.symbol {
            SpikeSymbol::Note {
                staff_step,
                duration,
                sharp,
            } => {
                let notehead: char = match duration {
                    SpikeDuration::Quarter => NOTEHEAD_BLACK,
                    SpikeDuration::Half => NOTEHEAD_HALF,
                    SpikeDuration::Whole => NOTEHEAD_WHOLE,
                };
                let head_width_pixels: f32 =
                    atlas.glyph(GlyphFont::Bravura, notehead)?.advance_ems * music_em_pixels;
                let head_x_pixels: f32 = center_x_pixels - head_width_pixels / 2.0;
                let head_y_pixels: f32 = geometry.step_y_pixels(staff_step);
                place(notehead, head_x_pixels, head_y_pixels)?;
                if sharp {
                    let sharp_width_pixels: f32 = atlas
                        .glyph(GlyphFont::Bravura, ACCIDENTAL_SHARP)?
                        .advance_ems
                        * music_em_pixels;
                    place(
                        ACCIDENTAL_SHARP,
                        head_x_pixels - sharp_width_pixels - ACCIDENTAL_GAP_SPACES * space_pixels,
                        head_y_pixels,
                    )?;
                }
                if duration != SpikeDuration::Whole {
                    lines.push(stem(
                        geometry,
                        head_x_pixels,
                        head_y_pixels,
                        staff_step,
                        stem_thickness_pixels,
                    ));
                }
            }
            SpikeSymbol::Rest { duration } => {
                // Quarter and half rests sit on the middle line; the whole
                // rest hangs from the fourth line (step 6).
                let (rest_glyph, staff_step): (char, i8) = match duration {
                    SpikeDuration::Quarter => (REST_QUARTER, MIDDLE_LINE_STEP),
                    SpikeDuration::Half => (REST_HALF, MIDDLE_LINE_STEP),
                    SpikeDuration::Whole => (REST_WHOLE, 6),
                };
                let rest_width_pixels: f32 =
                    atlas.glyph(GlyphFont::Bravura, rest_glyph)?.advance_ems * music_em_pixels;
                place(
                    rest_glyph,
                    center_x_pixels - rest_width_pixels / 2.0,
                    geometry.step_y_pixels(staff_step),
                )?;
            }
        }
    }

    let chord_em_pixels: f32 = CHORD_SYMBOL_EM_SPACES * space_pixels;
    let chord_baseline_y_pixels: f32 =
        geometry.staff_top_pixels - CHORD_SYMBOL_BASELINE_ABOVE_STAFF_SPACES * space_pixels;
    for (bar, chord_symbol) in (0..BARS_IN_STRIP).zip(SPIKE_CHORD_SYMBOLS) {
        atlas.text(
            GlyphFont::DejaVuSans,
            chord_symbol,
            geometry.bar_left_pixels(bar) + CHORD_SYMBOL_INSET_SPACES * space_pixels,
            chord_baseline_y_pixels,
            chord_em_pixels,
            theme::TEXT,
            glyphs,
        )?;
    }
    Ok(())
}

/// Where an event is centred across the bar: a whole bar's symbol in the
/// bar's middle; others in the middle of the quarter note they start on.
fn event_center_x_pixels(geometry: &StripGeometry, event: SpikeEvent) -> f32 {
    let bar_left_pixels: f32 = geometry.bar_left_pixels(event.bar);
    let whole_bar: bool = matches!(
        event.symbol,
        SpikeSymbol::Note {
            duration: SpikeDuration::Whole,
            ..
        } | SpikeSymbol::Rest {
            duration: SpikeDuration::Whole
        }
    );
    if whole_bar {
        bar_left_pixels + geometry.bar_width_pixels / 2.0
    } else {
        // The bar after its lead-in is shared by the quarter notes:
        // (beat + ½) q ÷ 4 q per bar × that width.
        let lead_in_pixels: f32 = BAR_LEAD_IN_SPACES * geometry.space_pixels;
        bar_left_pixels
            + lead_in_pixels
            + (geometry.bar_width_pixels - lead_in_pixels) * (f32::from(event.beat) + 0.5)
                / f32::from(QUARTER_NOTES_PER_BAR)
    }
}

/// The stem for a notehead at (`head_x_pixels`, `head_y_pixels`): up from
/// the right side below the middle line, down from the left side otherwise.
fn stem(
    geometry: &StripGeometry,
    head_x_pixels: f32,
    head_y_pixels: f32,
    staff_step: i8,
    thickness_pixels: f32,
) -> LineInstance {
    let space_pixels: f32 = geometry.space_pixels;
    let half_thickness_pixels: f32 = thickness_pixels / 2.0;
    if staff_step < MIDDLE_LINE_STEP {
        // The stem's right edge on the anchor; y runs down on screen.
        let x_pixels: f32 = snapped_center_pixels(
            head_x_pixels + STEM_UP_X_SPACES * space_pixels - half_thickness_pixels,
            thickness_pixels,
        );
        vertical_line(
            x_pixels,
            head_y_pixels - STEM_LENGTH_SPACES * space_pixels,
            head_y_pixels - STEM_ATTACH_Y_SPACES * space_pixels,
            thickness_pixels,
            theme::NOTE,
        )
    } else {
        // The stem's left edge on the anchor.
        let x_pixels: f32 =
            snapped_center_pixels(head_x_pixels + half_thickness_pixels, thickness_pixels);
        vertical_line(
            x_pixels,
            head_y_pixels + STEM_ATTACH_Y_SPACES * space_pixels,
            head_y_pixels + STEM_LENGTH_SPACES * space_pixels,
            thickness_pixels,
            theme::NOTE,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spike_music_is_laid_out_inside_the_window() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        for (width, height) in [(2560.0, 1440.0), (1280.0, 720.0), (3840.0, 2160.0)] {
            let geometry: StripGeometry = StripGeometry::new(width, height);
            let mut lines: Vec<LineInstance> = Vec::new();
            let mut glyphs: Vec<GlyphInstance> = Vec::new();
            spike_music(&geometry, &atlas, &mut lines, &mut glyphs).unwrap();
            // Clef 1, time signature 2, noteheads 20, sharps 2, rests 3,
            // chord-symbol characters 22.
            assert_eq!(glyphs.len(), 1 + 2 + 20 + 2 + 3 + 22);
            // A stem for every note but the 3 whole notes.
            assert_eq!(lines.len(), 20 - 3);
            for glyph in &glyphs {
                assert!(glyph.left_x_pixels >= 0.0 && glyph.top_y_pixels >= 0.0);
                assert!(glyph.left_x_pixels + glyph.width_pixels <= width);
                assert!(glyph.top_y_pixels + glyph.height_pixels <= height);
            }
        }
    }

    #[test]
    fn a_sharp_on_the_first_beat_clears_the_barline() {
        let atlas: GlyphAtlas = GlyphAtlas::embedded().unwrap();
        let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0);
        let mut lines: Vec<LineInstance> = Vec::new();
        let mut glyphs: Vec<GlyphInstance> = Vec::new();
        spike_music(&geometry, &atlas, &mut lines, &mut glyphs).unwrap();
        let sharp_width_pixels: f32 = atlas
            .glyph(GlyphFont::Bravura, ACCIDENTAL_SHARP)
            .unwrap()
            .advance_ems
            * geometry.music_em_pixels();
        // Bar 5 opens with F♯: no glyph's ink may start left of its barline.
        // A glyph's rectangle includes the field's empty border, so compare
        // the ink: the rectangle inset by the border.
        let border_pixels: f32 =
            atlas.spread_pixels * geometry.music_em_pixels() / atlas.pixels_per_em;
        let barline_x_pixels: f32 = geometry.bar_left_pixels(5);
        let sharp: &GlyphInstance = glyphs
            .iter()
            .find(|glyph| {
                let ink_left_pixels: f32 = glyph.left_x_pixels + border_pixels;
                ink_left_pixels > barline_x_pixels - sharp_width_pixels
                    && ink_left_pixels < barline_x_pixels + geometry.bar_width_pixels / 4.0
                    && glyph.top_y_pixels < geometry.staff_bottom_pixels
            })
            .unwrap();
        assert!(sharp.left_x_pixels + border_pixels > barline_x_pixels);
    }

    #[test]
    fn stems_go_up_below_the_middle_line_and_down_from_it() {
        let geometry: StripGeometry = StripGeometry::new(2560.0, 1440.0);
        let up: LineInstance = stem(&geometry, 100.0, geometry.step_y_pixels(2), 2, 3.0);
        let down: LineInstance = stem(&geometry, 100.0, geometry.step_y_pixels(6), 6, 3.0);
        assert!(up.start_y_pixels < geometry.step_y_pixels(2));
        assert!(up.start_x_pixels > 100.0 + geometry.space_pixels);
        assert!(down.end_y_pixels > geometry.step_y_pixels(6));
        assert!(down.start_x_pixels < 100.0 + geometry.space_pixels);
    }
}
