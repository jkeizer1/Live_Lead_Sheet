//! Which key does what. The keys are read by their place on the keyboard
//! (winit's physical key codes), so Ctrl+Z is the same key whatever the
//! letter the layout puts there. Arrows move the selection; Shift, Ctrl and
//! Alt on the up and down arrows move music by a semitone, a scale step and
//! an octave, the same everywhere, so the composer's hands learn one set of
//! interval keys. Ctrl+Shift on them moves notes through the tones of the
//! lead's chord.

use composer::session::{Command, Mode, Pane, Session, Zoom};
use composer::session_music::Step;
use winit::keyboard::{KeyCode, ModifiersState};

/// What the window does with a key: a command for the session, or
/// something the window itself does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Session(Command),
    /// Play from the selected entry, or stop.
    PlayOrStop,
    PlayFromStart,
    /// Loop the selected entry, or stop.
    LoopEntry,
    Save,
    Quit,
}

/// The action for a key pressed with the given modifiers, if it has one.
#[must_use]
pub fn action_for(code: KeyCode, modifiers: ModifiersState, session: &Session) -> Option<Action> {
    let shift: bool = modifiers.shift_key();
    let control: bool = modifiers.control_key();
    let alt: bool = modifiers.alt_key();
    let command = |command: Command| Some(Action::Session(command));
    if control {
        return match code {
            KeyCode::KeyZ if shift => command(Command::Redo),
            KeyCode::KeyZ => command(Command::Undo),
            KeyCode::KeyY => command(Command::Redo),
            KeyCode::KeyS => Some(Action::Save),
            KeyCode::KeyQ => Some(Action::Quit),
            // Like zooming a page: Ctrl+= nearer (wider bars, fewer across),
            // Ctrl+- further (more bars across).
            KeyCode::Equal | KeyCode::NumpadAdd => command(Command::StaffBarWidth(1)),
            KeyCode::Minus | KeyCode::NumpadSubtract => command(Command::StaffBarWidth(-1)),
            KeyCode::ArrowUp if shift => command(Command::ChordTone(1)),
            KeyCode::ArrowDown if shift => command(Command::ChordTone(-1)),
            KeyCode::ArrowUp => command(Command::Move(Step::Diatonic(1))),
            KeyCode::ArrowDown => command(Command::Move(Step::Diatonic(-1))),
            _ => None,
        };
    }
    let in_library: bool = session.pane == Pane::Library;
    match code {
        KeyCode::F1 => command(Command::SetMode(Mode::ArrangeSections)),
        KeyCode::F2 => command(Command::SetMode(Mode::ComposeSections)),
        KeyCode::F3 => command(Command::SetMode(Mode::EditLines)),
        KeyCode::F4 => command(Command::SetMode(Mode::SongHarmony)),
        KeyCode::F5 => command(Command::SetMode(Mode::TrackHarmony)),
        KeyCode::Tab => command(Command::SwitchPane),
        KeyCode::Enter | KeyCode::NumpadEnter => command(Command::Commit),
        KeyCode::Escape => command(Command::Cancel),
        KeyCode::Space if shift => Some(Action::PlayFromStart),
        KeyCode::Space => Some(Action::PlayOrStop),
        KeyCode::ArrowUp if in_library => command(Command::Up),
        KeyCode::ArrowDown if in_library => command(Command::Down),
        KeyCode::ArrowUp if shift => command(Command::Move(Step::Chromatic(1))),
        KeyCode::ArrowDown if shift => command(Command::Move(Step::Chromatic(-1))),
        KeyCode::ArrowUp if alt => command(Command::Move(Step::Octave(1))),
        KeyCode::ArrowDown if alt => command(Command::Move(Step::Octave(-1))),
        KeyCode::ArrowUp => command(Command::Up),
        KeyCode::ArrowDown => command(Command::Down),
        KeyCode::ArrowLeft if alt && session.zoom == Zoom::Form => command(Command::MoveEntry(-1)),
        KeyCode::ArrowRight if alt && session.zoom == Zoom::Form => command(Command::MoveEntry(1)),
        KeyCode::ArrowLeft if shift && !in_library => command(Command::ExtendLeft),
        KeyCode::ArrowRight if shift && !in_library => command(Command::ExtendRight),
        KeyCode::ArrowLeft => command(Command::Left),
        KeyCode::ArrowRight => command(Command::Right),
        KeyCode::BracketLeft => command(Command::Repeat(-1)),
        KeyCode::BracketRight => command(Command::Repeat(1)),
        KeyCode::KeyM => command(Command::ToggleMute),
        KeyCode::KeyD => command(Command::DuplicateEntry),
        KeyCode::Delete => command(Command::RemoveEntry),
        KeyCode::KeyK => command(Command::KeyBySemitones(if shift { -1 } else { 1 })),
        KeyCode::KeyF => command(Command::KeyByFifths(if shift { -1 } else { 1 })),
        KeyCode::KeyT => command(Command::Tempo(if shift { -5 } else { 5 })),
        KeyCode::KeyA => command(Command::CycleAccent),
        KeyCode::KeyR => command(Command::ToggleRest),
        KeyCode::KeyC => command(Command::ConvertLineForm),
        KeyCode::KeyB => command(Command::BassFromRoots),
        KeyCode::KeyN => command(Command::NewSectionFromProgression),
        KeyCode::KeyL => Some(Action::LoopEntry),
        _ => None,
    }
}
