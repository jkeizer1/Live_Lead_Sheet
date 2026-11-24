//! What sits around the song: the menu bar, the dialog on top of the
//! screen (the folder browser, a question to answer), and the message shown
//! while no song is open. Keys and clicks arrive here first whenever a menu
//! or a dialog is open; what they ask of the program leaves as a
//! [`ShellRequest`] for the app to carry out. No window, no file.

use std::path::PathBuf;

use crate::folder_browser::{BrowserPurpose, BrowserRequest, FolderBrowser};
use crate::session::MessageKind;
use crate::settings_view::{
    SettingsButton, SettingsCommand, SettingsKey, SettingsOutcome, SettingsSnapshot, SettingsView,
};
use crate::text_field::TextField;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuId {
    File,
}

impl MenuId {
    pub const ALL: [MenuId; 1] = [MenuId::File];

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            MenuId::File => "File",
        }
    }

    #[must_use]
    pub const fn items(self) -> &'static [MenuCommand] {
        match self {
            MenuId::File => &[
                MenuCommand::NewSong,
                MenuCommand::OpenSong,
                MenuCommand::SaveSong,
                MenuCommand::SaveSongAs,
                MenuCommand::SongSettings,
                MenuCommand::Quit,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    NewSong,
    OpenSong,
    SaveSong,
    SaveSongAs,
    Quit,
    SongSettings,
}

impl MenuCommand {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            MenuCommand::NewSong => "New song...",
            MenuCommand::OpenSong => "Open song...",
            MenuCommand::SaveSong => "Save",
            MenuCommand::SaveSongAs => "Save as...",
            MenuCommand::Quit => "Quit",
            MenuCommand::SongSettings => "Settings...",
        }
    }

    /// The key that does the same without the menu.
    #[must_use]
    pub const fn shortcut(self) -> &'static str {
        match self {
            MenuCommand::NewSong => "Ctrl+N",
            MenuCommand::OpenSong => "Ctrl+O",
            MenuCommand::SaveSong => "Ctrl+S",
            MenuCommand::SaveSongAs => "Ctrl+Shift+S",
            MenuCommand::Quit => "Ctrl+Q",
            MenuCommand::SongSettings => "F6",
        }
    }

    /// Whether it works with no song open.
    #[must_use]
    pub const fn enabled(self, can_save: bool, song_open: bool) -> bool {
        match self {
            MenuCommand::NewSong | MenuCommand::OpenSong | MenuCommand::Quit => true,
            MenuCommand::SaveSong => can_save,
            MenuCommand::SongSettings | MenuCommand::SaveSongAs => song_open,
        }
    }
}

/// What a "yes" goes on to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    OpenSong,
    /// Go on to choosing the folder of a new song.
    NewSong,
    Quit,
    /// Remove every problem line in the repair view.
    TossAll,
}

/// A question with two answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmation {
    pub question: String,
    pub yes_label: String,
    pub no_label: String,
    pub pending: Pending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    Browser(FolderBrowser),
    Confirm(Confirmation),
    /// Boxed: the dialog holds both panes' state, much larger than the others.
    Settings(Box<SettingsView>),
    NewFolder(Box<NewFolderPrompt>),
}

/// The name of a new folder being typed, in the folder the browser shows.
/// The browser waits under it, to come back to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFolderPrompt {
    pub browser: FolderBrowser,
    /// The song folder being copied, when the name is for a copy.
    pub copy_from: Option<PathBuf>,
    pub field: TextField,
    /// Why the last try failed, in a sentence.
    pub message: Option<String>,
}

/// The keys the shell reads while a menu or dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKey {
    Up,
    Down,
    Enter,
    Escape,
    Backspace,
    PageUp,
    PageDown,
    Home,
    End,
    /// F10: opens the first menu.
    OpenMenu,
    Left,
    Right,
    Delete,
    /// The letters the song settings dialog reads as commands (T, C, L, N).
    Type,
    Clef,
    Lead,
    New,
    /// U: use the folder the browser shows.
    Use,
    /// Moves between the song settings pane and the studio settings pane.
    Tab,
    /// Ctrl+Up and Ctrl+Down in the song settings: move the chosen track.
    MoveUp,
    MoveDown,
    /// V: name a voice layer of the chosen track.
    Layer,
}

