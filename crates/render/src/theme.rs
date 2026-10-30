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
