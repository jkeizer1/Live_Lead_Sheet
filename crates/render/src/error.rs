//! Failures of the GPU drawing. Each says what was being done.

use std::fmt;

#[derive(Debug)]
pub enum RenderError {
    /// No graphics adapter can draw to the window.
    NoAdapter { source: wgpu::RequestAdapterError },
    /// The chosen adapter would not give a device.
    NoDevice {
        adapter_name: String,
        source: wgpu::RequestDeviceError,
    },
    /// The window's surface cannot be used with the chosen adapter.
    SurfaceUnsupported {
        adapter_name: String,
        width_pixels: u32,
        height_pixels: u32,
    },
    /// The surface was lost while drawing; the window must be recreated.
    SurfaceLost,
    /// The GPU driver rejected the frame.
    SurfaceRejectedFrame,
    /// A frame asked for more lines than the line buffer holds.
    TooManyLines { given: usize, capacity: usize },
    /// A frame asked for more glyphs than the glyph buffer holds.
    TooManyGlyphs { given: usize, capacity: usize },
    /// The glyph atlas compiled into the program is malformed. Line 0 means
    /// the table as a whole, or the image.
    AtlasUnreadable { line_number: usize, reason: String },
    /// The layout asked for a glyph the atlas does not have.
    GlyphMissing { font: &'static str, codepoint: char },
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::NoAdapter { source } => write!(
                f,
                "No graphics adapter can draw to the window ({source}). \
                 Check that Vulkan works: run vulkaninfo --summary."
            ),
            RenderError::NoDevice {
                adapter_name,
                source,
            } => write!(
                f,
                "Graphics adapter {adapter_name:?} would not start ({source})."
            ),
            RenderError::SurfaceUnsupported {
                adapter_name,
                width_pixels,
                height_pixels,
            } => write!(
                f,
                "Graphics adapter {adapter_name:?} cannot draw to a {width_pixels} × \
                 {height_pixels} pixel window."
            ),
            RenderError::SurfaceLost => {
                write!(
                    f,
                    "The window's drawing surface was lost. Restart the program."
                )
            }
            RenderError::TooManyLines { given, capacity } => write!(
                f,
                "A frame asked for {given} lines; the most that fit is {capacity}."
            ),
            RenderError::TooManyGlyphs { given, capacity } => write!(
                f,
                "A frame asked for {given} glyphs; the most that fit is {capacity}."
            ),
            RenderError::AtlasUnreadable {
                line_number,
                reason,
            } => write!(
                f,
                "The glyph atlas is damaged (glyphs.txt line {line_number}: {reason}). \
                 Rebuild it: cargo run --release -p atlas_tool"
            ),
            RenderError::GlyphMissing { font, codepoint } => write!(
                f,
                "The glyph atlas has no {font} glyph U+{:04X} ({codepoint:?}). Add it to \
                 atlas_tool's glyph list and rebuild: cargo run --release -p atlas_tool",
                u32::from(*codepoint)
            ),
            RenderError::SurfaceRejectedFrame => {
                write!(
                    f,
                    "The graphics driver rejected a frame. Restart the program."
                )
            }
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RenderError::NoAdapter { source } => Some(source),
            RenderError::NoDevice { source, .. } => Some(source),
            _ => None,
        }
    }
}