/// A place on the screen the shell owns, as a click can land on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellTarget {
    Menu(MenuId),
    MenuItem(MenuCommand),
    BrowserRow(usize),
    /// The browser's button that chooses the folder shown (new song).
    BrowserChoose,
    /// The browser's button that asks for a new folder's name.
    BrowserNewFolder,
    /// The browser's button that copies the chosen song folder.
    BrowserCopy,
    NewFolderAccept,
    NewFolderCancel,
    /// The browser's scroll bar, above (`up`) or below its thumb: one page.
    BrowserPage {
        up: bool,
    },
    BrowserCancel,
    SettingsRow(usize),
    /// A row of the studio pane of the same dialog.
    StudioRow(usize),
    /// Anywhere in the song pane's list (under its rows): gives it the keys.
    SongPane,
    /// Anywhere in the studio pane's list.
    StudioPane,
    /// The studio pane's scroll bar, above (`up`) or below its thumb.
    StudioPage {
        up: bool,
    },
    /// The song settings list's scroll bar, above (`up`) or below its thumb.
    SettingsPage {
        up: bool,
    },
    SettingsButton(SettingsButton),
    /// One option of the type or clef choice along the dialog's bottom.
    SettingsPick(usize),
    ConfirmYes,
    ConfirmNo,
    /// Anywhere else while a menu or dialog is open.
    Outside,
}

