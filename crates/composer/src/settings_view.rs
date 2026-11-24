//! The song settings dialog's state: which row is chosen and what is being
//! typed. The rows are the title, the chord numeral standard, and one per
//! track. What a row shows (type, clef, lead) is read from the song by the
//! screen; this holds only what the dialog itself remembers. No window, no
//! file: keys come in as [`SettingsKey`], and what they ask of the song
//! leaves as a [`SettingsCommand`] for the app to apply.

use player::studio_configuration::StudioConfiguration;
use vocabulary::song::Clef;

use crate::scroll_list::ScrollList;
use crate::session::MessageKind;
use crate::song_settings::{CLEFS, TrackType, clef_word};
use crate::studio_settings::{
    OutputField, StudioEdit, StudioNeeds, StudioRow, StudioRowView, edit_for_field, studio_rows,
    typing_start,
};
use crate::text_field::TextField;

/// How many rows the dialog shows at once, in each of its two panes.
pub const VISIBLE_SETTINGS_ROWS: usize = 14;

/// What the dialog is made from: the song's settings and the studio's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub title: String,
    pub track_names: Vec<String>,
    /// Each track's type now, which decides what its output needs.
    pub track_types: Vec<TrackType>,
    /// Each track's clef, for a voice.
    pub track_clefs: Vec<Option<Clef>>,
    /// For each track, every type and whether the track can change to it.
    pub type_options: Vec<Vec<(TrackType, Result<(), String>)>>,
    /// Absent when the song has no studio file.
    pub studio: Option<StudioConfiguration>,
    /// What the song needs that the studio lacks.
    pub needs: StudioNeeds,
}

/// The two panes side by side: the song's, on the left, and the studio's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Song,
    Studio,
}

/// The rows before the tracks: the title and the chord numeral standard.
pub const ROWS_BEFORE_TRACKS: usize = 2;

/// What the dialog asks the app to do to the song.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsCommand {
    SetTitle(String),
    /// Not set, key dependent, major referential, and round again.
    CycleNumerals,
    RenameTrack {
        track_index: usize,
        new_name: String,
    },
    SetType {
        track_index: usize,
        new_type: TrackType,
    },
    SetClef {
        track_index: usize,
        clef: Clef,
    },
    /// One place up or down among the song's tracks.
    MoveTrack {
        track_index: usize,
        up: bool,
    },
    /// Names (or, with `None`, un-names) one of a voice track's layers.
    SetVoiceLayerName {
        track_index: usize,
        number: u8,
        name: Option<String>,
    },
    MakeLead {
        track_index: usize,
    },
    /// A new poly voice; its type can be changed after.
    AddTrack {
        name: String,
    },
    DeleteTrack {
        track_index: usize,
    },
    /// A change to the studio, from the right pane.
    Studio(StudioEdit),
    /// Start a studio file for a song that has none.
    CreateStudio,
}

/// What a key or click means to the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsKey {
    Up,
    Down,
    PageUp,
    PageDown,
    First,
    Last,
    /// Edit the chosen row (or accept what is typed).
    Enter,
    /// Close the dialog (or cancel what is typed).
    Escape,
    Left,
    Right,
    Backspace,
    Delete,
    Type,
    Clef,
    Lead,
    New,
    DeleteTrack,
    /// Move between the song's pane and the studio's.
    Tab,
    /// Move the chosen track one place up or down among the tracks.
    MoveUp,
    MoveDown,
    /// Name one of the chosen voice's layers.
    Layer,
}

/// The buttons a click can land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsButton {
    Edit,
    Type,
    Clef,
    Lead,
    New,
    DeleteTrack,
    Accept,
    Cancel,
    Close,
    MoveUp,
    MoveDown,
    Layer,
}

/// A choice among a few words, shown along the bottom of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picker {
    /// "Type" or "Clef".
    pub what: &'static str,
    pub track_index: usize,
    pub options: Vec<PickerOption>,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerOption {
    pub label: String,
    /// Why it cannot be taken, in a sentence.
    pub refusal: Option<String>,
    pub value: PickerValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerValue {
    Type(TrackType),
    Clef(Clef),
}

/// What is being typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typing {
    Title,
    Rename {
        track_index: usize,
    },
    NewTrack,
    /// A voice layer's number and name, for a track.
    VoiceLayer {
        track_index: usize,
    },
    /// A studio row's value: the studio's name, a port, a track output
    /// setting, or a destination.
    Studio(StudioRow),
}

/// What the dialog asks of the shell after a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsOutcome {
    Nothing,
    Command(SettingsCommand),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsView {
    list: ScrollList,
    title: String,
    track_names: Vec<String>,
    track_types: Vec<TrackType>,
    typing: Option<(Typing, TextField)>,
    /// The chosen track is about to be deleted: Enter does it, Esc keeps it.
    confirming_delete: Option<usize>,
    track_clefs: Vec<Option<Clef>>,
    type_options: Vec<Vec<(TrackType, Result<(), String>)>>,
    /// A choice among a few words (a type, a clef) being made along the
    /// bottom of the dialog.
    picker: Option<Picker>,
    /// The track just moved: the chosen row follows it to its new place.
    following: Option<String>,
    /// What the last change said, in a sentence, until the next key.
    message: Option<(MessageKind, String)>,
    /// The pane the keys go to.
    pane: Pane,
    studio: Option<StudioConfiguration>,
    needs: StudioNeeds,
    studio_list: ScrollList,
    /// The song track whose output the studio pane shows: the last track
    /// row chosen in the song pane.
    output_track: Option<usize>,
    /// A studio row is about to be removed: Enter does it, Esc keeps it.
    confirming_studio_removal: Option<StudioRow>,
}

