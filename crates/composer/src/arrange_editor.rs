//! F1's editing state: which cell of which row is selected, and the dialog
//! open on it, if any: a section's new name, a tempo being typed, the key
//! picker, the meter picker, the list of sections to add an entry for, or
//! the new-section form. Keys, typed text and clicks come in; what they ask
//! of the song leaves as an [`ArrangeRequest`] for the session to write.
//! Nothing here touches the text.

use song_text::contents_text::parse_key;
use vocabulary::key::{Key, Mode};
use vocabulary::pitch::{Accidental, NoteLetter, NoteName};
use vocabulary::time_signature::TimeSignature;

use crate::arrangement::{ArrangeColumn, ArrangeRow, Origin};
use crate::composition_edits::{
    NewSection, beats_per_minute_text, meter_words, milli_beats_per_minute_of,
    tempo_words_from_typed,
};
use crate::key_signature::KeySignature;
use crate::session::MessageKind;
use crate::text_field::TextField;
use crate::timeline::{EntryView, Timeline};

/// The keys F1 reads, as the window maps them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrangeKey {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Plus,
    Minus,
    Delete,
    Backspace,
    Home,
    End,
    /// The next field of the new-section form (Tab).
    NextField,
    /// The selected row one place earlier or later (Alt+Up, Alt+Down).
    MoveRowUp,
    MoveRowDown,
    /// The new-section form (N).
    NewSection,
    /// An entry for a section the song already has (E).
    AddEntry,
    /// A key change line after the selected entry (K).
    AddChange,
    /// The selected entry written again after itself (D).
    Duplicate,
    /// The selected entry left out (repeat 0) or played once again (M).
    ToggleMute,
    /// In an entry's key picker: play its section's own key (S).
    SectionKey,
}

/// What a click in F1 lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrangeTarget {
    Cell {
        row: usize,
        column: ArrangeColumn,
    },
    /// A key in the key picker, or in the new-section form.
    KeyChoice {
        fifths: i8,
        minor: bool,
    },
    /// The "the composition's key" choice: no key of its own.
    NoKey,
    /// An entry's "its section's own key" choice (`key section`).
    SectionKey,
    MeterPreset {
        pulses_per_bar: u8,
        pulses_per_whole_note: u8,
    },
    FormField(FormField),
    /// A section in the add-entry list.
    SectionChoice(usize),
    Confirm,
    Cancel,
    /// The dialog's own background: a click there does nothing, so it
    /// does not reach the rows under the dialog.
    Panel,
}

/// The fields of the new-section form, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Name,
    Meter,
    Key,
    Tempo,
    CountIn,
}

const FORM_FIELDS: [FormField; 5] = [
    FormField::Name,
    FormField::Meter,
    FormField::Key,
    FormField::Tempo,
    FormField::CountIn,
];

/// Keys by their place on the circle of fifths, from 7 flats to 7 sharps:
/// each signature's major key and its relative minor.
const MAJOR_BY_FIFTHS: [(NoteLetter, Accidental); 15] = [
    (NoteLetter::C, Accidental::Flat),
    (NoteLetter::G, Accidental::Flat),
    (NoteLetter::D, Accidental::Flat),
    (NoteLetter::A, Accidental::Flat),
    (NoteLetter::E, Accidental::Flat),
    (NoteLetter::B, Accidental::Flat),
    (NoteLetter::F, Accidental::Natural),
    (NoteLetter::C, Accidental::Natural),
    (NoteLetter::G, Accidental::Natural),
    (NoteLetter::D, Accidental::Natural),
    (NoteLetter::A, Accidental::Natural),
    (NoteLetter::E, Accidental::Natural),
    (NoteLetter::B, Accidental::Natural),
    (NoteLetter::F, Accidental::Sharp),
    (NoteLetter::C, Accidental::Sharp),
];

const MINOR_BY_FIFTHS: [(NoteLetter, Accidental); 15] = [
    (NoteLetter::A, Accidental::Flat),
    (NoteLetter::E, Accidental::Flat),
    (NoteLetter::B, Accidental::Flat),
    (NoteLetter::F, Accidental::Natural),
    (NoteLetter::C, Accidental::Natural),
    (NoteLetter::G, Accidental::Natural),
    (NoteLetter::D, Accidental::Natural),
    (NoteLetter::A, Accidental::Natural),
    (NoteLetter::E, Accidental::Natural),
    (NoteLetter::B, Accidental::Natural),
    (NoteLetter::F, Accidental::Sharp),
    (NoteLetter::C, Accidental::Sharp),
    (NoteLetter::G, Accidental::Sharp),
    (NoteLetter::D, Accidental::Sharp),
    (NoteLetter::A, Accidental::Sharp),
];

/// Seven flats to seven sharps: every key signature the notation writes.
pub const MOST_FLATS: i8 = -7;
pub const MOST_SHARPS: i8 = 7;

/// The key with `fifths` sharps (flats when negative) in `mode`.
#[must_use]
pub fn key_of(fifths: i8, mode: Mode) -> Key {
    let clamped: i8 = fifths.clamp(MOST_FLATS, MOST_SHARPS);
    // −7..=7 shifted by 7 is 0..=14, an index into the tables.
    let index: usize = usize::try_from(clamped - MOST_FLATS).unwrap_or(0);
    let (letter, accidental): (NoteLetter, Accidental) = match mode {
        Mode::Major => MAJOR_BY_FIFTHS[index],
        Mode::Minor => MINOR_BY_FIFTHS[index],
    };
    Key::new(NoteName::new(letter, accidental), mode)
}

/// The key picker's choice: a place on the circle of fifths and a mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyChoice {
    pub fifths: i8,
    pub mode: Mode,
}

