//! The one native window and its event loop (winit). This is platform code:
//! it creates the window and the wgpu surface and hands the surface to the
//! renderer, turns keys and clicks into the session's commands, saves and
//! watches the song file, and asks the sending thread to play.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use composer::session::{Command, MessageKind, Session};
use composer::timeline::EntryView;
use player::schedule::{ScheduledEvent, schedule};
use player::song_plan::{SongPlan, plan_song};
use player::studio_configuration::StudioConfiguration;
use render::canvas::{Frame, HitTarget};
use render::glyph_atlas::GlyphAtlas;
use render::renderer::Renderer;
use render::screen::{PlayView, draw_screen};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{ModifiersState, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::key_map::{Action, action_for};
use crate::playback::{PlayRequest, Player};
use crate::song_files::{SongFile, short_name};

/// The window opens at the target screen's size: 2560 × 1440 pixels.
const WINDOW_WIDTH_PIXELS: u32 = 2560;
const WINDOW_HEIGHT_PIXELS: u32 = 1440;

/// How often the song file is looked at for changes made elsewhere.
const FILE_CHECK_INTERVAL: Duration = Duration::from_secs(1);
/// A second close within this time quits without saving.
const QUIT_CONFIRM_WINDOW: Duration = Duration::from_secs(5);
/// Two clicks on a library row within this time keep it.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// What the program was started with.
pub struct Startup {
    pub session: Session,
    pub song_file: SongFile,
    /// The studio, when one was given and read.
    pub studio: Option<StudioConfiguration>,
    pub studio_path: Option<PathBuf>,
}

/// Opens the window and runs until it is closed.
///
/// # Errors
///
/// A message, in the musician's terms, when the window or the drawing cannot
/// start or fails while running.
pub fn run(startup: Startup) -> Result<(), String> {
    let event_loop: EventLoop<()> =
        EventLoop::new().map_err(|e| format!("Cannot start the event loop: {e}"))?;
    // Poll: keep delivering events, so the window redraws continuously and
    // the playhead moves.
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app: WindowApp = WindowApp {
        startup: Some(startup),
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
    atlas: GlyphAtlas,
    session: Session,
    song_file: SongFile,
    studio: Option<StudioConfiguration>,
    player: Option<Player>,
    /// About the MIDI output, for the header.
    output_status: String,
    /// The last frame's click regions.
    last_frame_hits: Option<Frame>,
    cursor: Option<PhysicalPosition<f64>>,
    modifiers: ModifiersState,
    last_file_check: Instant,
    quit_asked_at: Option<Instant>,
    last_click: Option<(Instant, HitTarget)>,
    /// The number of the last message written to the terminal.
    last_logged_message_number: u64,
}

struct WindowApp {
    startup: Option<Startup>,
    open_window: Option<OpenWindow>,
    failure: Option<String>,
}

impl WindowApp {
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.failure = Some(message);
        event_loop.exit();
    }

    fn open(event_loop: &ActiveEventLoop, startup: Startup) -> Result<OpenWindow, String> {
        let title: String = format!("Live Lead Sheet - {}", short_name(&startup.song_file.path));
        let attributes = Window::default_attributes()
            .with_title(title)
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
        let (player, output_status): (Option<Player>, String) = match (
            &startup.studio,
            &startup.studio_path,
        ) {
            (Some(studio), Some(path)) => match Player::start(studio.port_names().to_vec()) {
                Ok(player) => {
                    println!(
                        "MIDI output ports for {} (connect them with the MIDI router or aconnect):",
                        short_name(path)
                    );
                    for port in player.ports() {
                        println!("  {port}");
                    }
                    let status: String =
                        format!("{} ports open ({})", player.ports().len(), short_name(path));
                    (Some(player), status)
                }
                Err(problem) => {
                    eprintln!("{problem}");
                    (
                        None,
                        String::from("MIDI output did not open: see the terminal"),
                    )
                }
            },
            _ => (None, String::from("No studio file given: playing is off")),
        };
        Ok(OpenWindow {
            renderer,
            window,
            atlas,
            session: startup.session,
            song_file: startup.song_file,
            studio: startup.studio,
            player,
            output_status,
            last_frame_hits: None,
            cursor: None,
            modifiers: ModifiersState::empty(),
            last_file_check: Instant::now(),
            quit_asked_at: None,
            last_click: None,
            last_logged_message_number: 0,
        })
    }
}

impl ApplicationHandler for WindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Called at start-up (and again on some systems); open only once.
        let Some(startup) = self.startup.take() else {
            return;
        };
        match WindowApp::open(event_loop, startup) {
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
            WindowEvent::CloseRequested => {
                if open_window.may_quit() {
                    event_loop.exit();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => open_window.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if let Some(action) = action_for(code, open_window.modifiers, &open_window.session)
                    && open_window.act(action)
                {
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => open_window.cursor = Some(position),
            WindowEvent::CursorLeft { .. } => open_window.cursor = None,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => open_window.click(),
            WindowEvent::MouseWheel { delta, .. } => open_window.wheel(delta),
            WindowEvent::Resized(size) => open_window.renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                open_window.housekeeping();
                open_window.log_new_problem();
                match open_window.draw() {
                    Ok(()) => open_window.window.request_redraw(),
                    Err(error) => self.fail(event_loop, error),
                }
            }
            _ => {}
        }
    }
}

impl OpenWindow {
    /// Carries out an action; true when the program should end.
    fn act(&mut self, action: Action) -> bool {
        match action {
            Action::Session(command) => {
                self.session.handle(command);
                false
            }
            Action::PlayOrStop => {
                self.play_or_stop(false, false);
                false
            }
            Action::PlayFromStart => {
                self.play_or_stop(true, false);
                false
            }
            Action::LoopEntry => {
                self.play_or_stop(false, true);
                false
            }
            Action::Save => {
                self.save();
                false
            }
            Action::Quit => self.may_quit(),
        }
    }

    /// Whether to quit now: at once when everything is saved, else on the
    /// second ask within a few seconds, so no edit is lost by one keypress.
    fn may_quit(&mut self) -> bool {
        if !self.session.document().has_unsaved_edits() {
            return true;
        }
        if self
            .quit_asked_at
            .is_some_and(|asked| asked.elapsed() < QUIT_CONFIRM_WINDOW)
        {
            return true;
        }
        self.quit_asked_at = Some(Instant::now());
        self.session.say(
            MessageKind::Problem,
            String::from("Unsaved edits. Ctrl+S saves them; close again within 5 seconds to quit without saving."),
        );
        false
    }

    fn save(&mut self) {
        let text: String = self.session.document().text().to_string();
        match self.song_file.save(&text) {
            Ok(()) => {
                self.session.mark_saved();
                self.session.say(
                    MessageKind::Done,
                    format!("Saved {}.", short_name(&self.song_file.path)),
                );
            }
            Err(problem) => self.session.say(MessageKind::Problem, problem),
        }
    }

    fn play_or_stop(&mut self, from_start: bool, looping: bool) {
        let Some(player) = &self.player else {
            self.session.say(
                MessageKind::Problem,
                String::from("Playing is off: start the program with a studio file, e.g. cargo run --release -p app -- songs/Demo.song songs/Demo.studio"),
            );
            return;
        };
        if player.playing() {
            if let Err(problem) = player.stop() {
                self.session.say(MessageKind::Problem, problem);
            }
            return;
        }
        let Some(studio) = &self.studio else {
            return;
        };
        let timeline = self.session.timeline();
        let Some(entry) = timeline.entries.get(self.session.selection.entry) else {
            return;
        };
        if entry.muted && !from_start {
            self.session.say(
                MessageKind::Problem,
                String::from(
                    "This entry is muted (repeat 0); press M to restore it, or pick another.",
                ),
            );
            return;
        }
        let (start_ticks, end_ticks): (u64, Option<u64>) = if from_start {
            (0, None)
        } else if looping {
            (
                entry.start_ticks,
                Some(entry.start_ticks + entry.length_ticks()),
            )
        } else {
            (entry.start_ticks, None)
        };
        let what: String =
            describe_play(entry, from_start, looping, self.session.preview().is_some());
        let song = self.session.playing_song();
        let request: Result<PlayRequest, String> = plan_song(song, studio)
            .map_err(|error| format!("Cannot play: {error}"))
            .and_then(|plan: SongPlan| {
                let events: Vec<ScheduledEvent> = schedule(
                    &plan.clock_port,
                    &plan.note_tracks,
                    &plan.control_tracks,
                    plan.stretch_length,
                )
                .map_err(|error| format!("Cannot play: {error}"))?;
                Ok(PlayRequest {
                    events,
                    tempo: plan.tempo,
                    tempo_changes: plan.tempo_changes.clone(),
                    start_ticks,
                    end_ticks,
                    looping,
                    clock_port: plan.clock_port.clone(),
                    note_outputs: plan
                        .note_tracks
                        .iter()
                        .map(|track| (track.port().clone(), track.channel()))
                        .collect(),
                })
            });
        match request.and_then(|request| player.play(request)) {
            Ok(()) => self.session.say(MessageKind::Done, what),
            Err(problem) => self.session.say(MessageKind::Problem, problem),
        }
    }

    fn click(&mut self) {
        let (Some(cursor), Some(frame)) = (self.cursor, &self.last_frame_hits) else {
            return;
        };
        // Window positions are far below 2^24, so f32 holds them to well
        // within a pixel.
        #[allow(clippy::cast_possible_truncation)]
        let Some(target) = frame.hit_at(cursor.x as f32, cursor.y as f32) else {
            return;
        };
        let double: bool = self
            .last_click
            .is_some_and(|(when, previous)| previous == target && when.elapsed() < DOUBLE_CLICK);
        self.last_click = Some((Instant::now(), target));
        let command: Command = match target {
            HitTarget::Mode(mode) => Command::SetMode(mode),
            HitTarget::LibraryTab(tab) => Command::SetTab(tab),
            HitTarget::Entry(entry) => Command::SelectEntry(entry),
            HitTarget::Bar { entry, bar } => Command::SelectBar { entry, bar },
            HitTarget::Track(track) => Command::SelectTrack(track),
            HitTarget::Element {
                entry,
                bar,
                track,
                element,
                pass_number,
            } => Command::SelectElement {
                entry,
                bar,
                track,
                element,
                pass_number,
            },
            HitTarget::LibraryRow(row) if double => {
                self.session.handle(Command::SelectLibraryRow(row));
                Command::Commit
            }
            HitTarget::LibraryRow(row) => Command::SelectLibraryRow(row),
            HitTarget::LineForm(track) => {
                // Selecting the track again would forget the question asked
                // by the first click, so only a different track is selected.
                if self.session.selection.track != track {
                    self.session.handle(Command::SelectTrack(track));
                }
                Command::ConvertLineForm
            }
        };
        self.session.handle(command);
    }

    /// The wheel: Ctrl zooms; over the library it browses.
    fn wheel(&mut self, delta: MouseScrollDelta) {
        let lines: f32 = match delta {
            MouseScrollDelta::LineDelta(_, lines) => lines,
            // Pixel deltas are small numbers of pixels; 40 pixels is about
            // one line of a wheel.
            #[allow(clippy::cast_possible_truncation)]
            MouseScrollDelta::PixelDelta(position) => (position.y / 40.0) as f32,
        };
        if lines == 0.0 {
            return;
        }
        let up: bool = lines > 0.0;
        if self.modifiers.control_key() {
            // Like Ctrl+= and Ctrl+-: wider bars, fewer across, or more.
            self.session
                .handle(Command::StaffBarWidth(if up { 1 } else { -1 }));
            return;
        }
        let over_library: bool = match (self.cursor, &self.last_frame_hits) {
            #[allow(clippy::cast_possible_truncation)]
            (Some(cursor), Some(frame)) => matches!(
                frame.hit_at(cursor.x as f32, cursor.y as f32),
                Some(HitTarget::LibraryRow(_) | HitTarget::LibraryTab(_))
            ),
            _ => false,
        };
        if over_library {
            if self.session.pane != composer::session::Pane::Library {
                self.session.handle(Command::SwitchPane);
            }
            self.session
                .handle(if up { Command::Up } else { Command::Down });
        }
    }

    /// Writes a new problem message to the terminal too, so it can be
    /// copied whole; the message line may cut it short. Once per frame
    /// catches every message, since each key or click is followed by one.
    fn log_new_problem(&mut self) {
        if let Some(message) = self.session.message()
            && message.number > self.last_logged_message_number
        {
            self.last_logged_message_number = message.number;
            if message.kind == MessageKind::Problem {
                println!("Problem: {}", message.text);
            }
        }
    }

    /// Between frames: the sending thread's problems, and the song file.
    fn housekeeping(&mut self) {
        if let Some(player) = &self.player
            && let Some(problem) = player.next_problem()
        {
            self.session
                .say(MessageKind::Problem, format!("MIDI output: {problem}"));
        }
        if self.last_file_check.elapsed() < FILE_CHECK_INTERVAL {
            return;
        }
        self.last_file_check = Instant::now();
        match self.song_file.changed_text() {
            Ok(Some(text)) if text != self.session.document().text() => {
                if self.session.document().has_unsaved_edits() {
                    self.session.say(
                        MessageKind::Problem,
                        String::from("The song file changed outside, but there are unsaved edits here. Ctrl+S keeps these; Ctrl+Z back to the saved version, then the file's changes load."),
                    );
                } else {
                    self.session.reload(text);
                }
            }
            Ok(_) => {}
            Err(problem) => self.session.say(MessageKind::Problem, problem),
        }
    }

    fn draw(&mut self) -> Result<(), String> {
        let size: PhysicalSize<u32> = self.window.inner_size();
        // Window sizes are far below 2^24, so the conversion to f32 is exact.
        #[allow(clippy::cast_precision_loss)]
        let (width_pixels, height_pixels): (f32, f32) = (size.width as f32, size.height as f32);
        let play: PlayView = match &self.player {
            Some(player) => PlayView {
                playing: player.playing(),
                looping: player.looping(),
                position_ticks: player.position_ticks(),
                status: self.output_status.clone(),
            },
            None => PlayView {
                playing: false,
                looping: false,
                position_ticks: None,
                status: self.output_status.clone(),
            },
        };
        let frame: Frame = draw_screen(
            &self.session,
            &play,
            &self.atlas,
            width_pixels,
            height_pixels,
        )
        .map_err(|e| e.to_string())?;
        self.renderer
            .draw_frame(&frame.lines, &frame.glyphs)
            .map_err(|e| e.to_string())?;
        self.last_frame_hits = Some(frame);
        Ok(())
    }
}

/// What the message line says when playing starts.
fn describe_play(entry: &EntryView, from_start: bool, looping: bool, preview: bool) -> String {
    let what: &str = if preview { "the preview" } else { "the song" };
    if from_start {
        format!("Playing {what} from the start. Space stops.")
    } else if looping {
        format!("Looping \"{}\" in {what}. Space stops.", entry.section_name)
    } else {
        format!(
            "Playing {what} from \"{}\". Space stops.",
            entry.section_name
        )
    }
}