impl SettingsView {
    #[must_use]
    pub fn new(snapshot: SettingsSnapshot) -> SettingsView {
        let mut list: ScrollList = ScrollList::new(VISIBLE_SETTINGS_ROWS);
        list.reset(ROWS_BEFORE_TRACKS + snapshot.track_names.len());
        let mut view: SettingsView = SettingsView {
            list,
            title: String::new(),
            track_names: Vec::new(),
            track_types: Vec::new(),
            typing: None,
            confirming_delete: None,
            track_clefs: Vec::new(),
            type_options: Vec::new(),
            picker: None,
            following: None,
            message: None,
            pane: Pane::Song,
            studio: None,
            needs: StudioNeeds::default(),
            studio_list: ScrollList::new(VISIBLE_SETTINGS_ROWS),
            output_track: None,
            confirming_studio_removal: None,
        };
        view.take(snapshot);
        view.studio_list.reset(view.studio_rows().len());
        if view
            .chosen_studio_row()
            .is_some_and(|row| !row.row.selectable())
        {
            view.step_studio_selection(true);
        }
        view
    }

    /// The song or the studio changed: take both again, keeping the chosen
    /// rows where they still exist.
    pub fn refresh(&mut self, snapshot: SettingsSnapshot) {
        self.take(snapshot);
        self.confirming_delete = None;
        self.confirming_studio_removal = None;
    }

    fn take(&mut self, snapshot: SettingsSnapshot) {
        let grew: bool =
            snapshot.track_names.len() > self.track_names.len() && !self.track_names.is_empty();
        self.title = snapshot.title;
        self.track_names = snapshot.track_names;
        self.track_types = snapshot.track_types;
        self.track_clefs = snapshot.track_clefs;
        self.type_options = snapshot.type_options;
        self.studio = snapshot.studio;
        self.needs = snapshot.needs;
        self.list
            .resize(ROWS_BEFORE_TRACKS + self.track_names.len());
        self.picker = None;
        if let Some(name) = self.following.take()
            && let Some(position) = self.track_names.iter().position(|n| *n == name)
        {
            self.list.select(ROWS_BEFORE_TRACKS + position);
        }
        if grew {
            // A part was added: it is chosen, and its type is the next thing
            // to say.
            let last: usize = self.track_names.len() - 1;
            self.list.select(ROWS_BEFORE_TRACKS + last);
            self.open_type_picker(last);
        }
        self.follow_track_choice();
    }

    /// The studio pane shows the output of the song track last chosen.
    fn follow_track_choice(&mut self) {
        if let Some(track_index) = self.selected_track() {
            self.output_track = Some(track_index);
        }
        if self
            .output_track
            .is_some_and(|index| index >= self.track_names.len())
        {
            self.output_track = None;
        }
        let rows: usize = self.studio_rows().len();
        self.studio_list.resize(rows);
    }

    /// The studio pane's rows.
    #[must_use]
    pub fn studio_rows(&self) -> Vec<StudioRowView> {
        let track: Option<(&str, TrackType)> = self.output_track.and_then(|index| {
            Some((
                self.track_names.get(index)?.as_str(),
                *self.track_types.get(index)?,
            ))
        });
        studio_rows(self.studio.as_ref(), track, &self.needs)
    }

    /// The pane that has the keys; a click in a pane gives it them.
    pub fn set_pane(&mut self, pane: Pane) {
        self.pane = pane;
    }

    /// The wheel over a pane: scrolls that pane's list by `rows`.
    pub fn scroll_pane(&mut self, pane: Pane, rows: isize) {
        match pane {
            Pane::Song => self.list.scroll_by(rows),
            Pane::Studio => self.studio_list.scroll_by(rows),
        }
    }

    /// Whether the song needs something of the studio for this track.
    #[must_use]
    pub fn track_needs_studio(&self, track_name: &str) -> bool {
        self.needs.tracks.iter().any(|name| name == track_name)
    }

    #[must_use]
    pub fn pane(&self) -> Pane {
        self.pane
    }

    #[must_use]
    pub fn studio_list(&self) -> &ScrollList {
        &self.studio_list
    }

    pub fn studio_list_mut(&mut self) -> &mut ScrollList {
        &mut self.studio_list
    }

    /// The studio row about to be removed, if one is.
    #[must_use]
    pub fn confirming_studio_removal(&self) -> Option<StudioRow> {
        self.confirming_studio_removal
    }

    /// The song track whose output the studio pane shows.
    #[must_use]
    pub fn output_track_name(&self) -> Option<&str> {
        self.output_track
            .and_then(|index| self.track_names.get(index))
            .map(String::as_str)
    }

    /// What the last change said.
    pub fn say(&mut self, kind: MessageKind, text: String) {
        self.message = Some((kind, text));
    }

    #[must_use]
    pub fn message(&self) -> Option<&(MessageKind, String)> {
        self.message.as_ref()
    }

    /// The choice being made along the bottom of the dialog, if any.
    #[must_use]
    pub fn picker(&self) -> Option<&Picker> {
        self.picker.as_ref()
    }

    /// Opens the choice of types for a track.
    fn open_type_picker(&mut self, track_index: usize) {
        let Some(options) = self.type_options.get(track_index) else {
            return;
        };
        let options: Vec<PickerOption> = options
            .iter()
            .map(|(candidate, allowed)| PickerOption {
                label: candidate.word().to_string(),
                refusal: allowed.clone().err(),
                value: PickerValue::Type(*candidate),
            })
            .collect();
        let current: Option<TrackType> = self.track_types.get(track_index).copied();
        let selected: usize = options
            .iter()
            .position(|o| matches!(o.value, PickerValue::Type(t) if Some(t) == current))
            .unwrap_or(0);
        self.picker = Some(Picker {
            what: "Type",
            track_index,
            options,
            selected,
        });
    }