impl KeyChoice {
    #[must_use]
    pub fn of(key: Key) -> KeyChoice {
        KeyChoice {
            fifths: KeySignature::of(key)
                .fifths()
                .clamp(MOST_FLATS, MOST_SHARPS),
            mode: key.mode(),
        }
    }

    #[must_use]
    pub fn key(self) -> Key {
        key_of(self.fifths, self.mode)
    }

    fn step(&mut self, fifths: i8) {
        self.fifths = (self.fifths + fifths).clamp(MOST_FLATS, MOST_SHARPS);
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Major => Mode::Minor,
            Mode::Minor => Mode::Major,
        };
    }
}

/// The meters the meter picker offers with one click.
pub const METER_PRESETS: [(u8, u8); 12] = [
    (2, 4),
    (3, 4),
    (4, 4),
    (5, 4),
    (6, 4),
    (2, 2),
    (3, 8),
    (5, 8),
    (6, 8),
    (7, 8),
    (9, 8),
    (12, 8),
];

/// The lower figures a meter may have (Song Notation, section 5).
const LOWER_FIGURES: [u8; 5] = [1, 2, 4, 8, 16];

/// Most pulses a bar may have in the picker; the song file's reader has
/// the last word on what it accepts.
const MOST_PULSES_PER_BAR: u8 = 32;

/// The meter picker's choice: the upper and lower figures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeterChoice {
    pub pulses_per_bar: u8,
    pub pulses_per_whole_note: u8,
}

impl MeterChoice {
    #[must_use]
    pub fn of(meter: TimeSignature) -> MeterChoice {
        MeterChoice {
            pulses_per_bar: meter.pulses_per_bar(),
            pulses_per_whole_note: meter.pulses_per_whole_note(),
        }
    }

    /// # Errors
    ///
    /// When the figures make no meter the notation allows.
    pub fn meter(self) -> Result<TimeSignature, String> {
        TimeSignature::new(self.pulses_per_bar, self.pulses_per_whole_note).map_err(|_| {
            format!(
                "{}/{} is not a meter the song can write. Choose another",
                self.pulses_per_bar, self.pulses_per_whole_note
            )
        })
    }

    fn upper(&mut self, delta: i16) {
        let next: i16 = i16::from(self.pulses_per_bar) + delta;
        self.pulses_per_bar = u8::try_from(next.clamp(1, i16::from(MOST_PULSES_PER_BAR)))
            .unwrap_or(self.pulses_per_bar);
    }

    fn lower(&mut self, delta: isize) {
        let place: usize = LOWER_FIGURES
            .iter()
            .position(|figure: &u8| *figure == self.pulses_per_whole_note)
            .unwrap_or(2);
        if let Some(next) = place
            .checked_add_signed(delta)
            .and_then(|next: usize| LOWER_FIGURES.get(next))
        {
            self.pulses_per_whole_note = *next;
        }
    }
}

/// What a dialog writes when it is confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogPlace {
    /// A cell of a row.
    Cell { row: usize, column: ArrangeColumn },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSectionForm {
    pub name: TextField,
    pub meter: MeterChoice,
    /// `None`: the section plays in the composition's key.
    pub key: Option<KeyChoice>,
    /// Empty: the section plays at the composition's tempo.
    pub tempo: TextField,
    pub count_in: bool,
    pub field: FormField,
    /// The row the new entry goes after.
    pub after_row: usize,
    /// The composition's key at that row, where a key choice starts.
    pub key_there: Option<Key>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrangeDialog {
    Rename {
        section_name: String,
        field: TextField,
    },
    /// A key of the other mode chosen for an entry: a key override keeps
    /// major or minor, so the entry needs a section of its own, made as a
    /// close transposition (Editor Draft, "A key override in F1"). Asks its
    /// name.
    OtherModeSection {
        entry: usize,
        section_name: String,
        section_key: Key,
        key: Key,
        field: TextField,
    },
    Tempo {
        place: DialogPlace,
        field: TextField,
    },
    Key {
        place: DialogPlace,
        choice: KeyChoice,
        can_clear: bool,
        /// An entry's picker: it may play its section's own key.
        can_take_section_key: bool,
    },
    Meter {
        place: DialogPlace,
        choice: MeterChoice,
    },
    AddEntry {
        after_row: usize,
        names: Vec<String>,
        chosen: usize,
    },
    NewSection(NewSectionForm),
}

/// A change F1 asks the session to write into the song.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrangeEdit {
    StartKey(Key),
    StartMeter(TimeSignature),
    StartTempo(String),
    ChangeKey {
        line_index: usize,
        key: Option<Key>,
    },
    ChangeTempo {
        line_index: usize,
        tempo_words: Option<String>,
    },
    EntryKey {
        entry: usize,
        key: Option<Key>,
    },
    /// `key section` on an entry: its section's own key, whatever key line
    /// is above it.
    EntrySectionKey {
        entry: usize,
    },
    /// A copy of the entry's section in `key`, the other mode, named
    /// `new_name`, which the entry then plays.
    SectionInOtherMode {
        entry: usize,
        new_name: String,
        key: Key,
    },
    EntryTempo {
        entry: usize,
        tempo_words: Option<String>,
    },
    EntryRepeat {
        entry: usize,
        repeat_count: u32,
    },
    SectionMeter {
        section_name: String,
        meter: TimeSignature,
    },
    Rename {
        old_name: String,
        new_name: String,
    },
    RemoveEntry(usize),
    RemoveChange(usize),
    MoveRow {
        row: usize,
        delta: isize,
    },
    Duplicate(usize),
    AddChange {
        after_row: usize,
        key: Key,
    },
    AddEntry {
        after_row: usize,
        section_name: String,
    },
    NewSection {
        after_row: usize,
        section: NewSection,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrangeRequest {
    /// Write the edit; when it is kept, select `then_row`.
    Edit {
        edit: ArrangeEdit,
        then_row: Option<usize>,
    },
    Say(MessageKind, String),
}

// Both are the answers of functions that may also answer nothing, so they
// give the `Option` those functions return.
#[allow(clippy::unnecessary_wraps)]
fn say_problem(text: &str) -> Option<ArrangeRequest> {
    Some(ArrangeRequest::Say(MessageKind::Problem, text.to_string()))
}

#[allow(clippy::unnecessary_wraps)]
fn edit(edit: ArrangeEdit, then_row: Option<usize>) -> Option<ArrangeRequest> {
    Some(ArrangeRequest::Edit { edit, then_row })
}

/// What the editor reads of the song: the rows, what each entry plays, and
/// the sections there are.
pub struct ArrangeContext<'a> {
    pub rows: &'a [ArrangeRow],
    pub timeline: &'a Timeline,
    pub section_names: Vec<String>,
}

impl ArrangeContext<'_> {
    fn entry_view(&self, entry: usize) -> Option<&EntryView> {
        self.timeline.entries.get(entry)
    }

    /// The key in effect at a row: what an entry plays, a change line's
    /// key, or the start's, else the nearest one above.
    fn key_at(&self, row: usize) -> Option<Key> {
        (0..=row.min(self.rows.len().saturating_sub(1)))
            .rev()
            .find_map(|index: usize| match &self.rows[index] {
                ArrangeRow::Start(start) => start.key_words.as_deref().and_then(key_from_words),
                ArrangeRow::Change(change) => change.key_words.as_deref().and_then(key_from_words),
                ArrangeRow::Entry(entry) => {
                    self.entry_view(entry.entry_index).and_then(|view| view.key)
                }
            })
    }
}

