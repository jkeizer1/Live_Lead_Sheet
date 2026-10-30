//! The one native window and its event loop (winit). This is platform code:
//! it creates the window and the wgpu surface and hands the surface to the
//! renderer, which knows nothing about windows.

use std::sync::Arc;

use render::renderer::Renderer;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// The spike's target: 2560 × 1440 pixels.
const WINDOW_WIDTH_PIXELS: u32 = 2560;
const WINDOW_HEIGHT_PIXELS: u32 = 1440;

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
        let renderer: Renderer = Renderer::new(&instance, surface, size.width, size.height)
            .map_err(|e| e.to_string())?;
        println!("Drawing with {}", renderer.adapter_summary());
        Ok(OpenWindow { renderer, window })
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
            WindowEvent::Resized(size) => {
                open_window.renderer.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => match open_window.renderer.draw_frame() {
                Ok(()) => open_window.window.request_redraw(),
                Err(error) => self.fail(event_loop, error.to_string()),
            },
            _ => {}
        }
    }
}