    /// Opens the choice of clefs for a voice track.
    fn open_clef_picker(&mut self, track_index: usize) {
        let Some(Some(current)) = self.track_clefs.get(track_index).copied() else {
            self.message = Some((
                MessageKind::Problem,
                String::from("Only a voice has a clef."),
            ));
            return;
        };
        let options: Vec<PickerOption> = CLEFS
            .iter()
            .map(|clef| PickerOption {
                label: clef_word(*clef).to_string(),
                refusal: None,
                value: PickerValue::Clef(*clef),
            })
            .collect();
        let selected: usize = CLEFS.iter().position(|c| *c == current).unwrap_or(0);
        self.picker = Some(Picker {
            what: "Clef",
            track_index,
            options,
            selected,
        });
    }

    /// A key while a choice is open: Left and Right move among the
    /// options, Enter takes the one shown (or says why it cannot be), Esc
    /// puts it away.
    fn picker_key(&mut self, key: SettingsKey) -> SettingsOutcome {
        let Some(picker) = self.picker.as_mut() else {
            return SettingsOutcome::Nothing;
        };
        match key {
            SettingsKey::Left | SettingsKey::Up => {
                picker.selected = picker.selected.saturating_sub(1);
            }
            SettingsKey::Right | SettingsKey::Down => {
                picker.selected = (picker.selected + 1).min(picker.options.len() - 1);
            }
            SettingsKey::First => picker.selected = 0,
            SettingsKey::Last => picker.selected = picker.options.len() - 1,
            SettingsKey::Escape => self.picker = None,
            SettingsKey::Enter => {
                let option: PickerOption = picker.options[picker.selected].clone();
                let track_index: usize = picker.track_index;
                if let Some(reason) = option.refusal {
                    self.message = Some((MessageKind::Problem, reason));
                    return SettingsOutcome::Nothing;
                }
                self.picker = None;
                return SettingsOutcome::Command(match option.value {
                    PickerValue::Type(new_type) => SettingsCommand::SetType {
                        track_index,
                        new_type,
                    },
                    PickerValue::Clef(clef) => SettingsCommand::SetClef { track_index, clef },
                });
            }
            _ => {}
        }
        // The reason a refused option is refused shows as it is passed over.
        if let Some(picker) = &self.picker
            && let Some(reason) = &picker.options[picker.selected].refusal
        {
            self.message = Some((MessageKind::Problem, reason.clone()));
        }
        SettingsOutcome::Nothing
    }

    /// A click on one of the options of the open choice.
    pub fn click_pick(&mut self, option: usize, double: bool) -> SettingsOutcome {
        let Some(picker) = self.picker.as_mut() else {
            return SettingsOutcome::Nothing;
        };
        if option < picker.options.len() {
            picker.selected = option;
        }
        if double {
            self.picker_key(SettingsKey::Enter)
        } else {
            SettingsOutcome::Nothing
        }
    }

    #[must_use]
    pub fn list(&self) -> &ScrollList {
        &self.list
    }

    pub fn list_mut(&mut self) -> &mut ScrollList {
        &mut self.list
    }

    #[must_use]
    pub fn typing(&self) -> Option<&(Typing, TextField)> {
        self.typing.as_ref()
    }

    #[must_use]
    pub fn is_typing(&self) -> bool {
        self.typing.is_some()
    }

    #[must_use]
    pub fn confirming_delete(&self) -> Option<usize> {
        self.confirming_delete
    }

    /// The chosen track, when the chosen row is one.
    #[must_use]
    pub fn selected_track(&self) -> Option<usize> {
        self.list.selected().checked_sub(ROWS_BEFORE_TRACKS)
    }

    #[must_use]
    pub fn track_name(&self, track_index: usize) -> Option<&str> {
        self.track_names.get(track_index).map(String::as_str)
    }

    pub fn type_text(&mut self, text: &str) {
        if let Some((_, field)) = self.typing.as_mut() {
            field.insert(text);
        }
    }

    pub fn click_row(&mut self, row: usize, double: bool) -> SettingsOutcome {
        if self.typing.is_some() {
            return SettingsOutcome::Nothing;
        }
        self.confirming_delete = None;
        self.pane = Pane::Song;
        self.list.select(row);
        self.follow_track_choice();
        if double {
            self.key(SettingsKey::Enter)
        } else {
            SettingsOutcome::Nothing
        }
    }

    /// A click on a row of the studio pane.
    pub fn click_studio_row(&mut self, row: usize, double: bool) -> SettingsOutcome {
        if self.typing.is_some() {
            return SettingsOutcome::Nothing;
        }
        self.confirming_studio_removal = None;
        self.pane = Pane::Studio;
        self.studio_list.select(row);
        if double {
            self.key(SettingsKey::Enter)
        } else {
            SettingsOutcome::Nothing
        }
    }

    pub fn click_button(&mut self, button: SettingsButton) -> SettingsOutcome {
        let key: SettingsKey = match button {
            SettingsButton::Edit | SettingsButton::Accept => SettingsKey::Enter,
            SettingsButton::Type => SettingsKey::Type,
            SettingsButton::Clef => SettingsKey::Clef,
            SettingsButton::Lead => SettingsKey::Lead,
            SettingsButton::MoveUp => SettingsKey::MoveUp,
            SettingsButton::MoveDown => SettingsKey::MoveDown,
            SettingsButton::Layer => SettingsKey::Layer,
            SettingsButton::New => SettingsKey::New,
            SettingsButton::DeleteTrack => SettingsKey::DeleteTrack,
            SettingsButton::Cancel | SettingsButton::Close => SettingsKey::Escape,
        };
        self.key(key)
    }