/// `E minor` as a key.
#[must_use]
pub fn key_from_words(words: &str) -> Option<Key> {
    let (tonic, mode): (&str, &str) = words.split_once(' ')?;
    parse_key(tonic, mode).ok()
}

fn meter_from_words(words: &str) -> Option<TimeSignature> {
    let (upper, lower): (&str, &str) = words.split_once('/')?;
    TimeSignature::new(upper.parse().ok()?, lower.parse().ok()?).ok()
}

/// Which row and cell are selected, and the dialog open on them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrangeEditor {
    pub row: usize,
    pub column: ArrangeColumn,
    pub dialog: Option<ArrangeDialog>,
}

impl Default for ArrangeEditor {
    fn default() -> Self {
        ArrangeEditor {
            row: 1,
            column: ArrangeColumn::Name,
            dialog: None,
        }
    }
}

impl ArrangeEditor {
    /// Keeps the selection on a row and a cell that exist.
    pub fn clamp(&mut self, rows: &[ArrangeRow]) {
        self.row = self.row.min(rows.len().saturating_sub(1));
        if let Some(row) = rows.get(self.row)
            && !row.columns().contains(&self.column)
        {
            self.column = nearest_column(row.columns(), self.column);
        }
    }

    /// The entry the selected row is, if it is one.
    #[must_use]
    pub fn selected_entry(&self, rows: &[ArrangeRow]) -> Option<usize> {
        match rows.get(self.row) {
            Some(ArrangeRow::Entry(entry)) => Some(entry.entry_index),
            _ => None,
        }
    }

    /// Whether a dialog is open: then typed text goes to it.
    #[must_use]
    pub fn has_dialog(&self) -> bool {
        self.dialog.is_some()
    }

    pub fn close_dialog(&mut self) {
        self.dialog = None;
    }

    /// A key press.
    pub fn key(&mut self, key: ArrangeKey, context: &ArrangeContext<'_>) -> Option<ArrangeRequest> {
        if self.dialog.is_some() {
            return self.dialog_key(key, context);
        }
        match key {
            ArrangeKey::Up => self.step_row(-1, context.rows),
            ArrangeKey::Down => self.step_row(1, context.rows),
            ArrangeKey::Left => self.step_column(-1, context.rows),
            ArrangeKey::Right => self.step_column(1, context.rows),
            ArrangeKey::Enter => return self.open_cell(context),
            ArrangeKey::Plus => return self.adjust(1, context),
            ArrangeKey::Minus => return self.adjust(-1, context),
            ArrangeKey::Delete => return self.clear_cell(context),
            ArrangeKey::MoveRowUp => {
                return edit(
                    ArrangeEdit::MoveRow {
                        row: self.row,
                        delta: -1,
                    },
                    self.row.checked_sub(1),
                );
            }
            ArrangeKey::MoveRowDown => {
                return edit(
                    ArrangeEdit::MoveRow {
                        row: self.row,
                        delta: 1,
                    },
                    Some(self.row + 1),
                );
            }
            ArrangeKey::NewSection => self.open_new_section(context),
            ArrangeKey::AddEntry => {
                if context.section_names.is_empty() {
                    return say_problem("The song has no sections yet. Press N to make one.");
                }
                self.dialog = Some(ArrangeDialog::AddEntry {
                    after_row: self.row,
                    names: context.section_names.clone(),
                    chosen: 0,
                });
            }
            ArrangeKey::AddChange => {
                let Some(key) = context.key_at(self.row) else {
                    return say_problem("There is no key here to start the change from.");
                };
                if matches!(context.rows.get(self.row), Some(ArrangeRow::Start(_))) {
                    return say_problem(
                        "A key or tempo change goes after an entry. Select an entry first.",
                    );
                }
                self.column = ArrangeColumn::Key;
                return edit(
                    ArrangeEdit::AddChange {
                        after_row: self.row,
                        key,
                    },
                    Some(self.row + 1),
                );
            }
            ArrangeKey::Duplicate => {
                let Some(entry) = self.selected_entry(context.rows) else {
                    return say_problem("Select an entry to write it again.");
                };
                return edit(ArrangeEdit::Duplicate(entry), Some(self.row + 1));
            }
            ArrangeKey::ToggleMute => {
                let Some(view) = self
                    .selected_entry(context.rows)
                    .and_then(|entry: usize| context.entry_view(entry).map(|view| (entry, view)))
                else {
                    return say_problem("Select an entry to leave it out or put it back.");
                };
                let repeat_count: u32 = u32::from(view.1.muted);
                return edit(
                    ArrangeEdit::EntryRepeat {
                        entry: view.0,
                        repeat_count,
                    },
                    None,
                );
            }
            ArrangeKey::Escape
            | ArrangeKey::Backspace
            | ArrangeKey::Home
            | ArrangeKey::End
            | ArrangeKey::NextField
            | ArrangeKey::SectionKey => {}
        }
        None
    }