/// What the shell asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellRequest {
    /// A menu item was chosen (the menu has closed).
    Menu(MenuCommand),
    /// List this folder for the browser.
    ListFolder(PathBuf),
    /// Open this song (the browser has closed).
    OpenSong(PathBuf),
    /// "Yes" was answered (the question has closed).
    Confirmed(Pending),
    /// The song settings dialog asks for a change to the song.
    Settings(SettingsCommand),
    /// Make the folder `name` inside `parent`; when `copy_from` is a song
    /// folder, make it a copy of that song, with its files renamed.
    MakeFolder {
        parent: PathBuf,
        name: String,
        copy_from: Option<PathBuf>,
    },
    /// Make the new song's files in this folder, and open it.
    NewSongIn(PathBuf),
    /// Save the open song, with its studio, in this folder, and open that.
    SaveAsIn(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenMenu {
    id: MenuId,
    /// The item the keys are on.
    cursor: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    open_menu: Option<OpenMenu>,
    modal: Option<Modal>,
    can_save: bool,
    /// A song is open for editing (not being repaired).
    song_open: bool,
    /// A message was just copied to the clipboard (shown for a moment).
    copied: bool,
    message: Option<(MessageKind, String)>,
}

impl Default for Shell {
    fn default() -> Shell {
        Shell::new()
    }
}

impl Shell {
    #[must_use]
    pub fn new() -> Shell {
        Shell {
            open_menu: None,
            modal: None,
            can_save: false,
            song_open: false,
            copied: false,
            message: None,
        }
    }

    pub fn set_can_save(&mut self, can_save: bool) {
        self.can_save = can_save;
    }

    pub fn set_copied(&mut self, copied: bool) {
        self.copied = copied;
    }

    #[must_use]
    pub fn copied(&self) -> bool {
        self.copied
    }

    /// The message shown in the open settings dialog, if any.
    #[must_use]
    pub fn dialog_message(&self) -> Option<&str> {
        match &self.modal {
            Some(Modal::Settings(view)) => view.message().map(|(_, text)| text.as_str()),
            _ => None,
        }
    }

    pub fn set_song_open(&mut self, song_open: bool) {
        self.song_open = song_open;
    }

    #[must_use]
    pub fn song_open(&self) -> bool {
        self.song_open
    }

    #[must_use]
    pub fn can_save(&self) -> bool {
        self.can_save
    }

    /// The message shown while no song is open.
    #[must_use]
    pub fn message(&self) -> Option<&(MessageKind, String)> {
        self.message.as_ref()
    }

    pub fn say(&mut self, kind: MessageKind, text: String) {
        self.message = Some((kind, text));
    }

    #[must_use]
    pub fn modal(&self) -> Option<&Modal> {
        self.modal.as_ref()
    }

    pub fn browser_mut(&mut self) -> Option<&mut FolderBrowser> {
        match self.modal.as_mut() {
            Some(Modal::Browser(browser)) => Some(browser),
            _ => None,
        }
    }

    pub fn settings_mut(&mut self) -> Option<&mut SettingsView> {
        match self.modal.as_mut() {
            Some(Modal::Settings(view)) => Some(view),
            _ => None,
        }
    }

    /// Opens the song settings dialog.
    pub fn open_settings(&mut self, snapshot: SettingsSnapshot) {
        self.open_menu = None;
        self.modal = Some(Modal::Settings(Box::new(SettingsView::new(snapshot))));
    }

    /// The song changed under the open settings dialog.
    pub fn refresh_settings(&mut self, snapshot: SettingsSnapshot) {
        if let Some(view) = self.settings_mut() {
            view.refresh(snapshot);
        }
    }

    /// What the last settings change said, shown in the dialog.
    pub fn settings_say(&mut self, kind: MessageKind, text: String) {
        if let Some(view) = self.settings_mut() {
            view.say(kind, text);
        }
    }

    /// Whether typed characters go to a text field in a dialog (otherwise
    /// the letters are commands).
    #[must_use]
    pub fn is_typing(&self) -> bool {
        match &self.modal {
            Some(Modal::Settings(view)) => view.is_typing(),
            Some(Modal::NewFolder(_)) => true,
            _ => false,
        }
    }

    pub fn type_text(&mut self, text: &str) {
        if let Some(Modal::NewFolder(prompt)) = &mut self.modal {
            prompt.field.insert(text);
        } else if let Some(view) = self.settings_mut() {
            view.type_text(text);
        }
    }

    /// Opens the folder browser to choose where a new song's folder goes.
    pub fn open_new_song_browser(&mut self, directory: PathBuf) {
        self.open_menu = None;
        self.modal = Some(Modal::Browser(FolderBrowser::for_new_song(directory)));
    }

    /// Opens the folder browser to choose where the open song is saved as.
    pub fn open_save_as_browser(&mut self, directory: PathBuf) {
        self.open_menu = None;
        self.modal = Some(Modal::Browser(FolderBrowser::choosing_folder(
            directory,
            BrowserPurpose::ChooseFolderForSaveAs,
        )));
    }

    /// A folder was made from the name prompt: the browser comes back, in
    /// the new folder, for the same purpose.
    pub fn reopen_browser_in(&mut self, folder: PathBuf) {
        if let Some(Modal::NewFolder(prompt)) = &self.modal {
            let purpose: BrowserPurpose = prompt.browser.purpose();
            self.modal = Some(Modal::Browser(FolderBrowser::choosing_folder(
                folder, purpose,
            )));
        }
    }

    /// The name prompt of a new folder, when open.
    #[must_use]
    pub fn new_folder_prompt(&self) -> Option<&NewFolderPrompt> {
        match &self.modal {
            Some(Modal::NewFolder(prompt)) => Some(prompt),
            _ => None,
        }
    }

    /// Why the folder could not be made; the prompt stays open.
    pub fn new_folder_say(&mut self, text: String) {
        if let Some(Modal::NewFolder(prompt)) = &mut self.modal {
            prompt.message = Some(text);
        }
    }

    /// Closes the open dialog.
    pub fn close_dialog(&mut self) {
        self.modal = None;
    }

    /// The new song's folder could not be used: the browser says why.
    pub fn browser_problem(&mut self, text: String) {
        if let Some(browser) = self.browser_mut() {
            browser.set_problem(text);
        }
    }

    fn new_folder_key(&mut self, key: ShellKey) -> Option<ShellRequest> {
        let Some(Modal::NewFolder(prompt)) = &mut self.modal else {
            return None;
        };
        match key {
            ShellKey::Enter => {
                return Some(ShellRequest::MakeFolder {
                    parent: prompt.browser.directory().to_path_buf(),
                    name: prompt.field.text().to_string(),
                    copy_from: prompt.copy_from.clone(),
                });
            }
            ShellKey::Escape => {
                // Back to the browser it came from.
                let browser: FolderBrowser = prompt.browser.clone();
                self.modal = Some(Modal::Browser(browser));
            }
            ShellKey::Left => prompt.field.left(),
            ShellKey::Right => prompt.field.right(),
            ShellKey::Home => prompt.field.home(),
            ShellKey::End => prompt.field.end(),
            ShellKey::Backspace => prompt.field.backspace(),
            ShellKey::Delete => prompt.field.delete(),
            _ => {}
        }
        None
    }

    /// The open menu and the item the keys are on.
    #[must_use]
    pub fn open_menu(&self) -> Option<(MenuId, usize)> {
        self.open_menu.map(|open: OpenMenu| (open.id, open.cursor))
    }

    /// Whether keys go to the shell: a menu or a dialog is open.
    #[must_use]
    pub fn wants_keys(&self) -> bool {
        self.open_menu.is_some() || self.modal.is_some()
    }

    /// Opens the folder browser on `directory`; the app then lists it.
    pub fn open_browser(&mut self, directory: PathBuf) {
        self.open_menu = None;
        self.modal = Some(Modal::Browser(FolderBrowser::new(directory)));
    }

    pub fn ask(&mut self, confirmation: Confirmation) {
        self.open_menu = None;
        self.modal = Some(Modal::Confirm(confirmation));
    }

    /// The wheel over a dialog: `rows` toward the bottom (negative: toward
    /// the top) of the folder browser's list.
    ///
    /// In the settings dialog the pane under the pointer scrolls (`over` is
    /// what the pointer is on); off both panes, the one that has the keys.
    pub fn scroll(&mut self, rows: isize, over: Option<ShellTarget>) {
        if let Some(browser) = self.browser_mut() {
            browser.scroll_by(rows);
        }
        if let Some(view) = self.settings_mut() {
            let pane: crate::settings_view::Pane = match over {
                Some(
                    ShellTarget::SettingsRow(_)
                    | ShellTarget::SettingsPage { .. }
                    | ShellTarget::SongPane,
                ) => crate::settings_view::Pane::Song,
                Some(
                    ShellTarget::StudioRow(_)
                    | ShellTarget::StudioPage { .. }
                    | ShellTarget::StudioPane,
                ) => crate::settings_view::Pane::Studio,
                _ => view.pane(),
            };
            view.scroll_pane(pane, rows);
        }
    }

    pub fn close(&mut self) {
        self.open_menu = None;
        self.modal = None;
    }

    pub fn handle_key(&mut self, key: ShellKey) -> Option<ShellRequest> {
        match &mut self.modal {
            Some(Modal::Browser(browser)) => {
                let request: Option<BrowserRequest> = match key {
                    ShellKey::Up => {
                        browser.up();
                        None
                    }
                    ShellKey::Down => {
                        browser.down();
                        None
                    }
                    ShellKey::Enter => browser.enter(),
                    ShellKey::PageUp => {
                        browser.page_up();
                        None
                    }
                    ShellKey::PageDown => {
                        browser.page_down();
                        None
                    }
                    ShellKey::Home => {
                        browser.first();
                        None
                    }
                    ShellKey::End => {
                        browser.last();
                        None
                    }
                    ShellKey::Backspace => browser.parent(),
                    ShellKey::Escape => {
                        self.modal = None;
                        None
                    }
                    ShellKey::New => browser.new_folder_here(),
                    ShellKey::Use => browser.choose_here(),
                    ShellKey::Clef => browser.copy_selected(),
                    ShellKey::OpenMenu
                    | ShellKey::Left
                    | ShellKey::Right
                    | ShellKey::Delete
                    | ShellKey::Type
                    | ShellKey::Lead
                    | ShellKey::MoveUp
                    | ShellKey::MoveDown
                    | ShellKey::Layer
                    | ShellKey::Tab => None,
                };
                return self.browser_request(request);
            }
            Some(Modal::Confirm(_)) => {
                return match key {
                    ShellKey::Enter => self.answer(true),
                    ShellKey::Escape => self.answer(false),
                    _ => None,
                };
            }
            Some(Modal::NewFolder(_)) => return self.new_folder_key(key),
            Some(Modal::Settings(view)) => {
                let key: Option<SettingsKey> = match key {
                    ShellKey::Up => Some(SettingsKey::Up),
                    ShellKey::Down => Some(SettingsKey::Down),
                    ShellKey::PageUp => Some(SettingsKey::PageUp),
                    ShellKey::PageDown => Some(SettingsKey::PageDown),
                    ShellKey::Home => Some(SettingsKey::First),
                    ShellKey::End => Some(SettingsKey::Last),
                    ShellKey::Enter => Some(SettingsKey::Enter),
                    ShellKey::Escape => Some(SettingsKey::Escape),
                    ShellKey::Left => Some(SettingsKey::Left),
                    ShellKey::Right => Some(SettingsKey::Right),
                    ShellKey::Backspace => Some(SettingsKey::Backspace),
                    ShellKey::Delete => Some(SettingsKey::Delete),
                    ShellKey::Type => Some(SettingsKey::Type),
                    ShellKey::Clef => Some(SettingsKey::Clef),
                    ShellKey::Lead => Some(SettingsKey::Lead),
                    ShellKey::New => Some(SettingsKey::New),
                    ShellKey::Tab => Some(SettingsKey::Tab),
                    ShellKey::MoveUp => Some(SettingsKey::MoveUp),
                    ShellKey::MoveDown => Some(SettingsKey::MoveDown),
                    ShellKey::Layer => Some(SettingsKey::Layer),
                    ShellKey::OpenMenu | ShellKey::Use => None,
                };
                let outcome: SettingsOutcome =
                    key.map_or(SettingsOutcome::Nothing, |key| view.key(key));
                return self.settings_outcome(outcome);
            }
            None => {}
        }
        if let Some(open) = self.open_menu {
            return self.menu_key(open, key);
        }
        if key == ShellKey::OpenMenu {
            self.open_menu = Some(OpenMenu {
                id: MenuId::File,
                cursor: 0,
            });
        }
        None
    }

    pub fn click(&mut self, target: ShellTarget, double: bool) -> Option<ShellRequest> {
        match target {
            ShellTarget::Menu(id) => {
                self.open_menu = match self.open_menu {
                    Some(open) if open.id == id => None,
                    _ => Some(OpenMenu { id, cursor: 0 }),
                };
                None
            }
            ShellTarget::MenuItem(command) => self.run_menu_item(command),
            ShellTarget::BrowserRow(index) => {
                let Some(Modal::Browser(browser)) = &mut self.modal else {
                    return None;
                };
                browser.select(index);
                let request: Option<BrowserRequest> = if double { browser.enter() } else { None };
                self.browser_request(request)
            }
            ShellTarget::BrowserPage { up } => {
                if let Some(browser) = self.browser_mut() {
                    browser.scroll_page(up);
                }
                None
            }
            // Each is drawn only while its own dialog is open.
            ShellTarget::BrowserCopy => {
                let request: Option<BrowserRequest> = self
                    .browser_mut()
                    .and_then(|browser| browser.copy_selected());
                self.browser_request(request)
            }
            ShellTarget::BrowserNewFolder => {
                let request: Option<BrowserRequest> = self
                    .browser_mut()
                    .and_then(|browser| browser.new_folder_here());
                self.browser_request(request)
            }
            ShellTarget::BrowserChoose => {
                let request: Option<BrowserRequest> =
                    self.browser_mut().and_then(|browser| browser.choose_here());
                self.browser_request(request)
            }
            ShellTarget::NewFolderAccept => self.new_folder_key(ShellKey::Enter),
            ShellTarget::NewFolderCancel => self.new_folder_key(ShellKey::Escape),
            ShellTarget::BrowserCancel | ShellTarget::ConfirmNo => self.answer(false),
            ShellTarget::ConfirmYes => self.answer(true),
            ShellTarget::SongPane | ShellTarget::StudioPane => {
                if let Some(view) = self.settings_mut() {
                    view.set_pane(if target == ShellTarget::SongPane {
                        crate::settings_view::Pane::Song
                    } else {
                        crate::settings_view::Pane::Studio
                    });
                }
                None
            }
            ShellTarget::SettingsRow(_)
            | ShellTarget::StudioRow(_)
            | ShellTarget::SettingsButton(_)
            | ShellTarget::SettingsPick(_) => self.click_in_settings(target, double),
            ShellTarget::StudioPage { up } => {
                if let Some(view) = self.settings_mut() {
                    view.studio_list_mut().scroll_page(up);
                }
                None
            }
            ShellTarget::SettingsPage { up } => {
                if let Some(view) = self.settings_mut() {
                    view.list_mut().scroll_page(up);
                }
                None
            }
            ShellTarget::Outside => {
                // A dialog is answered on its own buttons; a menu closes.
                if self.modal.is_none() {
                    self.open_menu = None;
                }
                None
            }
        }
    }

    /// A click on a row, a button or an option of the settings dialog.
    fn click_in_settings(&mut self, target: ShellTarget, double: bool) -> Option<ShellRequest> {
        let view: &mut SettingsView = self.settings_mut()?;
        let outcome: SettingsOutcome = match target {
            ShellTarget::SettingsRow(row) => view.click_row(row, double),
            ShellTarget::StudioRow(row) => view.click_studio_row(row, double),
            ShellTarget::SettingsButton(button) => view.click_button(button),
            ShellTarget::SettingsPick(option) => view.click_pick(option, double),
            _ => SettingsOutcome::Nothing,
        };
        self.settings_outcome(outcome)
    }

    fn menu_key(&mut self, open: OpenMenu, key: ShellKey) -> Option<ShellRequest> {
        let items: &[MenuCommand] = open.id.items();
        match key {
            ShellKey::Up => {
                self.open_menu = Some(OpenMenu {
                    cursor: open.cursor.saturating_sub(1),
                    ..open
                });
                None
            }
            ShellKey::Down => {
                self.open_menu = Some(OpenMenu {
                    cursor: (open.cursor + 1).min(items.len().saturating_sub(1)),
                    ..open
                });
                None
            }
            ShellKey::Enter => items
                .get(open.cursor)
                .and_then(|command: &MenuCommand| self.run_menu_item(*command)),
            ShellKey::Escape | ShellKey::OpenMenu => {
                self.open_menu = None;
                None
            }
            ShellKey::Backspace
            | ShellKey::PageUp
            | ShellKey::PageDown
            | ShellKey::Home
            | ShellKey::End
            | ShellKey::Left
            | ShellKey::Right
            | ShellKey::Delete
            | ShellKey::Type
            | ShellKey::Clef
            | ShellKey::Lead
            | ShellKey::New
            | ShellKey::Use
            | ShellKey::MoveUp
            | ShellKey::MoveDown
            | ShellKey::Layer
            | ShellKey::Tab => None,
        }
    }

    fn settings_outcome(&mut self, outcome: SettingsOutcome) -> Option<ShellRequest> {
        match outcome {
            SettingsOutcome::Nothing => None,
            SettingsOutcome::Command(command) => Some(ShellRequest::Settings(command)),
            SettingsOutcome::Close => {
                self.modal = None;
                None
            }
        }
    }

    fn run_menu_item(&mut self, command: MenuCommand) -> Option<ShellRequest> {
        if !command.enabled(self.can_save, self.song_open) {
            return None;
        }
        self.open_menu = None;
        Some(ShellRequest::Menu(command))
    }

    /// Closes the question or browser; "yes" on a question carries on.
    fn answer(&mut self, yes: bool) -> Option<ShellRequest> {
        let modal: Option<Modal> = self.modal.take();
        match modal {
            Some(Modal::Confirm(confirmation)) if yes => {
                Some(ShellRequest::Confirmed(confirmation.pending))
            }
            _ => None,
        }
    }

    /// The name prompt over the browser, for a new folder or a copy of the
    /// song folder `copy_from` (its name, with " copy", to start from).
    fn ask_folder_name(&mut self, copy_from: Option<PathBuf>) {
        if let Some(Modal::Browser(browser)) = self.modal.take() {
            let start: String = copy_from
                .as_ref()
                .and_then(|source| source.file_name())
                .map_or_else(String::new, |name| {
                    format!("{} copy", name.to_string_lossy())
                });
            self.modal = Some(Modal::NewFolder(Box::new(NewFolderPrompt {
                browser,
                copy_from,
                field: TextField::new(&start),
                message: None,
            })));
        }
    }

    fn browser_request(&mut self, request: Option<BrowserRequest>) -> Option<ShellRequest> {
        match request? {
            BrowserRequest::List(directory) => Some(ShellRequest::ListFolder(directory)),
            BrowserRequest::Open(path) => {
                self.modal = None;
                Some(ShellRequest::OpenSong(path))
            }
            BrowserRequest::ChooseFolder(folder) => {
                let save_as: bool = matches!(
                    &self.modal,
                    Some(Modal::Browser(browser))
                        if browser.purpose() == BrowserPurpose::ChooseFolderForSaveAs
                );
                Some(if save_as {
                    ShellRequest::SaveAsIn(folder)
                } else {
                    ShellRequest::NewSongIn(folder)
                })
            }
            BrowserRequest::NewFolder => {
                self.ask_folder_name(None);
                None
            }
            BrowserRequest::CopySong(source) => {
                self.ask_folder_name(Some(source));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Confirmation, MenuCommand, MenuId, Modal, Pending, Shell, ShellKey, ShellRequest,
        ShellTarget,
    };
    use crate::folder_browser::{BrowserEntry, BrowserEntryKind};
    use std::path::PathBuf;

    fn question() -> Confirmation {
        Confirmation {
            question: String::from("Unsaved edits. Quit anyway?"),
            yes_label: String::from("Quit"),
            no_label: String::from("Stay"),
            pending: Pending::Quit,
        }
    }

    #[test]
    fn f10_opens_the_file_menu_and_enter_runs_the_item() {
        let mut shell: Shell = Shell::new();
        assert!(!shell.wants_keys());
        shell.handle_key(ShellKey::OpenMenu);
        assert!(shell.wants_keys());
        shell.handle_key(ShellKey::Down);
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            Some(ShellRequest::Menu(MenuCommand::OpenSong))
        );
        assert!(!shell.wants_keys());
    }

    #[test]
    fn save_does_nothing_while_no_song_is_open() {
        let mut shell: Shell = Shell::new();
        shell.handle_key(ShellKey::OpenMenu);
        shell.handle_key(ShellKey::Down);
        shell.handle_key(ShellKey::Down);
        assert_eq!(shell.handle_key(ShellKey::Enter), None);
        assert!(shell.wants_keys(), "the menu stays open on a disabled item");
        shell.set_can_save(true);
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            Some(ShellRequest::Menu(MenuCommand::SaveSong))
        );
    }

    #[test]
    fn a_click_on_the_open_menu_closes_it_and_a_click_outside_does_too() {
        let mut shell: Shell = Shell::new();
        shell.click(ShellTarget::Menu(MenuId::File), false);
        assert!(shell.open_menu().is_some());
        shell.click(ShellTarget::Menu(MenuId::File), false);
        assert!(shell.open_menu().is_none());
        shell.click(ShellTarget::Menu(MenuId::File), false);
        shell.click(ShellTarget::Outside, false);
        assert!(shell.open_menu().is_none());
    }

    #[test]
    fn a_question_answered_yes_carries_on_and_no_drops() {
        let mut shell: Shell = Shell::new();
        shell.ask(question());
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            Some(ShellRequest::Confirmed(Pending::Quit))
        );
        assert!(shell.modal().is_none());
        shell.ask(question());
        assert_eq!(shell.handle_key(ShellKey::Escape), None);
        assert!(shell.modal().is_none());
        shell.ask(question());
        assert_eq!(shell.click(ShellTarget::ConfirmNo, false), None);
        assert!(shell.modal().is_none());
    }

    #[test]
    fn a_click_outside_does_not_answer_a_question() {
        let mut shell: Shell = Shell::new();
        shell.ask(question());
        assert_eq!(shell.click(ShellTarget::Outside, false), None);
        assert!(matches!(shell.modal(), Some(Modal::Confirm(_))));
    }

    #[test]
    fn the_browser_lists_folders_and_closes_when_a_can_saves() {
        let mut shell: Shell = Shell::new();
        shell.open_browser(PathBuf::from("/music/songs"));
        if let Some(browser) = shell.browser_mut() {
            browser.set_listing(
                PathBuf::from("/music/songs"),
                vec![
                    BrowserEntry {
                        name: String::from("archive"),
                        kind: BrowserEntryKind::Folder,
                    },
                    BrowserEntry {
                        name: String::from("Demo"),
                        kind: BrowserEntryKind::SongFolder,
                    },
                ],
            );
        }
        // Row 0 is the way up, 1 "archive", 2 "Demo".
        assert_eq!(
            shell.click(ShellTarget::BrowserRow(1), true),
            Some(ShellRequest::ListFolder(PathBuf::from(
                "/music/songs/archive"
            )))
        );
        assert!(shell.modal().is_some());
        assert_eq!(shell.click(ShellTarget::BrowserRow(2), false), None);
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            Some(ShellRequest::OpenSong(PathBuf::from(
                "/music/songs/Demo/Demo.song"
            )))
        );
        assert!(shell.modal().is_none());
    }

    #[test]
    fn the_wheel_and_the_scroll_bar_move_the_browsers_view() {
        let mut shell: Shell = Shell::new();
        shell.open_browser(PathBuf::from("/music/songs"));
        if let Some(browser) = shell.browser_mut() {
            let found: Vec<BrowserEntry> = (0..40)
                .map(|number: u32| BrowserEntry {
                    name: format!("folder_{number:02}"),
                    kind: BrowserEntryKind::Folder,
                })
                .collect();
            browser.set_listing(PathBuf::from("/music/songs"), found);
        }
        shell.scroll(3, None);
        assert_eq!(shell.browser_mut().map(|b| b.top()), Some(3));
        shell.click(ShellTarget::BrowserPage { up: false }, false);
        assert_eq!(shell.browser_mut().map(|b| b.top()), Some(19));
        shell.click(ShellTarget::BrowserPage { up: true }, false);
        assert_eq!(shell.browser_mut().map(|b| b.top()), Some(3));
        assert_eq!(shell.browser_mut().map(|b| b.selected()), Some(0));
    }

    #[test]
    fn backspace_in_the_browser_goes_up_and_escape_closes_it() {
        let mut shell: Shell = Shell::new();
        shell.open_browser(PathBuf::from("/music/songs"));
        assert_eq!(
            shell.handle_key(ShellKey::Backspace),
            Some(ShellRequest::ListFolder(PathBuf::from("/music")))
        );
        shell.handle_key(ShellKey::Escape);
        assert!(shell.modal().is_none());
    }

    fn new_song_browser() -> Shell {
        let mut shell: Shell = Shell::new();
        shell.open_new_song_browser(PathBuf::from("/songs"));
        shell.browser_mut().unwrap().set_listing(
            PathBuf::from("/songs"),
            vec![
                BrowserEntry {
                    name: String::from("Demo"),
                    kind: BrowserEntryKind::SongFolder,
                },
                BrowserEntry {
                    name: String::from("Empty"),
                    kind: BrowserEntryKind::Folder,
                },
            ],
        );
        shell
    }

    #[test]
    fn a_new_song_goes_in_a_folder_made_here_or_chosen_and_a_song_folder_is_not_a_choice() {
        let mut shell: Shell = new_song_browser();
        shell.handle_key(ShellKey::Down);
        assert_eq!(shell.handle_key(ShellKey::Enter), None, "a song folder");
        shell.handle_key(ShellKey::Down);
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            Some(ShellRequest::ListFolder(PathBuf::from("/songs/Empty")))
        );
        assert_eq!(
            shell.handle_key(ShellKey::Use),
            Some(ShellRequest::NewSongIn(PathBuf::from("/songs")))
        );
        assert!(
            shell.wants_keys(),
            "the browser stays until the song is made"
        );
    }

    #[test]
    fn a_folder_is_named_in_a_prompt_over_the_browser_and_escape_returns() {
        let mut shell: Shell = new_song_browser();
        assert_eq!(shell.handle_key(ShellKey::New), None);
        assert!(shell.is_typing());
        shell.type_text("Blue");
        shell.handle_key(ShellKey::Backspace);
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            Some(ShellRequest::MakeFolder {
                parent: PathBuf::from("/songs"),
                name: String::from("Blu"),
                copy_from: None
            })
        );
        shell.new_folder_say(String::from("A folder with that name exists."));
        assert!(shell.new_folder_prompt().unwrap().message.is_some());
        shell.handle_key(ShellKey::Escape);
        assert!(shell.new_folder_prompt().is_none());
        assert!(shell.browser_mut().is_some(), "back at the browser");
    }

    #[test]
    fn a_song_folder_is_copied_under_a_new_name() {
        let mut shell: Shell = new_song_browser();
        shell.handle_key(ShellKey::Down);
        assert_eq!(shell.handle_key(ShellKey::Clef), None);
        let prompt = shell.new_folder_prompt().unwrap();
        assert_eq!(prompt.field.text(), "Demo copy");
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            Some(ShellRequest::MakeFolder {
                parent: PathBuf::from("/songs"),
                name: String::from("Demo copy"),
                copy_from: Some(PathBuf::from("/songs/Demo"))
            })
        );
    }
}
