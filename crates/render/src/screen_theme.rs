//! Colours of the editor's screen: the dark theme with soft contrast
//! (EFS-1.13), every control included. Each colour is written as the sRGB
//! values a designer reads, and turned into the linear light the GPU blends
//! in (the same conversion as `theme::BACKGROUND`).

/// sRGB 0 to 255 → linear light 0 to 1:
/// linear = ((s / 255 + 0.055) / 1.055) ^ 2.4, or s / 255 / 12.92 when dark.
#[must_use]
pub fn srgb(red: u8, green: u8, blue: u8) -> [f32; 4] {
    let channel = |value: u8| -> f32 {
        let encoded: f32 = f32::from(value) / 255.0;
        if encoded <= 0.040_45 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    };
    [channel(red), channel(green), channel(blue), 1.0]
}

/// The same colour, partly see-through.
#[must_use]
pub fn with_alpha(color: [f32; 4], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], alpha]
}

/// Panels a step lighter than the window's background (24, 26, 30).
#[must_use]
pub fn panel() -> [f32; 4] {
    srgb(32, 35, 41)
}

#[must_use]
pub fn panel_raised() -> [f32; 4] {
    srgb(40, 44, 52)
}

#[must_use]
pub fn border() -> [f32; 4] {
    srgb(62, 67, 78)
}

#[must_use]
pub fn text() -> [f32; 4] {
    srgb(206, 210, 218)
}

#[must_use]
pub fn text_soft() -> [f32; 4] {
    srgb(150, 156, 168)
}

#[must_use]
pub fn text_faint() -> [f32; 4] {
    srgb(102, 108, 120)
}

/// The selection: a soft blue fill and a brighter edge.
#[must_use]
pub fn selection_fill() -> [f32; 4] {
    srgb(44, 62, 88)
}

#[must_use]
pub fn selection_edge() -> [f32; 4] {
    srgb(104, 148, 204)
}

/// The bars of the selected section, under the music: a dark pale yellow
/// near the panel's own lightness (panel 32, 35, 41), apart from the blues
/// of what is selected. Written as the finished colour, not a see-through
/// wash: a faint pale yellow blended over this blue-grey comes out grey.
#[must_use]
pub fn section_fill() -> [f32; 4] {
    srgb(46, 43, 27)
}

/// The same, on the selected track's raised background (40, 44, 52).
#[must_use]
pub fn section_fill_raised() -> [f32; 4] {
    srgb(56, 52, 33)
}

/// A note derived from a sample source, not composed: the notes' grey
/// turned quieter and a little green, so it reads as "follows from".
#[must_use]
pub fn derived_note() -> [f32; 4] {
    srgb(134, 168, 146)
}

/// The keyboard's focus: which pane the arrows act in.
#[must_use]
pub fn focus_edge() -> [f32; 4] {
    srgb(132, 172, 222)
}

/// A preview not yet kept: a soft green.
#[must_use]
pub fn preview() -> [f32; 4] {
    srgb(118, 178, 128)
}

/// The playhead and the bar being played: the soft amber of `theme::PLAYHEAD`.
#[must_use]
pub fn playhead() -> [f32; 4] {
    srgb(214, 160, 90)
}

#[must_use]
pub fn problem() -> [f32; 4] {
    srgb(214, 120, 110)
}

#[must_use]
pub fn done() -> [f32; 4] {
    srgb(150, 196, 150)
}

/// Staff lines, barlines and notes, as the play strip draws them.
#[must_use]
pub fn staff_line() -> [f32; 4] {
    srgb(110, 116, 128)
}

#[must_use]
pub fn note() -> [f32; 4] {
    srgb(184, 190, 202)
}

/// The colour of a key on the key-centre lane: a hue for its place on the
/// circle of fifths, so neighbouring keys have neighbouring hues and a
/// change of key shows at a glance. Soft: low saturation, dark.
#[must_use]
pub fn key_hue(fifths: i8, minor: bool) -> [f32; 4] {
    // 30° of hue per fifth: C at 210° (blue), G 240°, F 180°, ...
    let hue_degrees: f32 = (210.0 + 30.0 * f32::from(fifths)).rem_euclid(360.0);
    let lightness: f32 = if minor { 0.27 } else { 0.33 };
    hsl(hue_degrees, 0.32, lightness)
}

/// Hue (degrees), saturation and lightness (0 to 1) → linear light.
fn hsl(hue_degrees: f32, saturation: f32, lightness: f32) -> [f32; 4] {
    let chroma: f32 = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector: f32 = hue_degrees / 60.0;
    let second: f32 = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue): (f32, f32, f32) = match sector {
        s if s < 1.0 => (chroma, second, 0.0),
        s if s < 2.0 => (second, chroma, 0.0),
        s if s < 3.0 => (0.0, chroma, second),
        s if s < 4.0 => (0.0, second, chroma),
        s if s < 5.0 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let lift: f32 = lightness - chroma / 2.0;
    // 0 ≤ (channel + lift) × 255 ≤ 255.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |value: f32| -> u8 { ((value + lift) * 255.0).round().clamp(0.0, 255.0) as u8 };
    srgb(byte(red), byte(green), byte(blue))
}
