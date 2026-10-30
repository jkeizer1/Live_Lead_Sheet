//! Colours of the dark theme: soft contrast everywhere, no bright white.

/// The window background, sRGB (24, 26, 30) out of 255.
///
/// wgpu takes a clear colour in linear light when the surface format is sRGB
/// (the renderer picks one), so these are the sRGB values converted:
/// linear = ((s / 255 + 0.055) / 1.055) ^ 2.4, giving 0.0091, 0.0103, 0.0130.
pub const BACKGROUND: wgpu::Color = wgpu::Color {
    r: 0.0091,
    g: 0.0103,
    b: 0.0130,
    a: 1.0,
};

/// Staff lines: sRGB (110, 116, 128), as linear light (same conversion as
/// `BACKGROUND`). Soft, well below white.
pub const STAFF_LINE: [f32; 4] = [0.156, 0.175, 0.216, 1.0];

/// Barlines: sRGB (150, 156, 168), a little brighter than the staff lines.
pub const BARLINE: [f32; 4] = [0.305, 0.333, 0.392, 1.0];

/// Playhead: sRGB (214, 160, 90), a soft amber.
pub const PLAYHEAD: [f32; 4] = [0.672, 0.350, 0.103, 1.0];

/// Wheel wedge tints by function, soft and a little muted. sRGB (120, 160,
/// 200), (124, 176, 136), (208, 140, 112) and, with colour off, a neutral
/// (140, 146, 160), as linear light.
pub const WHEEL_TONIC: [f32; 4] = [0.188, 0.352, 0.578, 1.0];
pub const WHEEL_SUBDOMINANT: [f32; 4] = [0.202, 0.434, 0.246, 1.0];
pub const WHEEL_DOMINANT: [f32; 4] = [0.631, 0.262, 0.162, 1.0];
pub const WHEEL_NEUTRAL: [f32; 4] = [0.262, 0.287, 0.352, 1.0];

/// Text (chord symbols, wheel labels): sRGB (206, 210, 218), light but not
/// white.
pub const TEXT: [f32; 4] = [0.617, 0.644, 0.701, 1.0];

/// Notes, rests, clef and stems: sRGB (176, 182, 194), a step below text.
pub const NOTE: [f32; 4] = [0.434, 0.468, 0.539, 1.0];

/// The hide/show staff control: a thin border, sRGB (96, 102, 114), and a
/// label a step below text, sRGB (168, 174, 186). Plainly visible, never
/// bright.
pub const CONTROL_BORDER: [f32; 4] = [0.117, 0.133, 0.168, 1.0];
pub const CONTROL_TEXT: [f32; 4] = [0.392, 0.423, 0.491, 1.0];

/// The current wedge's numeral: the background colour, sRGB (24, 26, 30),
/// dark on the bright fill, where light text would lose its contrast.
pub const WHEEL_CURRENT_NUMERAL: [f32; 4] = [0.0091, 0.0103, 0.0130, 1.0];
