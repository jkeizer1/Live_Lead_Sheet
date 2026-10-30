//! The GPU side: picks the graphics adapter, configures the window's
//! surface, and draws frames. Knows nothing about windows or the operating
//! system; it is handed a wgpu surface.

use crate::block_on::block_on;
use crate::error::RenderError;
use crate::theme;

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    configuration: wgpu::SurfaceConfiguration,
    adapter_summary: String,
}

impl Renderer {
    /// Chooses the high-performance adapter that can draw to `surface`,
    /// starts it, and configures the surface for a window of the given size.
    ///
    /// # Errors
    ///
    /// When no adapter can draw to the surface, the adapter will not start,
    /// or the surface cannot be configured at that size.
    pub fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width_pixels: u32,
        height_pixels: u32,
    ) -> Result<Renderer, RenderError> {
        let adapter: wgpu::Adapter =
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..wgpu::RequestAdapterOptions::default()
            }))
            .map_err(|source| RenderError::NoAdapter { source })?;
        let info: wgpu::AdapterInfo = adapter.get_info();
        let (device, queue): (wgpu::Device, wgpu::Queue) = block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default()),
        )
        .map_err(|source| RenderError::NoDevice {
            adapter_name: info.name.clone(),
            source,
        })?;
        let Some(mut configuration): Option<wgpu::SurfaceConfiguration> =
            surface.get_default_config(&adapter, width_pixels, height_pixels)
        else {
            return Err(RenderError::SurfaceUnsupported {
                adapter_name: info.name,
                width_pixels,
                height_pixels,
            });
        };
        // The default can be Immediate (no wait for the screen), which would
        // redraw thousands of times a second. Fifo waits for the screen's
        // refresh and is the one mode every Vulkan driver must offer.
        configuration.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &configuration);
        let adapter_summary: String = format!(
            "{} ({:?}, {:?}), surface format {:?}, present mode {:?}",
            info.name,
            info.backend,
            info.device_type,
            configuration.format,
            configuration.present_mode
        );
        Ok(Renderer {
            surface,
            device,
            queue,
            configuration,
            adapter_summary,
        })
    }

    /// Which adapter and surface settings were chosen, for the start-up line.
    #[must_use]
    pub fn adapter_summary(&self) -> &str {
        &self.adapter_summary
    }

    /// Follows the window's new size. A zero size (the window is minimised)
    /// is ignored: wgpu cannot configure a surface of zero pixels.
    pub fn resize(&mut self, width_pixels: u32, height_pixels: u32) {
        if width_pixels == 0 || height_pixels == 0 {
            return;
        }
        self.configuration.width = width_pixels;
        self.configuration.height = height_pixels;
        self.surface.configure(&self.device, &self.configuration);
    }

    /// Draws one frame: for now, the background colour.
    ///
    /// # Errors
    ///
    /// When the surface is lost or the driver rejects the frame. A frame
    /// that simply cannot be drawn now (window hidden, surface out of date)
    /// is skipped, and the next call tries again.
    pub fn draw_frame(&mut self) -> Result<(), RenderError> {
        let surface_texture: wgpu::SurfaceTexture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                // Drawable, but the surface no longer matches the window:
                // draw this frame, then reconfigure for the next.
                self.draw_to(&texture);
                self.queue.present(texture);
                self.surface.configure(&self.device, &self.configuration);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.configuration);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => return Err(RenderError::SurfaceLost),
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::SurfaceRejectedFrame);
            }
        };
        self.draw_to(&surface_texture);
        self.queue.present(surface_texture);
        Ok(())
    }

    fn draw_to(&self, surface_texture: &wgpu::SurfaceTexture) {
        let view: wgpu::TextureView = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder: wgpu::CommandEncoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            // Clearing is the whole pass: it draws nothing else yet.
            let _pass: wgpu::RenderPass<'_> =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("background"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(theme::BACKGROUND),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..wgpu::RenderPassDescriptor::default()
                });
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
