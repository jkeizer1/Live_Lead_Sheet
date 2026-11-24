//! The one native window and its event loop (winit). This is platform code:
//! it creates the window and the wgpu surface and hands the surface to the
//! renderer, turns keys and clicks into the session's commands and the
//! shell's requests (the menu, the folder browser, questions), opens and
//! closes songs, and asks the sending thread to play.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use composer::repair::RepairTarget;
use composer::session::{Command, MessageKind};
use composer::shell::{Confirmation, MenuCommand, Pending, Shell, ShellKey, ShellRequest};
use render::canvas::{Frame, HitTarget, MessageSource};
use render::glyph_atlas::GlyphAtlas;
use render::renderer::Renderer;
use render::screen::{Content, PlayView, draw_screen};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::key_map::{Action, action_for, action_without_song};
use crate::repair_work::RepairWork;
use crate::song_files::{
    LoadedSong, RepairSource, copy_song, default_song_folder, list_folder, load_song, make_folder,
    make_song_in_folder, report_failure, save_song_as, short_name,
};
use crate::workspace::Workspace;

/// The window opens at the target screen's size: 2560 × 1440 pixels.
const WINDOW_WIDTH_PIXELS: u32 = 2560;
const WINDOW_HEIGHT_PIXELS: u32 = 1440;

/// Rows a list scrolls for one step of the wheel.
const WHEEL_ROWS_PER_STEP: isize = 3;

/// How long "Message copied" stays in the menu bar.
const COPIED_NOTICE: Duration = Duration::from_secs(2);

/// Two clicks on a row within this time open it.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// What the program was started with.
pub struct Startup {
    /// The song to open, when one was given and it read.
    pub song: Option<LoadedSong>,
    /// The song or studio given had mistakes: the files to repair.
    pub repair: Option<RepairSource>,
    /// What to say first, when the song given did not open.
    pub message: Option<(MessageKind, String)>,
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
    /// The open song, if any.
    workspace: Option<Workspace>,
    /// The song or studio being repaired; while there is one, nothing is
    /// played and nothing composed.
    repair: Option<RepairWork>,
    /// The menu bar, dialogs, and the message shown with no song open.
    shell: Shell,
    /// The folder the browser last listed, so it opens there again.
    browser_directory: Option<PathBuf>,
    /// The last frame's click regions.
    last_frame_hits: Option<Frame>,
    cursor: Option<PhysicalPosition<f64>>,
    modifiers: ModifiersState,
    last_click: Option<(Instant, HitTarget)>,
    /// When "Message copied" stops showing.
    copied_until: Option<Instant>,
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
        let attributes = Window::default_attributes()
            .with_title(window_title(
                startup
                    .song
                    .as_ref()
                    .map(|song| &song.song_file.path)
                    .or(startup.repair.as_ref().map(|source| &source.song_path)),
            ))
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
        let workspace: Option<Workspace> = startup.song.map(Workspace::open);
        let repair: Option<RepairWork> = startup.repair.map(RepairWork::new);
        let mut shell: Shell = Shell::new();
        shell.set_can_save(workspace.is_some() || repair.is_some());
        shell.set_song_open(workspace.is_some());
        if let Some((kind, text)) = startup.message {
            shell.say(kind, text);
        }
        Ok(OpenWindow {
            renderer,
            window,
            atlas,
            workspace,
            repair,
            shell,
            browser_directory: None,
            last_frame_hits: None,
            cursor: None,
            modifiers: ModifiersState::empty(),
            last_click: None,
            copied_until: None,
        })
    }
}

fn window_title(song_path: Option<&PathBuf>) -> String {
    match song_path {
        Some(path) => format!("Live Lead Sheet - {}", short_name(path)),
        None => String::from("Live Lead Sheet"),
    }
}