    fn open_new_section(&mut self, context: &ArrangeContext<'_>) {
        self.dialog = Some(ArrangeDialog::NewSection(NewSectionForm {
            name: TextField::new(""),
            meter: MeterChoice {
                pulses_per_bar: 4,
                pulses_per_whole_note: 4,
            },
            key: None,
            tempo: TextField::new(""),
            count_in: false,
            field: FormField::Name,
            after_row: self.row,
            key_there: context.key_at(self.row),
        }));
    }

    fn step_row(&mut self, delta: isize, rows: &[ArrangeRow]) {
        if let Some(row) = self
            .row
            .checked_add_signed(delta)
            .filter(|row: &usize| *row < rows.len())
        {
            self.row = row;
            self.clamp(rows);
        }
    }

    fn step_column(&mut self, delta: isize, rows: &[ArrangeRow]) {
        let Some(row) = rows.get(self.row) else {
            return;
        };
        let columns: &[ArrangeColumn] = row.columns();
        let place: usize = columns
            .iter()
            .position(|column: &ArrangeColumn| *column == self.column)
            .unwrap_or(0);
        if let Some(column) = place
            .checked_add_signed(delta)
            .and_then(|next: usize| columns.get(next))
        {
            self.column = *column;
        }
    }

    /// Enter on a cell, or a click that edits it at once.
    fn open_cell(&mut self, context: &ArrangeContext<'_>) -> Option<ArrangeRequest> {
        let row: &ArrangeRow = context.rows.get(self.row)?;
        let place: DialogPlace = DialogPlace::Cell {
            row: self.row,
            column: self.column,
        };
        match (row, self.column) {
            (ArrangeRow::Entry(entry), ArrangeColumn::Name) => {
                self.dialog = Some(ArrangeDialog::Rename {
                    section_name: entry.section_name.clone(),
                    field: TextField::new(&entry.section_name),
                });
                None
            }
            (ArrangeRow::Change(_), ArrangeColumn::Name) => {
                say_problem("A change line has no name. Delete removes it.")
            }
            (_, ArrangeColumn::Plays) => {
                say_problem("Press + or - to play the entry more or fewer times.")
            }
            (_, ArrangeColumn::Key) => {
                let key: Option<Key> = match row {
                    ArrangeRow::Change(change) => {
                        change.key_words.as_deref().and_then(key_from_words)
                    }
                    _ => None,
                }
                .or_else(|| context.key_at(self.row));
                let choice: KeyChoice = key.map_or(
                    KeyChoice {
                        fifths: 0,
                        mode: Mode::Major,
                    },
                    KeyChoice::of,
                );
                self.dialog = Some(ArrangeDialog::Key {
                    place,
                    choice,
                    can_clear: !matches!(row, ArrangeRow::Start(_)),
                    can_take_section_key: matches!(row, ArrangeRow::Entry(_)),
                });
                None
            }
            (ArrangeRow::Start(start), ArrangeColumn::Meter) => {
                let meter: Option<TimeSignature> =
                    start.meter_words.as_deref().and_then(meter_from_words);
                self.open_meter(place, meter);
                None
            }
            (ArrangeRow::Entry(entry), ArrangeColumn::Meter) => {
                if !entry.meter_editable {
                    return say_problem(&format!(
                        "\"{}\" has music written for its meter. Change its bars in the Section Editor (F3).",
                        entry.section_name
                    ));
                }
                let meter: Option<TimeSignature> = context
                    .entry_view(entry.entry_index)
                    .and_then(|view| view.meter);
                self.open_meter(place, meter);
                None
            }
            (_, ArrangeColumn::Tempo) => {
                let milli: Option<u64> = self.tempo_here(context);
                self.dialog = Some(ArrangeDialog::Tempo {
                    place,
                    field: TextField::new(&milli.map_or_else(String::new, beats_per_minute_text)),
                });
                None
            }
            (ArrangeRow::Start(_), ArrangeColumn::Name)
            | (ArrangeRow::Change(_), ArrangeColumn::Meter) => None,
        }
    }

    fn open_meter(&mut self, place: DialogPlace, meter: Option<TimeSignature>) {
        self.dialog = Some(ArrangeDialog::Meter {
            place,
            choice: meter.map_or(
                MeterChoice {
                    pulses_per_bar: 4,
                    pulses_per_whole_note: 4,
                },
                MeterChoice::of,
            ),
        });
    }

    /// The tempo the selected row writes or plays, in milli-beats/min.
    fn tempo_here(&self, context: &ArrangeContext<'_>) -> Option<u64> {
        match context.rows.get(self.row)? {
            ArrangeRow::Start(start) => start
                .tempo_words
                .as_deref()
                .and_then(milli_beats_per_minute_of),
            ArrangeRow::Change(change) => change
                .tempo_words
                .as_deref()
                .and_then(milli_beats_per_minute_of),
            ArrangeRow::Entry(entry) => context
                .entry_view(entry.entry_index)
                .map(|view: &EntryView| view.milli_beats_per_minute),
        }
    }

