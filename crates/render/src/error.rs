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
