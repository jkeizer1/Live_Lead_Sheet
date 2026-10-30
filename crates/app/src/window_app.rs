//! The one native window and its event loop (winit). This is platform code:
//! it creates the window and the wgpu surface and hands the surface to the
//! renderer, which knows nothing about windows.

use std::sync::Arc;
use std::time::Instant;

use render::error::RenderError;
use render::glyph_atlas::GlyphAtlas;
use render::glyph_instance::GlyphInstance;
use render::line_instance::LineInstance;
use render::renderer::Renderer;
use render::spike_music::spike_music;
use render::strip_layout::{BARS_IN_STRIP, StripGeometry, strip_lines};
use render::wheel_instance::WheelInstance;
use render::wheel_layout::{wheel_labels, wheel_layout};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The spike's target: 2560 × 1440 pixels.
const WINDOW_WIDTH_PIXELS: u32 = 2560;
const WINDOW_HEIGHT_PIXELS: u32 = 1440;

/// Spike playhead speed: 120 quarter notes per minute, 4 quarter notes per
/// bar, so one bar lasts 4 q / (120 q/min) × 60 s/min = 2 s.
const SPIKE_SECONDS_PER_BAR: f32 = 2.0;

/// Opens the window and runs until it is closed.
///
/// # Errors
///
/// A message, in the musician's terms, when the window or the drawing cannot
/// start or fails while running.
pub fn run() -> Result<(), String> {
    let event_loop: EventLoop<()> =
        EventLoop::new().map_err(|e| format!("Cannot start the event loop: {e}"))?;
    // Poll: keep delivering events, so the window redraws continuously.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app: WindowApp = WindowApp {
        open_window: None,
        failure: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("The event loop failed: {e}"))?;
    match app.failure {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

/// What exists once the window is open.
struct OpenWindow {
    // The renderer holds a surface made from this window, so the window must
    // outlive it: `Arc` shares ownership, and fields drop in order, so the
    // renderer (first) is gone before the window.
    renderer: Renderer,
    window: Arc<Window>,
    /// Glyph positions and sizes, for layout; the renderer holds the image.
    atlas: GlyphAtlas,
    /// When the playhead was at the start of the strip.
    started_at: Instant,
    /// Wheel tinted by function; the C key switches it off and on.
    color_by_function: bool,
}

struct WindowApp {
    open_window: Option<OpenWindow>,
    /// Set when something failed; `run` reports it after the loop ends.
    failure: Option<String>,
}

impl WindowApp {
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.failure = Some(message);
        event_loop.exit();
    }

    fn open(event_loop: &ActiveEventLoop) -> Result<OpenWindow, String> {
        let attributes = Window::default_attributes()
            .with_title("Live Lead Sheet")
            .with_inner_size(PhysicalSize::new(WINDOW_WIDTH_PIXELS, WINDOW_HEIGHT_PIXELS));
        let window: Arc<Window> = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| format!("Cannot open the window: {e}"))?,
        );
        let size: PhysicalSize<u32> = window.inner_size();
        let instance: wgpu::Instance = wgpu::Instance::default();
        let surface: wgpu::Surface<'static> = instance
            .create_surface(Arc::clone(&window))
            .map_err(|e| format!("Cannot draw in the window: {e}"))?;
        let atlas: GlyphAtlas = GlyphAtlas::embedded().map_err(|e| e.to_string())?;
        let renderer: Renderer = Renderer::new(&instance, surface, size.width, size.height, &atlas)
            .map_err(|e| e.to_string())?;
        println!("Drawing with {}", renderer.adapter_summary());
        Ok(OpenWindow {
            renderer,
            window,
            atlas,
            started_at: Instant::now(),
            color_by_function: true,
        })
    }
}

impl ApplicationHandler for WindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Called at start-up (and again on some systems); open only once.
        if self.open_window.is_some() {
            return;
        }
        match WindowApp::open(event_loop) {
            Ok(open_window) => {
                open_window.window.request_redraw();
                self.open_window = Some(open_window);
            }
            Err(message) => self.fail(event_loop, message),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(open_window) = self.open_window.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Character(ref character),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } if character.eq_ignore_ascii_case("c") => {
                open_window.color_by_function = !open_window.color_by_function;
            }
            WindowEvent::Resized(size) => {
                open_window.renderer.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => match draw(open_window) {
                Ok(()) => open_window.window.request_redraw(),
                Err(error) => self.fail(event_loop, error.to_string()),
            },
            _ => {}
        }
    }
}

/// Lays out and draws one frame of the spike.
fn draw(open_window: &mut OpenWindow) -> Result<(), RenderError> {
    let size: PhysicalSize<u32> = open_window.window.inner_size();
    // Window sizes are far below 2^24, so the conversion to f32 is exact.
    #[allow(clippy::cast_precision_loss)]
    let (width_pixels, height_pixels): (f32, f32) = (size.width as f32, size.height as f32);
    let elapsed_seconds: f32 = open_window.started_at.elapsed().as_secs_f32();
    let strip_seconds: f32 = SPIKE_SECONDS_PER_BAR * f32::from(BARS_IN_STRIP);
    // The playhead wraps to the first bar after the last.
    let playhead_fraction: f32 = (elapsed_seconds / strip_seconds).fract();
    // Spike: the wheel's current wedge steps once per bar. The value is
    // non-negative and far below usize's range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let current_wedge: usize = (elapsed_seconds / SPIKE_SECONDS_PER_BAR).floor() as usize;

    let geometry: StripGeometry = StripGeometry::new(width_pixels, height_pixels);
    let mut lines: Vec<LineInstance> = strip_lines(&geometry, playhead_fraction);
    let mut glyphs: Vec<GlyphInstance> = Vec::new();
    spike_music(&geometry, &open_window.atlas, &mut lines, &mut glyphs)?;
    let wheel: WheelInstance = wheel_layout(
        width_pixels,
        height_pixels,
        current_wedge,
        open_window.color_by_function,
    );
    wheel_labels(&wheel, &open_window.atlas, &mut glyphs)?;
    open_window.renderer.draw_frame(&lines, &glyphs, &wheel)
}