    /// + or − on the selected cell: plays by one, tempo by 1 beat/min.
    fn adjust(&mut self, delta: i64, context: &ArrangeContext<'_>) -> Option<ArrangeRequest> {
        let row: &ArrangeRow = context.rows.get(self.row)?;
        match (row, self.column) {
            (ArrangeRow::Entry(entry), ArrangeColumn::Plays) => {
                let view: &EntryView = context.entry_view(entry.entry_index)?;
                let now: i64 = if view.muted {
                    0
                } else {
                    i64::from(view.repeat_count)
                };
                let repeat_count: u32 = u32::try_from((now + delta).max(0)).unwrap_or(0);
                edit(
                    ArrangeEdit::EntryRepeat {
                        entry: entry.entry_index,
                        repeat_count,
                    },
                    None,
                )
            }
            (_, ArrangeColumn::Tempo) => {
                let Some(milli) = self.tempo_here(context) else {
                    return say_problem(
                        "This line has no tempo in beats/min to change. Press Enter to type one.",
                    );
                };
                // 1 beat/min = 1000 milli-beats/min.
                let next: i64 = i64::try_from(milli).unwrap_or(i64::MAX) + delta * 1000;
                if next < 1000 {
                    return say_problem("The tempo cannot go below 1 beat/min.");
                }
                let words: String = format!(
                    "{} beats/min",
                    beats_per_minute_text(u64::try_from(next).unwrap_or(0))
                );
                Self::write_tempo(row, Some(words))
            }
            (_, ArrangeColumn::Key) => say_problem("Press Enter or click the key to choose one."),
            (_, ArrangeColumn::Meter) => {
                say_problem("Press Enter or click the meter to choose one.")
            }
            (_, ArrangeColumn::Name | ArrangeColumn::Plays) => None,
        }
    }

    fn write_tempo(row: &ArrangeRow, tempo_words: Option<String>) -> Option<ArrangeRequest> {
        match row {
            ArrangeRow::Start(_) => match tempo_words {
                Some(words) => edit(ArrangeEdit::StartTempo(words), None),
                None => say_problem("The song's start needs a tempo."),
            },
            ArrangeRow::Change(change) => edit(
                ArrangeEdit::ChangeTempo {
                    line_index: change.line_index,
                    tempo_words,
                },
                None,
            ),
            ArrangeRow::Entry(entry) => edit(
                ArrangeEdit::EntryTempo {
                    entry: entry.entry_index,
                    tempo_words,
                },
                None,
            ),
        }
    }

    /// A click on an entry's "the section's own key" choice.
    fn click_section_key(&mut self, context: &ArrangeContext<'_>) -> Option<ArrangeRequest> {
        let ArrangeDialog::Key {
            can_take_section_key: true,
            place: DialogPlace::Cell { row, column },
            ..
        } = self.dialog.as_ref()?
        else {
            return None;
        };
        let (row, column): (usize, ArrangeColumn) = (*row, *column);
        self.dialog = None;
        self.row = row;
        self.column = column;
        Self::write_section_key(context.rows.get(row)?)
    }

    /// `key section` on an entry's row.
    fn write_section_key(row: &ArrangeRow) -> Option<ArrangeRequest> {
        match row {
            ArrangeRow::Entry(entry) => edit(
                ArrangeEdit::EntrySectionKey {
                    entry: entry.entry_index,
                },
                None,
            ),
            ArrangeRow::Start(_) | ArrangeRow::Change(_) => {
                say_problem("Only an entry can play its section's own key.")
            }
        }
    }

    fn write_key(row: &ArrangeRow, key: Option<Key>) -> Option<ArrangeRequest> {
        match row {
            ArrangeRow::Start(_) => match key {
                Some(key) => edit(ArrangeEdit::StartKey(key), None),
                None => say_problem("The song's start needs a key."),
            },
            ArrangeRow::Change(change) => edit(
                ArrangeEdit::ChangeKey {
                    line_index: change.line_index,
                    key,
                },
                None,
            ),
            ArrangeRow::Entry(entry) => edit(
                ArrangeEdit::EntryKey {
                    entry: entry.entry_index,
                    key,
                },
                None,
            ),
        }
    }

    /// Delete on the selected cell: an entry's or change line's own key or
    /// tempo goes (the entry then plays what it inherits); on the name, the
    /// row goes.
    fn clear_cell(&mut self, context: &ArrangeContext<'_>) -> Option<ArrangeRequest> {
        let row: &ArrangeRow = context.rows.get(self.row)?;
        match (row, self.column) {
            (ArrangeRow::Start(_), _) => {
                say_problem("The song's start keeps its key, meter and tempo. Change them instead.")
            }
            (ArrangeRow::Entry(entry), ArrangeColumn::Name) => {
                edit(ArrangeEdit::RemoveEntry(entry.entry_index), None)
            }
            (ArrangeRow::Change(change), ArrangeColumn::Name) => {
                edit(ArrangeEdit::RemoveChange(change.line_index), None)
            }
            (ArrangeRow::Entry(entry), ArrangeColumn::Key) => {
                if entry.key_origin != Origin::Entry {
                    return say_problem("This entry has no key of its own to clear.");
                }
                Self::write_key(row, None)
            }
            (ArrangeRow::Entry(entry), ArrangeColumn::Tempo) => {
                if entry.tempo_origin != Origin::Entry {
                    return say_problem("This entry has no tempo of its own to clear.");
                }
                Self::write_tempo(row, None)
            }
            (ArrangeRow::Change(change), ArrangeColumn::Key) => edit(
                ArrangeEdit::ChangeKey {
                    line_index: change.line_index,
                    key: None,
                },
                None,
            ),
            (ArrangeRow::Change(change), ArrangeColumn::Tempo) => edit(
                ArrangeEdit::ChangeTempo {
                    line_index: change.line_index,
                    tempo_words: None,
                },
                None,
            ),
            (_, ArrangeColumn::Plays) => {
                say_problem("Press - to play the entry fewer times; at 0 it is left out.")
            }
            (_, ArrangeColumn::Meter) => {
                say_problem("A section always has a meter. Press Enter to choose another.")
            }
        }
    }