    pub fn key(&mut self, key: SettingsKey) -> SettingsOutcome {
        self.message = None;
        if self.typing.is_some() {
            return self.typing_key(key);
        }
        if self.picker.is_some() {
            return self.picker_key(key);
        }
        if let Some(row) = self.confirming_studio_removal {
            self.confirming_studio_removal = None;
            return match key {
                SettingsKey::Enter => self.studio_removal(row),
                _ => SettingsOutcome::Nothing,
            };
        }
        if key == SettingsKey::Tab {
            self.pane = match self.pane {
                Pane::Song => Pane::Studio,
                Pane::Studio => Pane::Song,
            };
            return SettingsOutcome::Nothing;
        }
        if self.pane == Pane::Studio {
            return self.studio_key(key);
        }
        if let Some(track_index) = self.confirming_delete {
            return match key {
                SettingsKey::Enter => {
                    self.confirming_delete = None;
                    SettingsOutcome::Command(SettingsCommand::DeleteTrack { track_index })
                }
                SettingsKey::Escape => {
                    self.confirming_delete = None;
                    SettingsOutcome::Nothing
                }
                _ => SettingsOutcome::Nothing,
            };
        }
        let track: Option<usize> = self.selected_track();
        match key {
            SettingsKey::Up => self.list.up(),
            SettingsKey::Down => self.list.down(),
            SettingsKey::PageUp => self.list.page_up(),
            SettingsKey::PageDown => self.list.page_down(),
            SettingsKey::First => self.list.first(),
            SettingsKey::Last => self.list.last(),
            SettingsKey::Escape => return SettingsOutcome::Close,
            SettingsKey::Enter => return self.edit_chosen(track),
            SettingsKey::Left | SettingsKey::Right if self.list.selected() == 1 => {
                return SettingsOutcome::Command(SettingsCommand::CycleNumerals);
            }
            SettingsKey::Type => {
                if let Some(track_index) = track {
                    self.open_type_picker(track_index);
                }
            }
            SettingsKey::Clef => {
                if let Some(track_index) = track {
                    self.open_clef_picker(track_index);
                }
            }
            SettingsKey::MoveUp | SettingsKey::MoveDown => {
                if let Some(track_index) = track {
                    let up: bool = key == SettingsKey::MoveUp;
                    self.following = self.track_names.get(track_index).cloned();
                    return SettingsOutcome::Command(SettingsCommand::MoveTrack {
                        track_index,
                        up,
                    });
                }
            }
            SettingsKey::Layer => {
                if let Some(track_index) = track {
                    self.typing = Some((Typing::VoiceLayer { track_index }, TextField::new("")));
                }
            }
            SettingsKey::Lead => {
                if let Some(track_index) = track {
                    return SettingsOutcome::Command(SettingsCommand::MakeLead { track_index });
                }
            }
            SettingsKey::New => self.typing = Some((Typing::NewTrack, TextField::new(""))),
            SettingsKey::DeleteTrack | SettingsKey::Delete => {
                self.confirming_delete = track;
            }
            SettingsKey::Left | SettingsKey::Right | SettingsKey::Backspace | SettingsKey::Tab => {}
        }
        self.follow_track_choice();
        SettingsOutcome::Nothing
    }

    /// `2 Inner` names layer 2 Inner; `2` alone clears its name.
    fn voice_layer_typed(&mut self, track_index: usize, typed: &str) -> SettingsOutcome {
        let typed: &str = typed.trim();
        let (number_text, name_text): (&str, &str) =
            typed.split_once(char::is_whitespace).unwrap_or((typed, ""));
        let Ok(number) = number_text.parse::<u8>() else {
            self.message = Some((
                MessageKind::Problem,
                String::from("Type the layer's number, then its name, e.g. 2 Inner."),
            ));
            return SettingsOutcome::Nothing;
        };
        let name: &str = name_text.trim();
        SettingsOutcome::Command(SettingsCommand::SetVoiceLayerName {
            track_index,
            number,
            name: (!name.is_empty()).then(|| name.to_string()),
        })
    }

    /// A key while the studio pane has the keys.
    fn studio_key(&mut self, key: SettingsKey) -> SettingsOutcome {
        match key {
            SettingsKey::Up => self.step_studio_selection(false),
            SettingsKey::Down => self.step_studio_selection(true),
            SettingsKey::PageUp => self.studio_list.page_up(),
            SettingsKey::PageDown => self.studio_list.page_down(),
            SettingsKey::First => self.studio_list.first(),
            SettingsKey::Last => self.studio_list.last(),
            SettingsKey::Escape => return SettingsOutcome::Close,
            SettingsKey::Enter => return self.edit_studio_row(),
            SettingsKey::Delete | SettingsKey::DeleteTrack => {
                if let Some(view) = self.chosen_studio_row() {
                    self.confirming_studio_removal = removable(view.row);
                }
            }
            SettingsKey::Left
            | SettingsKey::Right
            | SettingsKey::Backspace
            | SettingsKey::Type
            | SettingsKey::Clef
            | SettingsKey::Lead
            | SettingsKey::New
            | SettingsKey::MoveUp
            | SettingsKey::MoveDown
            | SettingsKey::Layer
            | SettingsKey::Tab => {}
        }
        SettingsOutcome::Nothing
    }

    /// Moves the chosen studio row one down or up, past headings, which
    /// hold nothing to edit.
    fn step_studio_selection(&mut self, down: bool) {
        let rows: Vec<StudioRowView> = self.studio_rows();
        loop {
            let before: usize = self.studio_list.selected();
            if down {
                self.studio_list.down();
            } else {
                self.studio_list.up();
            }
            let now: usize = self.studio_list.selected();
            let on_heading: bool = rows.get(now).is_some_and(|row| !row.row.selectable());
            if now == before || !on_heading {
                if now == before && on_heading {
                    // At an edge with only headings beyond: go back.
                    self.studio_list.select(before);
                }
                return;
            }
        }
    }