/// The key as the shell reads it, while a menu or dialog is open.
fn shell_key(code: KeyCode, control: bool) -> Option<ShellKey> {
    match code {
        // Ctrl+Up and Ctrl+Down move a track in the song settings.
        KeyCode::ArrowUp if control => Some(ShellKey::MoveUp),
        KeyCode::ArrowDown if control => Some(ShellKey::MoveDown),
        KeyCode::ArrowUp => Some(ShellKey::Up),
        KeyCode::ArrowDown => Some(ShellKey::Down),
        KeyCode::Enter | KeyCode::NumpadEnter => Some(ShellKey::Enter),
        KeyCode::Escape => Some(ShellKey::Escape),
        KeyCode::Backspace => Some(ShellKey::Backspace),
        KeyCode::PageUp => Some(ShellKey::PageUp),
        KeyCode::PageDown => Some(ShellKey::PageDown),
        KeyCode::Home => Some(ShellKey::Home),
        KeyCode::End => Some(ShellKey::End),
        KeyCode::F10 => Some(ShellKey::OpenMenu),
        KeyCode::ArrowLeft => Some(ShellKey::Left),
        KeyCode::ArrowRight => Some(ShellKey::Right),
        KeyCode::Delete => Some(ShellKey::Delete),
        KeyCode::KeyT => Some(ShellKey::Type),
        KeyCode::KeyC => Some(ShellKey::Clef),
        KeyCode::KeyL => Some(ShellKey::Lead),
        KeyCode::KeyN => Some(ShellKey::New),
        KeyCode::KeyV => Some(ShellKey::Layer),
        KeyCode::Tab => Some(ShellKey::Tab),
        _ => None,
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
                if open_window.ask_to_quit() {
                    event_loop.exit();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => open_window.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        text,
                        ..
                    },
                ..
            } => {
                if open_window.key(code, text.as_deref()) {
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => open_window.cursor = Some(position),
            WindowEvent::CursorLeft { .. } => open_window.cursor = None,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if open_window.click() {
                    event_loop.exit();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => open_window.wheel(delta),
            WindowEvent::Resized(size) => open_window.renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                if let Some(workspace) = open_window.workspace.as_mut() {
                    workspace.housekeeping();
                    workspace.log_new_problem();
                }
                if open_window
                    .copied_until
                    .is_some_and(|until| Instant::now() >= until)
                {
                    open_window.copied_until = None;
                    open_window.shell.set_copied(false);
                }
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
    /// A key press, with the text it typed; true when the program should end.
    fn key(&mut self, code: KeyCode, text: Option<&str>) -> bool {
        if self.shell.is_typing() {
            // A dialog's text field: the editing keys go to it, and every
            // other key types what it makes.
            return match code {
                KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::Escape
                | KeyCode::Backspace
                | KeyCode::Delete
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::Home
                | KeyCode::End => {
                    let request: Option<ShellRequest> =
                        shell_key(code, self.modifiers.control_key())
                            .and_then(|key| self.shell.handle_key(key));
                    self.carry_out(request)
                }
                _ => {
                    if !self.modifiers.control_key()
                        && let Some(typed) = text
                    {
                        self.shell.type_text(typed);
                    }
                    false
                }
            };
        }
        if self.shell.wants_keys() {
            // A menu or dialog is open: it takes every key.
            return match shell_key(code, self.modifiers.control_key()) {
                Some(key) => {
                    let request: Option<ShellRequest> = self.shell.handle_key(key);
                    self.carry_out(request)
                }
                None => false,
            };
        }
        if self.repair.is_some() {
            return self.repair_key(code, text);
        }
        let action: Option<Action> = match &self.workspace {
            Some(workspace) => action_for(code, self.modifiers, &workspace.session),
            None => action_without_song(code, self.modifiers),
        };
        match action {
            Some(action) => self.act(action),
            None => false,
        }
    }

    /// Carries out an action; true when the program should end.
    fn act(&mut self, action: Action) -> bool {
        match action {
            Action::Session(command) => {
                if let Some(workspace) = self.workspace.as_mut() {
                    workspace.handle(command);
                }
                false
            }
            Action::PlayOrStop => {
                self.with_workspace(|workspace| workspace.play_or_stop(false, false));
                false
            }
            Action::PlayFromStart => {
                self.with_workspace(|workspace| workspace.play_or_stop(true, false));
                false
            }
            Action::LoopEntry => {
                self.with_workspace(|workspace| workspace.play_or_stop(false, true));
                false
            }
            Action::Save => {
                self.save();
                false
            }
            Action::Quit => self.ask_to_quit(),
            Action::OpenSong => {
                self.ask_to_open();
                false
            }
            Action::NewSong => {
                self.ask_to_start_new();
                false
            }
            Action::SaveAs => {
                self.start_save_as_browsing();
                false
            }
            Action::OpenMenu => {
                self.shell.handle_key(ShellKey::OpenMenu);
                false
            }
            Action::OpenSongSettings => {
                self.open_song_settings();
                false
            }
        }
    }

    /// Opens the song settings dialog on the open song.
    fn open_song_settings(&mut self) {
        if let Some(workspace) = self.workspace.as_ref() {
            self.shell.open_settings(workspace.settings_snapshot());
        }
    }

    fn with_workspace(&mut self, action: impl FnOnce(&mut Workspace)) {
        if let Some(workspace) = self.workspace.as_mut() {
            action(workspace);
        }
    }

    /// What the shell asked for; true when the program should end.
    fn carry_out(&mut self, request: Option<ShellRequest>) -> bool {
        let Some(request) = request else {
            return false;
        };
        match request {
            ShellRequest::Menu(MenuCommand::OpenSong) => self.ask_to_open(),
            ShellRequest::Menu(MenuCommand::NewSong) => self.ask_to_start_new(),
            ShellRequest::MakeFolder {
                parent,
                name,
                copy_from,
            } => self.make_folder_or_copy(&parent, &name, copy_from.as_deref()),
            ShellRequest::NewSongIn(folder) => self.new_song_in(&folder),
            ShellRequest::SaveAsIn(folder) => self.save_as_in(&folder),
            ShellRequest::Menu(MenuCommand::SaveSongAs) => self.start_save_as_browsing(),
            ShellRequest::Menu(MenuCommand::SaveSong) => self.save(),
            ShellRequest::Menu(MenuCommand::Quit) => return self.ask_to_quit(),
            ShellRequest::Menu(MenuCommand::SongSettings) => self.open_song_settings(),
            ShellRequest::Settings(command) => {
                if let Some(workspace) = self.workspace.as_mut() {
                    workspace.apply_settings(command);
                    self.shell.refresh_settings(workspace.settings_snapshot());
                    if let Some(message) = workspace.session.message() {
                        self.shell.settings_say(message.kind, message.text.clone());
                    }
                }
            }
            ShellRequest::ListFolder(directory) => self.list_into_browser(&directory),
            ShellRequest::OpenSong(path) => self.open_song(&path),
            ShellRequest::Confirmed(Pending::Quit) => return true,
            ShellRequest::Confirmed(Pending::OpenSong) => self.start_browsing(),
            ShellRequest::Confirmed(Pending::NewSong) => self.start_new_song_browsing(),
            ShellRequest::Confirmed(Pending::TossAll) => {
                if let Some(work) = self.repair.as_mut() {
                    work.repair.toss_all();
                }
            }
        }
        false
    }

    /// Whether to quit now: at once when everything is saved, else after
    /// the musician says so, so no edit is lost by one keypress.
    fn ask_to_quit(&mut self) -> bool {
        let Some(name) = self.unsaved_name() else {
            return true;
        };
        self.shell.ask(Confirmation {
            question: format!("Unsaved edits in {name}. Quit without saving?"),
            yes_label: String::from("Quit"),
            no_label: String::from("Stay"),
            pending: Pending::Quit,
        });
        false
    }

    /// Choosing another song: asks first when the open one has unsaved
    /// edits.
    fn ask_to_open(&mut self) {
        if let Some(name) = self.unsaved_name() {
            self.shell.ask(Confirmation {
                question: format!("Unsaved edits in {name}. Open another song without saving?"),
                yes_label: String::from("Open"),
                no_label: String::from("Stay"),
                pending: Pending::OpenSong,
            });
        } else {
            self.start_browsing();
        }
    }

    /// Starting a new song closes the open one: asks first when it has
    /// unsaved edits.
    fn ask_to_start_new(&mut self) {
        if let Some(name) = self.unsaved_name() {
            self.shell.ask(Confirmation {
                question: format!("Unsaved edits in {name}. Start a new song without saving?"),
                yes_label: String::from("New song"),
                no_label: String::from("Stay"),
                pending: Pending::NewSong,
            });
        } else {
            self.start_new_song_browsing();
        }
    }

    fn start_new_song_browsing(&mut self) {
        let directory: PathBuf = self
            .browser_directory
            .clone()
            .unwrap_or_else(default_song_folder);
        self.shell.open_new_song_browser(directory.clone());
        self.list_into_browser(&directory);
    }

    /// A folder named in the prompt over the browser: made, and the browser
    /// goes into it; or, for a copy, the song folder is copied under the
    /// name and the copy opens. When it cannot be done the prompt stays
    /// open and says why.
    fn make_folder_or_copy(&mut self, parent: &Path, name: &str, copy_from: Option<&Path>) {
        match copy_from {
            Some(source) => match copy_song(source, parent, name) {
                Ok(song_path) => {
                    self.shell.close_dialog();
                    self.open_song(&song_path);
                }
                Err(problem) => self.shell.new_folder_say(problem),
            },
            None => match make_folder(parent, name) {
                Ok(folder) => {
                    self.shell.reopen_browser_in(folder.clone());
                    self.list_into_browser(&folder);
                }
                Err(problem) => self.shell.new_folder_say(problem),
            },
        }
    }

    fn start_save_as_browsing(&mut self) {
        if self.workspace.is_none() {
            return;
        }
        let directory: PathBuf = self
            .browser_directory
            .clone()
            .unwrap_or_else(default_song_folder);
        self.shell.open_save_as_browser(directory.clone());
        self.list_into_browser(&directory);
    }

    /// Writes the open song and studio, with their unsaved edits, to the
    /// chosen folder and opens the result. The old files stay as saved.
    fn save_as_in(&mut self, folder: &Path) {
        let Some(workspace) = self.workspace.as_ref() else {
            return;
        };
        let result: Result<PathBuf, String> =
            workspace
                .texts_for_save_as()
                .and_then(|(song_text, studio_text)| {
                    save_song_as(
                        folder,
                        workspace.song_path(),
                        &song_text,
                        studio_text.as_deref(),
                    )
                });
        match result {
            Ok(song_path) => {
                self.shell.close_dialog();
                self.open_song(&song_path);
            }
            Err(problem) => self.shell.browser_problem(problem),
        }
    }

    /// The folder chosen in the browser becomes the new song's folder; when
    /// it cannot, the browser stays and says why.
    fn new_song_in(&mut self, folder: &Path) {
        match make_song_in_folder(folder) {
            Ok(song_path) => {
                self.shell.close_dialog();
                self.open_song(&song_path);
            }
            Err(problem) => self.shell.browser_problem(problem),
        }
    }

    /// The song with edits not yet saved: the open song, or the one being
    /// repaired.
    fn unsaved_name(&self) -> Option<String> {
        if let Some(workspace) = self.workspace.as_ref().filter(|w| w.has_unsaved_edits()) {
            return Some(short_name(workspace.song_path()));
        }
        self.repair
            .as_ref()
            .filter(|work| work.repair.changed())
            .map(|work| short_name(&work.song_path))
    }

    /// Ctrl+S or File, Save: the open song, or the repaired files.
    fn save(&mut self) {
        if self.repair.is_some() {
            self.save_repair();
        } else {
            self.with_workspace(Workspace::save);
        }
    }

    fn start_browsing(&mut self) {
        let directory: PathBuf = self
            .browser_directory
            .clone()
            .unwrap_or_else(default_song_folder);
        self.shell.open_browser(directory.clone());
        self.list_into_browser(&directory);
    }

    fn list_into_browser(&mut self, directory: &Path) {
        let listing: Result<Vec<_>, String> = list_folder(directory);
        let Some(browser) = self.shell.browser_mut() else {
            return;
        };
        match listing {
            Ok(entries) => {
                browser.set_listing(directory.to_path_buf(), entries);
                self.browser_directory = Some(directory.to_path_buf());
            }
            Err(problem) => browser.set_problem(problem),
        }
    }

    /// Opens a song chosen in the browser. The song now open is closed
    /// first only when the new one reads or goes to repair; if it cannot
    /// be read at all, the old stays.
    fn open_song(&mut self, path: &Path) {
        self.open_song_with(path, None);
    }

    fn open_song_with(&mut self, path: &Path, studio_path: Option<&Path>) {
        match load_song(path, studio_path) {
            Ok(loaded) => {
                // The old workspace drops first, closing its MIDI ports and
                // silencing its notes, so the new one can open the same names.
                self.workspace = None;
                self.repair = None;
                let mut workspace: Workspace = Workspace::open(loaded);
                workspace
                    .session
                    .say(MessageKind::Done, format!("Opened {}.", short_name(path)));
                self.window
                    .set_title(&window_title(Some(workspace.song_path())));
                self.workspace = Some(workspace);
                self.shell.set_can_save(true);
                self.shell.set_song_open(true);
            }
            Err(failure) => {
                let (kind, text): (MessageKind, String) = report_failure(path, &failure);
                if let Some(source) = failure.repair {
                    self.workspace = None;
                    self.window
                        .set_title(&window_title(Some(&source.song_path)));
                    self.repair = Some(RepairWork::new(source));
                    self.shell.set_can_save(true);
                    self.shell.set_song_open(false);
                    self.shell.say(kind, text);
                    return;
                }
                match self.workspace.as_mut() {
                    Some(workspace) => workspace.session.say(kind, text),
                    None => self.shell.say(kind, text),
                }
            }
        }
    }

    /// Writes the repaired files; when both now read cleanly, opens the
    /// song.
    fn save_repair(&mut self) {
        let Some(work) = self.repair.as_mut() else {
            return;
        };
        if let Err(problem) = work.save() {
            self.shell.say(MessageKind::Problem, problem);
            return;
        }
        if work.repair.is_clean() {
            let song_path: PathBuf = work.song_path.clone();
            let studio_path: Option<PathBuf> = work.studio_path.clone();
            self.open_song_with(&song_path, studio_path.as_deref());
        } else {
            let remaining: usize = work.repair.problems().len();
            self.shell.say(
                MessageKind::Problem,
                format!("Saved. {remaining} to repair before the song opens."),
            );
        }
    }

    /// A key on the repair screen; true when the program should end.
    fn repair_key(&mut self, code: KeyCode, text: Option<&str>) -> bool {
        let control: bool = self.modifiers.control_key();
        let Some(work) = self.repair.as_mut() else {
            return false;
        };
        let repair = &mut work.repair;
        if let Some(edit) = repair.editing_mut() {
            match code {
                KeyCode::Enter | KeyCode::NumpadEnter => repair.accept_edit(),
                KeyCode::Escape => repair.cancel_edit(),
                KeyCode::ArrowLeft => edit.field.left(),
                KeyCode::ArrowRight => edit.field.right(),
                KeyCode::Home => edit.field.home(),
                KeyCode::End => edit.field.end(),
                KeyCode::Backspace => edit.field.backspace(),
                KeyCode::Delete => edit.field.delete(),
                _ if !control => {
                    if let Some(typed) = text {
                        edit.field.insert(typed);
                    }
                }
                _ => {}
            }
            return false;
        }
        match code {
            KeyCode::ArrowUp => repair.list_mut().up(),
            KeyCode::ArrowDown => repair.list_mut().down(),
            KeyCode::PageUp => repair.list_mut().page_up(),
            KeyCode::PageDown => repair.list_mut().page_down(),
            KeyCode::Home => repair.list_mut().first(),
            KeyCode::End => repair.list_mut().last(),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::KeyE if !control => {
                repair.begin_edit();
            }
            KeyCode::KeyI if !control => repair.begin_insert(),
            KeyCode::KeyT if control => self.ask_to_toss_all(),
            KeyCode::KeyT | KeyCode::Delete => repair.toss_selected(),
            KeyCode::KeyZ if control => repair.undo(),
            KeyCode::KeyS if control => self.save_repair(),
            _ => {
                return match action_without_song(code, self.modifiers) {
                    Some(action) => self.act(action),
                    None => false,
                };
            }
        }
        false
    }

    fn ask_to_toss_all(&mut self) {
        let Some(work) = self.repair.as_ref().filter(|w| !w.repair.is_clean()) else {
            return;
        };
        let count: usize = work.repair.problems().len();
        self.shell.ask(Confirmation {
            question: format!(
                "Toss {count} line{}? Each is noted at the top of its file. Ctrl+Z brings them back.",
                if count == 1 { "" } else { "s" }
            ),
            yes_label: String::from("Toss all"),
            no_label: String::from("Keep"),
            pending: Pending::TossAll,
        });
    }

    /// Copies the whole message that was clicked to the clipboard.
    fn copy_message(&mut self, source: MessageSource) {
        let text: Option<String> = match source {
            MessageSource::Dialog => self.shell.dialog_message().map(str::to_string),
            MessageSource::Footer => match (&self.repair, &self.workspace) {
                (None, Some(workspace)) => workspace
                    .session
                    .message()
                    .map(|message| message.text.clone()),
                _ => self.shell.message().map(|(_, text)| text.clone()),
            },
        };
        let Some(text) = text else {
            return;
        };
        match crate::clipboard::copy(&text) {
            Ok(()) => {
                self.shell.set_copied(true);
                self.copied_until = Some(Instant::now() + COPIED_NOTICE);
            }
            Err(problem) => self.shell.say(MessageKind::Problem, problem),
        }
    }

    /// A click on the repair screen.
    fn repair_click(&mut self, target: RepairTarget, double: bool) {
        let Some(work) = self.repair.as_mut() else {
            return;
        };
        let repair = &mut work.repair;
        match target {
            RepairTarget::Row(index) => {
                repair.list_mut().select(index);
                if double && repair.editing().is_none() {
                    repair.begin_edit();
                }
            }
            RepairTarget::Page { up } => repair.list_mut().scroll_page(up),
            RepairTarget::Edit => repair.begin_edit(),
            RepairTarget::Insert => repair.begin_insert(),
            RepairTarget::Toss => repair.toss_selected(),
            RepairTarget::TossAll => self.ask_to_toss_all(),
            RepairTarget::Undo => repair.undo(),
            RepairTarget::Save => self.save_repair(),
            RepairTarget::Accept => repair.accept_edit(),
            RepairTarget::Cancel => repair.cancel_edit(),
        }
    }

    /// A click; true when the program should end.
    fn click(&mut self) -> bool {
        let (Some(cursor), Some(frame)) = (self.cursor, &self.last_frame_hits) else {
            return false;
        };
        // Window positions are far below 2^24, so f32 holds them to well
        // within a pixel.
        #[allow(clippy::cast_possible_truncation)]
        let Some(target) = frame.hit_at(cursor.x as f32, cursor.y as f32) else {
            return false;
        };
        let double: bool = self
            .last_click
            .is_some_and(|(when, previous)| previous == target && when.elapsed() < DOUBLE_CLICK);
        self.last_click = Some((Instant::now(), target));
        if let HitTarget::Shell(shell_target) = target {
            let request: Option<ShellRequest> = self.shell.click(shell_target, double);
            return self.carry_out(request);
        }
        if let HitTarget::CopyMessage(source) = target {
            self.copy_message(source);
            return false;
        }
        if let HitTarget::Repair(repair_target) = target {
            self.repair_click(repair_target, double);
            return false;
        }
        let Some(workspace) = self.workspace.as_mut() else {
            return false;
        };
        let command: Command = match target {
            HitTarget::Shell(_) | HitTarget::Repair(_) | HitTarget::CopyMessage(_) => return false,
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
                workspace.handle(Command::SelectLibraryRow(row));
                Command::Commit
            }
            HitTarget::LibraryRow(row) => Command::SelectLibraryRow(row),
            HitTarget::LineForm(track) => {
                // Selecting the track again would forget the question asked
                // by the first click, so only a different track is selected.
                if workspace.session.selection.track != track {
                    workspace.handle(Command::SelectTrack(track));
                }
                Command::ConvertLineForm
            }
        };
        workspace.handle(command);
        false
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
        if self.shell.wants_keys() {
            // Over a dialog the wheel scrolls its list, three rows a step;
            // the song underneath does not move.
            let over: Option<composer::shell::ShellTarget> =
                match (self.cursor, &self.last_frame_hits) {
                    #[allow(clippy::cast_possible_truncation)]
                    (Some(cursor), Some(frame)) => {
                        match frame.hit_at(cursor.x as f32, cursor.y as f32) {
                            Some(HitTarget::Shell(target)) => Some(target),
                            _ => None,
                        }
                    }
                    _ => None,
                };
            self.shell.scroll(
                if up {
                    -WHEEL_ROWS_PER_STEP
                } else {
                    WHEEL_ROWS_PER_STEP
                },
                over,
            );
            return;
        }
        if let Some(work) = self.repair.as_mut() {
            work.repair.list_mut().scroll_by(if up {
                -WHEEL_ROWS_PER_STEP
            } else {
                WHEEL_ROWS_PER_STEP
            });
            return;
        }
        let control: bool = self.modifiers.control_key();
        let over_library: bool = match (self.cursor, &self.last_frame_hits) {
            #[allow(clippy::cast_possible_truncation)]
            (Some(cursor), Some(frame)) => matches!(
                frame.hit_at(cursor.x as f32, cursor.y as f32),
                Some(HitTarget::LibraryRow(_) | HitTarget::LibraryTab(_))
            ),
            _ => false,
        };
        let Some(workspace) = self.workspace.as_mut() else {
            return;
        };
        if control {
            // Like Ctrl+= and Ctrl+-: wider bars, fewer across, or more.
            workspace.handle(Command::StaffBarWidth(if up { 1 } else { -1 }));
            return;
        }
        if over_library {
            if workspace.session.pane != composer::session::Pane::Library {
                workspace.handle(Command::SwitchPane);
            }
            workspace.handle(if up { Command::Up } else { Command::Down });
        }
    }

    fn draw(&mut self) -> Result<(), String> {
        let size: PhysicalSize<u32> = self.window.inner_size();
        // Window sizes are far below 2^24, so the conversion to f32 is exact.
        #[allow(clippy::cast_precision_loss)]
        let (width_pixels, height_pixels): (f32, f32) = (size.width as f32, size.height as f32);
        let play: PlayView = self.workspace.as_ref().map_or_else(
            || PlayView {
                playing: false,
                looping: false,
                position_ticks: None,
                status: String::new(),
            },
            Workspace::play_view,
        );
        let content: Content<'_> = match (&self.repair, &self.workspace) {
            (Some(work), _) => Content::Repair(&work.repair),
            (None, Some(workspace)) => Content::Song(&workspace.session),
            (None, None) => Content::Empty,
        };
        let frame: Frame = draw_screen(
            &content,
            &self.shell,
            &play,
            &self.atlas,
            width_pixels,
            height_pixels,
        )
        .map_err(|e| e.to_string())?;
        self.renderer
            .draw_frame(&frame)
            .map_err(|e| e.to_string())?;
        self.last_frame_hits = Some(frame);
        Ok(())
    }
}