    /// Text typed while a dialog is open.
    pub fn type_text(&mut self, typed: &str) {
        let Some(dialog) = self.dialog.as_mut() else {
            return;
        };
        match dialog {
            ArrangeDialog::Rename { field, .. } | ArrangeDialog::OtherModeSection { field, .. } => {
                field.insert(typed);
            }
            ArrangeDialog::Tempo { field, .. } => type_tempo(field, typed),
            ArrangeDialog::Meter { choice, .. } => match typed {
                "+" => choice.upper(1),
                "-" => choice.upper(-1),
                _ => {}
            },
            ArrangeDialog::Key { choice, .. } => {
                if typed == " " || typed.eq_ignore_ascii_case("m") {
                    choice.toggle_mode();
                }
            }
            ArrangeDialog::AddEntry { .. } => {}
            ArrangeDialog::NewSection(form) => match form.field {
                FormField::Name => form.name.insert(typed),
                FormField::Tempo => type_tempo(&mut form.tempo, typed),
                FormField::Meter => match typed {
                    "+" => form.meter.upper(1),
                    "-" => form.meter.upper(-1),
                    _ => {}
                },
                FormField::Key => {
                    if typed == " " || typed.eq_ignore_ascii_case("m") {
                        let mut choice: KeyChoice =
                            form.key.unwrap_or_else(|| start_choice(form.key_there));
                        choice.toggle_mode();
                        form.key = Some(choice);
                    }
                }
                FormField::CountIn => {
                    if typed == " " {
                        form.count_in = !form.count_in;
                    }
                }
            },
        }
    }

    fn dialog_key(
        &mut self,
        key: ArrangeKey,
        context: &ArrangeContext<'_>,
    ) -> Option<ArrangeRequest> {
        if key == ArrangeKey::Escape {
            self.dialog = None;
            return None;
        }
        if key == ArrangeKey::Enter {
            return self.confirm(context);
        }
        let dialog: &mut ArrangeDialog = self.dialog.as_mut()?;
        match dialog {
            ArrangeDialog::Rename { field, .. }
            | ArrangeDialog::Tempo { field, .. }
            | ArrangeDialog::OtherModeSection { field, .. } => {
                edit_field(field, key);
            }
            ArrangeDialog::Key {
                choice,
                can_clear,
                can_take_section_key,
                place,
            } => match key {
                ArrangeKey::SectionKey if *can_take_section_key => {
                    let DialogPlace::Cell { row, column } = *place;
                    self.dialog = None;
                    self.row = row;
                    self.column = column;
                    return Self::write_section_key(context.rows.get(row)?);
                }
                ArrangeKey::Left => choice.step(-1),
                ArrangeKey::Right => choice.step(1),
                ArrangeKey::Up | ArrangeKey::Down => choice.toggle_mode(),
                ArrangeKey::Delete if *can_clear => {
                    let place: DialogPlace = *place;
                    self.dialog = None;
                    return self.cell_edit(place, CellValue::Key(None), context);
                }
                _ => {}
            },
            ArrangeDialog::Meter { choice, .. } => match key {
                ArrangeKey::Up | ArrangeKey::Plus => choice.upper(1),
                ArrangeKey::Down | ArrangeKey::Minus => choice.upper(-1),
                ArrangeKey::Left => choice.lower(-1),
                ArrangeKey::Right => choice.lower(1),
                _ => {}
            },
            ArrangeDialog::AddEntry { names, chosen, .. } => match key {
                ArrangeKey::Up => *chosen = chosen.saturating_sub(1),
                ArrangeKey::Down => *chosen = (*chosen + 1).min(names.len().saturating_sub(1)),
                _ => {}
            },
            ArrangeDialog::NewSection(form) => form_key(form, key),
        }
        None
    }