    fn chosen_studio_row(&self) -> Option<StudioRowView> {
        self.studio_rows()
            .into_iter()
            .nth(self.studio_list.selected())
    }

    /// Enter on a studio row: type its value, step it, or add what it adds.
    fn edit_studio_row(&mut self) -> SettingsOutcome {
        let Some(view) = self.chosen_studio_row() else {
            return SettingsOutcome::Nothing;
        };
        let track_name: Option<String> = self.output_track_name().map(str::to_string);
        let start_typing = |this: &mut SettingsView, text: &str| {
            this.typing = Some((Typing::Studio(view.row), TextField::new(text)));
            SettingsOutcome::Nothing
        };
        match view.row {
            StudioRow::Heading | StudioRow::Note | StudioRow::Warning => SettingsOutcome::Nothing,
            StudioRow::Create => SettingsOutcome::Command(SettingsCommand::CreateStudio),
            // The port's name is the row's label; the others show theirs as the value.
            StudioRow::Port(_) => start_typing(self, &view.label),
            StudioRow::Name | StudioRow::DestinationName(_) => start_typing(self, &view.value),
            StudioRow::AddPort | StudioRow::AddDestination => start_typing(self, ""),
            StudioRow::DestinationChannel(_) | StudioRow::Field(_)
                if matches!(view.row, StudioRow::Field(OutputField::Port)) =>
            {
                self.step_port(view.row, track_name)
            }
            StudioRow::DestinationChannel(_) | StudioRow::Field(_) => {
                start_typing(self, &typing_start(&view))
            }
            StudioRow::DestinationPort(_) => self.step_port(view.row, track_name),
            StudioRow::AddOutput => match track_name {
                Some(track_name) => {
                    SettingsOutcome::Command(SettingsCommand::Studio(StudioEdit::AddOutput {
                        track_name,
                    }))
                }
                None => SettingsOutcome::Nothing,
            },
        }
    }

    /// Steps a track's or a destination's port to the next port.
    fn step_port(&self, row: StudioRow, track_name: Option<String>) -> SettingsOutcome {
        let Some(studio) = &self.studio else {
            return SettingsOutcome::Nothing;
        };
        let ports: &[String] = studio.port_names();
        let next_after = |current: &str| -> Option<usize> {
            let position: usize = ports.iter().position(|port| port == current)?;
            Some((position + 1) % ports.len())
        };
        let edit: Option<StudioEdit> = match (row, track_name) {
            (StudioRow::Field(OutputField::Port), Some(track_name)) => studio
                .output_of(&track_name)
                .and_then(|output| next_after(&output.port_name))
                .map(|port_index| StudioEdit::SetTrackPort {
                    track_name,
                    port_index,
                }),
            (StudioRow::DestinationPort(destination_index), _) => studio
                .destinations()
                .get(destination_index)
                .and_then(|destination| next_after(&destination.port_name))
                .map(|port_index| StudioEdit::SetDestinationPort {
                    destination_index,
                    port_index,
                }),
            _ => None,
        };
        edit.map_or(SettingsOutcome::Nothing, |edit| {
            SettingsOutcome::Command(SettingsCommand::Studio(edit))
        })
    }

    /// What was typed into a studio row, as an edit (or one sentence on why
    /// it is not a value for that row).
    fn studio_typed(&mut self, row: StudioRow, typed: &str) -> SettingsOutcome {
        let typed: String = typed.trim().to_string();
        let track_name: String = self.output_track_name().unwrap_or_default().to_string();
        let edit: Result<StudioEdit, String> = match row {
            StudioRow::Name => Ok(StudioEdit::SetName(typed)),
            StudioRow::AddPort => Ok(StudioEdit::AddPort(typed)),
            StudioRow::Port(port_index) => Ok(StudioEdit::RenamePort {
                port_index,
                new_name: typed,
            }),
            StudioRow::AddDestination => Ok(StudioEdit::AddDestination { name: typed }),
            StudioRow::DestinationName(destination_index) => Ok(StudioEdit::RenameDestination {
                destination_index,
                new_name: typed,
            }),
            StudioRow::DestinationChannel(destination_index) => if typed.is_empty() {
                Ok(None)
            } else {
                typed
                    .parse::<u8>()
                    .map(Some)
                    .map_err(|_| String::from("A channel is 1 to 16."))
            }
            .map(|channel_number| StudioEdit::SetDestinationChannel {
                destination_index,
                channel_number,
            }),
            StudioRow::Field(field) => edit_for_field(&track_name, field, &typed),
            StudioRow::Heading
            | StudioRow::Note
            | StudioRow::Warning
            | StudioRow::DestinationPort(_)
            | StudioRow::AddOutput
            | StudioRow::Create => Err(String::from("That row is not typed.")),
        };
        match edit {
            Ok(edit) => SettingsOutcome::Command(SettingsCommand::Studio(edit)),
            Err(reason) => {
                self.message = Some((MessageKind::Problem, reason));
                SettingsOutcome::Nothing
            }
        }
    }