    /// Enter in a dialog, or its OK button.
    fn confirm(&mut self, context: &ArrangeContext<'_>) -> Option<ArrangeRequest> {
        let dialog: ArrangeDialog = self.dialog.take()?;
        match dialog {
            ArrangeDialog::Rename {
                section_name,
                field,
            } => edit(
                ArrangeEdit::Rename {
                    old_name: section_name,
                    new_name: field.text().to_string(),
                },
                None,
            ),
            ArrangeDialog::OtherModeSection {
                entry, key, field, ..
            } => edit(
                ArrangeEdit::SectionInOtherMode {
                    entry,
                    new_name: field.text().to_string(),
                    key,
                },
                None,
            ),
            ArrangeDialog::Tempo { place, field } => match tempo_words_from_typed(field.text()) {
                Ok(words) => self.cell_edit(place, CellValue::Tempo(Some(words)), context),
                Err(problem) => {
                    self.dialog = Some(ArrangeDialog::Tempo { place, field });
                    say_problem(&format!("{problem}."))
                }
            },
            ArrangeDialog::Key { place, choice, .. } => {
                self.cell_edit(place, CellValue::Key(Some(choice.key())), context)
            }
            ArrangeDialog::Meter { place, choice } => match choice.meter() {
                Ok(meter) => self.cell_edit(place, CellValue::Meter(meter), context),
                Err(problem) => {
                    self.dialog = Some(ArrangeDialog::Meter { place, choice });
                    say_problem(&format!("{problem}."))
                }
            },
            ArrangeDialog::AddEntry {
                after_row,
                names,
                chosen,
            } => {
                let section_name: String = names.get(chosen)?.clone();
                edit(
                    ArrangeEdit::AddEntry {
                        after_row,
                        section_name,
                    },
                    Some(after_row + 1),
                )
            }
            ArrangeDialog::NewSection(form) => {
                let meter: Result<TimeSignature, String> = form.meter.meter();
                let tempo: Result<Option<String>, String> = if form.tempo.text().trim().is_empty() {
                    Ok(None)
                } else {
                    tempo_words_from_typed(form.tempo.text()).map(Some)
                };
                match (meter, tempo) {
                    (Ok(meter), Ok(tempo_words)) => {
                        let after_row: usize = form.after_row;
                        let section: NewSection = NewSection {
                            name: form.name.text().to_string(),
                            meter,
                            // A section with a pitched line writes its own
                            // key (Song Notation, section 5): left as the
                            // composition's, it is written as the key in force
                            // there. A count-in needs none.
                            key: form.key.map(KeyChoice::key).or(if form.count_in {
                                None
                            } else {
                                form.key_there
                            }),
                            tempo_words,
                            count_in: form.count_in,
                        };
                        // Kept open until the song takes it, so a refused
                        // name can be corrected.
                        self.dialog = Some(ArrangeDialog::NewSection(form));
                        edit(
                            ArrangeEdit::NewSection { after_row, section },
                            Some(after_row + 1),
                        )
                    }
                    (Err(problem), _) | (_, Err(problem)) => {
                        self.dialog = Some(ArrangeDialog::NewSection(form));
                        say_problem(&format!("{problem}."))
                    }
                }
            }
        }
    }

    /// The edit a dialog's value makes in the cell it was opened on.
    fn cell_edit(
        &mut self,
        place: DialogPlace,
        value: CellValue,
        context: &ArrangeContext<'_>,
    ) -> Option<ArrangeRequest> {
        let DialogPlace::Cell { row, column } = place;
        self.row = row;
        self.column = column;
        let written: &ArrangeRow = context.rows.get(row)?;
        match value {
            // An entry given a key of the other mode needs a section of its
            // own: ask for its name.
            CellValue::Key(Some(key))
                if let ArrangeRow::Entry(entry) = written
                    && let Some(section_key) = context
                        .entry_view(entry.entry_index)
                        .and_then(|view: &EntryView| view.section_key)
                    && section_key.mode() != key.mode() =>
            {
                self.dialog = Some(ArrangeDialog::OtherModeSection {
                    entry: entry.entry_index,
                    section_name: entry.section_name.clone(),
                    section_key,
                    key,
                    field: TextField::new(&format!("{} ({key})", entry.section_name)),
                });
                None
            }
            CellValue::Key(key) => Self::write_key(written, key),
            CellValue::Tempo(words) => Self::write_tempo(written, words),
            CellValue::Meter(meter) => match written {
                ArrangeRow::Start(_) => edit(ArrangeEdit::StartMeter(meter), None),
                ArrangeRow::Entry(entry) => edit(
                    ArrangeEdit::SectionMeter {
                        section_name: entry.section_name.clone(),
                        meter,
                    },
                    None,
                ),
                ArrangeRow::Change(_) => say_problem(
                    "The meter belongs to each section; a change line cannot change it.",
                ),
            },
        }
    }

    /// A click.
    pub fn click(
        &mut self,
        target: ArrangeTarget,
        double: bool,
        context: &ArrangeContext<'_>,
    ) -> Option<ArrangeRequest> {
        match target {
            ArrangeTarget::Cell { row, column } => {
                self.dialog = None;
                let already: bool = self.row == row && self.column == column;
                self.row = row;
                self.column = column;
                self.clamp(context.rows);
                // Key, meter and tempo open at once; a name opens on a
                // second click or a double-click, so a click can select the
                // row; plays take + and −.
                let opens: bool = match column {
                    ArrangeColumn::Key | ArrangeColumn::Meter | ArrangeColumn::Tempo => true,
                    ArrangeColumn::Name => double || already,
                    ArrangeColumn::Plays => false,
                };
                if opens && self.column == column {
                    return self.open_cell(context);
                }
                None
            }
            ArrangeTarget::KeyChoice { fifths, minor } => {
                let mode: Mode = if minor { Mode::Minor } else { Mode::Major };
                let choice: KeyChoice = KeyChoice { fifths, mode };
                match self.dialog.as_mut()? {
                    ArrangeDialog::Key { choice: chosen, .. } => {
                        *chosen = choice;
                        self.confirm(context)
                    }
                    ArrangeDialog::NewSection(form) => {
                        form.key = Some(choice);
                        form.field = FormField::Key;
                        None
                    }
                    _ => None,
                }
            }
            ArrangeTarget::SectionKey => self.click_section_key(context),
            ArrangeTarget::NoKey => match self.dialog.as_mut()? {
                ArrangeDialog::Key {
                    can_clear: true,
                    place,
                    ..
                } => {
                    let place: DialogPlace = *place;
                    self.dialog = None;
                    self.cell_edit(place, CellValue::Key(None), context)
                }
                ArrangeDialog::NewSection(form) => {
                    form.key = None;
                    form.field = FormField::Key;
                    None
                }
                _ => None,
            },
            ArrangeTarget::MeterPreset {
                pulses_per_bar,
                pulses_per_whole_note,
            } => {
                let choice: MeterChoice = MeterChoice {
                    pulses_per_bar,
                    pulses_per_whole_note,
                };
                match self.dialog.as_mut()? {
                    ArrangeDialog::Meter { choice: chosen, .. } => {
                        *chosen = choice;
                        self.confirm(context)
                    }
                    ArrangeDialog::NewSection(form) => {
                        form.meter = choice;
                        form.field = FormField::Meter;
                        None
                    }
                    _ => None,
                }
            }
            ArrangeTarget::FormField(field) => {
                if let Some(ArrangeDialog::NewSection(form)) = self.dialog.as_mut() {
                    if field == FormField::CountIn && form.field == FormField::CountIn {
                        form.count_in = !form.count_in;
                    }
                    form.field = field;
                }
                None
            }
            ArrangeTarget::SectionChoice(index) => {
                if let Some(ArrangeDialog::AddEntry { chosen, .. }) = self.dialog.as_mut() {
                    *chosen = index;
                    if double {
                        return self.confirm(context);
                    }
                }
                None
            }
            ArrangeTarget::Confirm => self.confirm(context),
            ArrangeTarget::Cancel => {
                self.dialog = None;
                None
            }
            ArrangeTarget::Panel => None,
        }
    }
}