    /// The edit that removes what a studio row is.
    fn studio_removal(&self, row: StudioRow) -> SettingsOutcome {
        let edit: Option<StudioEdit> = match row {
            StudioRow::Port(port_index) => Some(StudioEdit::DeletePort { port_index }),
            StudioRow::DestinationName(destination_index)
            | StudioRow::DestinationPort(destination_index)
            | StudioRow::DestinationChannel(destination_index) => {
                Some(StudioEdit::DeleteDestination { destination_index })
            }
            StudioRow::Field(_) => {
                self.output_track_name()
                    .map(|track_name| StudioEdit::RemoveOutput {
                        track_name: track_name.to_string(),
                    })
            }
            _ => None,
        };
        edit.map_or(SettingsOutcome::Nothing, |edit| {
            SettingsOutcome::Command(SettingsCommand::Studio(edit))
        })
    }

    fn edit_chosen(&mut self, track: Option<usize>) -> SettingsOutcome {
        match (self.list.selected(), track) {
            (0, _) => {
                self.typing = Some((Typing::Title, TextField::new(&self.title)));
                SettingsOutcome::Nothing
            }
            (1, _) => SettingsOutcome::Command(SettingsCommand::CycleNumerals),
            (_, Some(track_index)) => {
                let name: String = self
                    .track_names
                    .get(track_index)
                    .cloned()
                    .unwrap_or_default();
                self.typing = Some((Typing::Rename { track_index }, TextField::new(&name)));
                SettingsOutcome::Nothing
            }
            _ => SettingsOutcome::Nothing,
        }
    }

    fn typing_key(&mut self, key: SettingsKey) -> SettingsOutcome {
        let Some((typing, field)) = self.typing.as_mut() else {
            return SettingsOutcome::Nothing;
        };
        match key {
            SettingsKey::Enter => {
                let typed: String = field.text().to_string();
                let typing: Typing = *typing;
                self.typing = None;
                SettingsOutcome::Command(match typing {
                    Typing::Title => SettingsCommand::SetTitle(typed),
                    Typing::Rename { track_index } => SettingsCommand::RenameTrack {
                        track_index,
                        new_name: typed,
                    },
                    Typing::NewTrack => SettingsCommand::AddTrack { name: typed },
                    Typing::VoiceLayer { track_index } => {
                        return self.voice_layer_typed(track_index, &typed);
                    }
                    Typing::Studio(row) => return self.studio_typed(row, &typed),
                })
            }
            SettingsKey::Escape => {
                self.typing = None;
                SettingsOutcome::Nothing
            }
            SettingsKey::Left => {
                field.left();
                SettingsOutcome::Nothing
            }
            SettingsKey::Right => {
                field.right();
                SettingsOutcome::Nothing
            }
            SettingsKey::First => {
                field.home();
                SettingsOutcome::Nothing
            }
            SettingsKey::Last => {
                field.end();
                SettingsOutcome::Nothing
            }
            SettingsKey::Backspace => {
                field.backspace();
                SettingsOutcome::Nothing
            }
            SettingsKey::Delete => {
                field.delete();
                SettingsOutcome::Nothing
            }
            _ => SettingsOutcome::Nothing,
        }
    }
}