/// A value a dialog gives for a cell.
enum CellValue {
    Key(Option<Key>),
    Tempo(Option<String>),
    Meter(TimeSignature),
}

fn start_choice(key_there: Option<Key>) -> KeyChoice {
    key_there.map_or(
        KeyChoice {
            fifths: 0,
            mode: Mode::Major,
        },
        KeyChoice::of,
    )
}

/// Typing in a tempo field: digits and a point go in; + and − move the
/// number by 1 beat/min.
fn type_tempo(field: &mut TextField, typed: &str) {
    match typed {
        "+" | "-" => {
            let delta: i64 = if typed == "+" { 1000 } else { -1000 };
            let now: Option<u64> = tempo_words_from_typed(field.text())
                .ok()
                .and_then(|words: String| milli_beats_per_minute_of(&words));
            if let Some(milli) = now {
                let next: i64 = (i64::try_from(milli).unwrap_or(i64::MAX) + delta).max(1000);
                *field =
                    TextField::new(&beats_per_minute_text(u64::try_from(next).unwrap_or(1000)));
            }
        }
        _ => {
            let kept: String = typed
                .chars()
                .filter(|character: &char| character.is_ascii_digit() || *character == '.')
                .collect();
            field.insert(&kept);
        }
    }
}

fn edit_field(field: &mut TextField, key: ArrangeKey) {
    match key {
        ArrangeKey::Backspace => field.backspace(),
        ArrangeKey::Delete => field.delete(),
        ArrangeKey::Left => field.left(),
        ArrangeKey::Right => field.right(),
        ArrangeKey::Home => field.home(),
        ArrangeKey::End => field.end(),
        _ => {}
    }
}

fn form_key(form: &mut NewSectionForm, key: ArrangeKey) {
    let place: usize = FORM_FIELDS
        .iter()
        .position(|field: &FormField| *field == form.field)
        .unwrap_or(0);
    let step_field = |delta: isize| -> FormField {
        place
            .checked_add_signed(delta)
            .and_then(|next: usize| FORM_FIELDS.get(next))
            .copied()
            .unwrap_or(FORM_FIELDS[place])
    };
    match key {
        ArrangeKey::Up => form.field = step_field(-1),
        ArrangeKey::Down => form.field = step_field(1),
        ArrangeKey::NextField => {
            form.field = FORM_FIELDS[(place + 1) % FORM_FIELDS.len()];
        }
        _ => match form.field {
            FormField::Name => edit_field(&mut form.name, key),
            FormField::Tempo => edit_field(&mut form.tempo, key),
            FormField::Meter => match key {
                ArrangeKey::Left => form.meter.lower(-1),
                ArrangeKey::Right => form.meter.lower(1),
                ArrangeKey::Plus => form.meter.upper(1),
                ArrangeKey::Minus => form.meter.upper(-1),
                _ => {}
            },
            FormField::Key => match key {
                ArrangeKey::Left | ArrangeKey::Right => {
                    let mut choice: KeyChoice =
                        form.key.unwrap_or_else(|| start_choice(form.key_there));
                    choice.step(if key == ArrangeKey::Left { -1 } else { 1 });
                    form.key = Some(choice);
                }
                ArrangeKey::Delete | ArrangeKey::Backspace => form.key = None,
                _ => {}
            },
            FormField::CountIn => {}
        },
    }
}

/// The meter as the screen writes it.
#[must_use]
pub fn meter_text(choice: MeterChoice) -> String {
    choice.meter().map_or_else(
        |_| format!("{}/{}", choice.pulses_per_bar, choice.pulses_per_whole_note),
        meter_words,
    )
}

/// The row's cell nearest `wanted`, left to right in screen order, for a
/// row that lacks `wanted` (a change line has no meter).
fn nearest_column(columns: &[ArrangeColumn], wanted: ArrangeColumn) -> ArrangeColumn {
    const ORDER: [ArrangeColumn; 5] = [
        ArrangeColumn::Name,
        ArrangeColumn::Plays,
        ArrangeColumn::Key,
        ArrangeColumn::Meter,
        ArrangeColumn::Tempo,
    ];
    let place = |column: ArrangeColumn| -> usize {
        ORDER
            .iter()
            .position(|ordered: &ArrangeColumn| *ordered == column)
            .unwrap_or(0)
    };
    columns
        .iter()
        .copied()
        .min_by_key(|column: &ArrangeColumn| place(*column).abs_diff(place(wanted)))
        .unwrap_or(wanted)
}