/// Whether a studio row can be removed, and which.
fn removable(row: StudioRow) -> Option<StudioRow> {
    match row {
        StudioRow::Port(_)
        | StudioRow::DestinationName(_)
        | StudioRow::DestinationPort(_)
        | StudioRow::DestinationChannel(_)
        | StudioRow::Field(_) => Some(row),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{SettingsCommand, SettingsKey, SettingsOutcome, SettingsSnapshot, SettingsView};
    use crate::song_settings::TrackType;
    use crate::studio_settings::StudioNeeds;
    use vocabulary::song::Clef;

    /// Every type, each allowed except those named.
    fn options_except(refused: &[TrackType]) -> Vec<(TrackType, Result<(), String>)> {
        [
            TrackType::PolyVoice,
            TrackType::MonoVoice,
            TrackType::ClockAndTransport,
            TrackType::CueControl,
            TrackType::MeterControl,
        ]
        .into_iter()
        .map(|candidate: TrackType| {
            let allowed: Result<(), String> = if refused.contains(&candidate) {
                Err(String::from("The song uses it."))
            } else {
                Ok(())
            };
            (candidate, allowed)
        })
        .collect()
    }

    fn view() -> SettingsView {
        SettingsView::new(SettingsSnapshot {
            title: String::from("Song"),
            track_names: vec![
                String::from("Clock"),
                String::from("Keys"),
                String::from("Bass"),
            ],
            track_types: vec![
                TrackType::ClockAndTransport,
                TrackType::PolyVoice,
                TrackType::PolyVoice,
            ],
            track_clefs: vec![None, Some(Clef::Treble), Some(Clef::Treble)],
            type_options: vec![
                options_except(&[TrackType::PolyVoice, TrackType::MonoVoice]),
                options_except(&[]),
                options_except(&[]),
            ],
            studio: None,
            needs: StudioNeeds::default(),
        })
    }

    fn command(outcome: SettingsOutcome) -> SettingsCommand {
        match outcome {
            SettingsOutcome::Command(command) => command,
            other => panic!("expected a command, got {other:?}"),
        }
    }

    #[test]
    fn enter_on_the_title_types_it_and_enter_again_sets_it() {
        let mut view: SettingsView = view();
        assert_eq!(view.key(SettingsKey::Enter), SettingsOutcome::Nothing);
        assert!(view.is_typing());
        view.type_text("s");
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::SetTitle(String::from("Songs"))
        );
        assert!(!view.is_typing());
    }

    #[test]
    fn escape_while_typing_cancels_the_typing_and_not_the_dialog() {
        let mut view: SettingsView = view();
        view.key(SettingsKey::Enter);
        assert_eq!(view.key(SettingsKey::Escape), SettingsOutcome::Nothing);
        assert!(!view.is_typing());
        assert_eq!(view.key(SettingsKey::Escape), SettingsOutcome::Close);
    }

    #[test]
    fn the_numerals_row_cycles_and_a_track_row_renames() {
        let mut view: SettingsView = view();
        view.key(SettingsKey::Down);
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::CycleNumerals
        );
        view.key(SettingsKey::Down);
        view.key(SettingsKey::Down);
        assert_eq!(view.selected_track(), Some(1));
        view.key(SettingsKey::Enter);
        view.key(SettingsKey::Backspace);
        view.type_text("z");
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::RenameTrack {
                track_index: 1,
                new_name: String::from("Keyz")
            }
        );
    }

    #[test]
    fn type_clef_and_lead_act_on_a_track_row_only() {
        let mut view: SettingsView = view();
        assert_eq!(view.key(SettingsKey::Type), SettingsOutcome::Nothing);
        assert!(view.picker().is_none());
        view.list_mut().select(4);
        // T opens the choice of types, showing the track's own type
        // (poly_voice, the first); three steps right is cue_control, and
        // Enter takes it.
        view.key(SettingsKey::Type);
        assert_eq!(view.picker().unwrap().what, "Type");
        for _ in 0..3 {
            view.key(SettingsKey::Right);
        }
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::SetType {
                track_index: 2,
                new_type: TrackType::CueControl
            }
        );
        assert!(view.picker().is_none());
        view.key(SettingsKey::Clef);
        assert_eq!(view.picker().unwrap().what, "Clef");
        view.key(SettingsKey::Right);
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::SetClef {
                track_index: 2,
                clef: Clef::Bass
            }
        );
        assert_eq!(
            command(view.key(SettingsKey::Lead)),
            SettingsCommand::MakeLead { track_index: 2 }
        );
    }

    #[test]
    fn a_type_the_track_cannot_take_says_why_and_is_not_applied() {
        let mut view: SettingsView = view();
        view.list_mut().select(2);
        view.key(SettingsKey::Type);
        // Clock is a clock_and_transport; poly_voice (the first option) is
        // refused, and the reason shows as it is passed over.
        view.key(SettingsKey::First);
        let (_, text): &(crate::session::MessageKind, String) = view.message().unwrap();
        assert_eq!(text, "The song uses it.");
        assert_eq!(view.key(SettingsKey::Enter), SettingsOutcome::Nothing);
        assert!(view.picker().is_some(), "the choice stays open");
        view.key(SettingsKey::Escape);
        assert!(view.picker().is_none());
    }

    #[test]
    fn a_clock_has_no_clef_to_choose() {
        let mut view: SettingsView = view();
        view.list_mut().select(2);
        view.key(SettingsKey::Clef);
        assert!(view.picker().is_none());
        assert_eq!(view.message().unwrap().1, "Only a voice has a clef.");
    }

    #[test]
    fn move_keys_ask_to_move_the_chosen_track_and_it_stays_chosen() {
        let mut view: SettingsView = view();
        view.list_mut().select(4);
        assert_eq!(
            command(view.key(SettingsKey::MoveUp)),
            SettingsCommand::MoveTrack {
                track_index: 2,
                up: true
            }
        );
        // The song then has Bass before Keys; the chosen row follows Bass.
        view.refresh(SettingsSnapshot {
            title: String::from("Song"),
            track_names: vec![
                String::from("Clock"),
                String::from("Bass"),
                String::from("Keys"),
            ],
            track_types: vec![
                TrackType::ClockAndTransport,
                TrackType::PolyVoice,
                TrackType::PolyVoice,
            ],
            track_clefs: vec![None, Some(Clef::Treble), Some(Clef::Treble)],
            type_options: vec![
                options_except(&[]),
                options_except(&[]),
                options_except(&[]),
            ],
            studio: None,
            needs: StudioNeeds::default(),
        });
        assert_eq!(view.selected_track(), Some(1));
    }

    #[test]
    fn a_layer_is_named_by_number_and_name() {
        let mut view: SettingsView = view();
        view.list_mut().select(3);
        view.key(SettingsKey::Layer);
        view.type_text("2 Inner voice");
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::SetVoiceLayerName {
                track_index: 1,
                number: 2,
                name: Some(String::from("Inner voice"))
            }
        );
        view.key(SettingsKey::Layer);
        view.type_text("2");
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::SetVoiceLayerName {
                track_index: 1,
                number: 2,
                name: None
            }
        );
    }

    #[test]
    fn adding_a_track_opens_the_choice_of_its_type() {
        let mut view: SettingsView = view();
        view.refresh(SettingsSnapshot {
            title: String::from("Song"),
            track_names: vec![
                String::from("Clock"),
                String::from("Keys"),
                String::from("Bass"),
                String::from("Pad"),
            ],
            track_types: vec![
                TrackType::ClockAndTransport,
                TrackType::PolyVoice,
                TrackType::PolyVoice,
                TrackType::PolyVoice,
            ],
            track_clefs: vec![
                None,
                Some(Clef::Treble),
                Some(Clef::Treble),
                Some(Clef::Treble),
            ],
            type_options: vec![
                options_except(&[]),
                options_except(&[]),
                options_except(&[]),
                options_except(&[]),
            ],
            studio: None,
            needs: StudioNeeds::default(),
        });
        assert_eq!(view.selected_track(), Some(3));
        assert_eq!(view.picker().unwrap().what, "Type");
    }

    #[test]
    fn deleting_asks_first_and_escape_keeps_the_track() {
        let mut view: SettingsView = view();
        view.list_mut().select(3);
        view.key(SettingsKey::Delete);
        assert_eq!(view.confirming_delete(), Some(1));
        view.key(SettingsKey::Escape);
        assert_eq!(view.confirming_delete(), None);
        view.key(SettingsKey::Delete);
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::DeleteTrack { track_index: 1 }
        );
    }

    #[test]
    fn a_new_track_is_typed_by_name() {
        let mut view: SettingsView = view();
        view.key(SettingsKey::New);
        view.type_text("Pad");
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::AddTrack {
                name: String::from("Pad")
            }
        );
    }

    #[test]
    fn refreshing_keeps_the_chosen_row_when_it_still_exists() {
        let mut view: SettingsView = view();
        view.list_mut().select(4);
        view.refresh(SettingsSnapshot {
            title: String::from("Song"),
            track_names: vec![String::from("Clock"), String::from("Keys")],
            track_types: vec![TrackType::PolyVoice, TrackType::PolyVoice],
            track_clefs: vec![Some(Clef::Treble), Some(Clef::Treble)],
            type_options: vec![options_except(&[]), options_except(&[])],
            studio: None,
            needs: StudioNeeds::default(),
        });
        assert_eq!(view.list().selected(), 3);
        assert_eq!(view.list().len(), 4);
    }

    const STUDIO: &str = include_str!("../../../songs_canonical/song_settings/studio_base.studio");

    fn view_with_studio() -> SettingsView {
        let studio = song_text::studio_file::parse_studio("studio_base.studio", STUDIO).unwrap();
        SettingsView::new(SettingsSnapshot {
            title: String::from("Song"),
            track_names: vec![
                String::from("Clock"),
                String::from("Keys"),
                String::from("Bass"),
            ],
            track_types: vec![
                TrackType::ClockAndTransport,
                TrackType::PolyVoice,
                TrackType::PolyVoice,
            ],
            track_clefs: vec![None, Some(Clef::Treble), Some(Clef::Treble)],
            type_options: vec![
                options_except(&[]),
                options_except(&[]),
                options_except(&[]),
            ],
            studio: Some(studio),
            needs: StudioNeeds::default(),
        })
    }

    /// Chooses the studio row whose label is `label`, in the studio pane.
    fn go_to(view: &mut SettingsView, label: &str) {
        if view.pane() == super::Pane::Song {
            view.key(SettingsKey::Tab);
        }
        let index: usize = view
            .studio_rows()
            .iter()
            .position(|row| row.label == label)
            .unwrap();
        view.studio_list_mut().select(index);
    }

    #[test]
    fn tab_moves_between_the_panes_and_the_studio_follows_the_chosen_track() {
        let mut view: SettingsView = view_with_studio();
        assert_eq!(view.pane(), super::Pane::Song);
        view.list_mut().select(3);
        view.key(SettingsKey::Down);
        assert_eq!(view.output_track_name(), Some("Bass"));
        view.key(SettingsKey::Tab);
        assert_eq!(view.pane(), super::Pane::Studio);
        assert!(
            view.studio_rows()
                .iter()
                .any(|r| r.label == "Track destination: Bass")
        );
        view.key(SettingsKey::Tab);
        assert_eq!(view.pane(), super::Pane::Song);
    }

    #[test]
    fn typing_a_channel_into_a_track_output_gives_the_edit() {
        let mut view: SettingsView = view_with_studio();
        view.list_mut().select(3);
        view.key(SettingsKey::Up);
        view.key(SettingsKey::Down);
        go_to(&mut view, "channel");
        view.key(SettingsKey::Enter);
        view.key(SettingsKey::Backspace);
        view.type_text("9");
        assert_eq!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::Studio(crate::studio_settings::StudioEdit::SetTrackChannel {
                track_name: String::from("Keys"),
                channel_number: Some(9)
            })
        );
    }

    #[test]
    fn a_value_that_is_not_a_number_says_one_sentence_and_changes_nothing() {
        let mut view: SettingsView = view_with_studio();
        view.list_mut().select(3);
        view.key(SettingsKey::Up);
        view.key(SettingsKey::Down);
        go_to(&mut view, "channel");
        view.key(SettingsKey::Enter);
        view.type_text("x");
        assert_eq!(view.key(SettingsKey::Enter), SettingsOutcome::Nothing);
        assert!(view.message().is_some());
    }

    #[test]
    fn a_port_is_stepped_and_removal_asks_first() {
        let mut view: SettingsView = view_with_studio();
        view.list_mut().select(3);
        view.key(SettingsKey::Up);
        view.key(SettingsKey::Down);
        go_to(&mut view, "port");
        assert!(matches!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::Studio(crate::studio_settings::StudioEdit::SetTrackPort {
                port_index: 2,
                ..
            })
        ));
        go_to(&mut view, "To SQ-1");
        view.key(SettingsKey::Delete);
        assert!(view.confirming_studio_removal().is_some());
        assert_eq!(view.key(SettingsKey::Escape), SettingsOutcome::Nothing);
        view.key(SettingsKey::Delete);
        assert!(matches!(
            command(view.key(SettingsKey::Enter)),
            SettingsCommand::Studio(crate::studio_settings::StudioEdit::DeletePort {
                port_index: 2
            })
        ));
    }

    #[test]
    fn the_wheel_scrolls_the_pane_it_is_given_and_a_click_gives_a_pane_the_keys() {
        let mut view: SettingsView = view_with_studio();
        // With a track chosen its output rows make the studio list longer
        // than the pane.
        view.list_mut().select(3);
        view.key(SettingsKey::Up);
        view.key(SettingsKey::Down);
        view.scroll_pane(super::Pane::Studio, 3);
        assert_eq!(view.studio_list().top(), 3);
        assert_eq!(view.list().top(), 0, "the song pane stays");
        view.scroll_pane(super::Pane::Studio, -9);
        assert_eq!(view.studio_list().top(), 0);
        view.set_pane(super::Pane::Studio);
        assert_eq!(view.pane(), super::Pane::Studio);
    }
}
