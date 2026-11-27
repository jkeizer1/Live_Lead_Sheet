//! The editor's state while the musician composes: the mode they work in,
//! what is selected, where the library browser is, and a preview of what
//! the library row under the cursor would do. Keys and clicks arrive as
//! [`Command`]s; nothing here knows about windows.
//!
//! Browsing is free: moving through a library shows its row applied to the
//! selected bars at once, as a preview that changes nothing until Enter
//! applies it (Esc drops it). The preview is a whole song, so it can also
//! be played before it is applied.

use vocabulary::key::Key;
use vocabulary::progression::Progression;
use vocabulary::rhythm_pattern::RhythmPattern;
use vocabulary::song::{Section, Song, Track, TrackKind};
use vocabulary::time_signature::TimeSignature;

use crate::apply::RhythmBars;
use crate::arrange_editor::{
    ArrangeContext, ArrangeEditor, ArrangeKey, ArrangeRequest, ArrangeTarget,
};
use crate::arrange_writer::write_arrange_edit;
use crate::arrangement::{ArrangeRow, arrangement_rows};
use crate::edits::{
    duplicate_entry, move_entry, remove_entry, set_entry_key, set_entry_repeat, set_entry_tempo,
    set_section_key, set_section_tempo,
};
use crate::key_signature::{key_by_fifths, semitones_between, transposed_key};
use crate::library::{Libraries, LibraryRow, LibraryTab};
use crate::session_music::{
    Step, Target, apply_progression, apply_rhythm, bass_from_roots, cycle_accent, is_sampled,
    make_coupled, make_sampled, move_contents, move_to_chord_tones, new_section_from_progression,
    toggle_rest, transpose_section_notes,
};
use crate::song_document::{SongDocument, read_song};
use crate::song_source::SongSource;
use crate::timeline::{EntryView, Timeline};

/// What the musician works on, in the order of the song's hierarchy: the
/// sections in the composition, a section, the lines of a section, then
/// the song's harmony and a track's own harmony. Each mode has its own
/// view and library tab and says where an edit goes. Whether a line is
/// coupled or sampled belongs to the composed line, not to a mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The `song_composition` block: the song's start, its key and tempo
    /// changes, and its entries (which section, how many times, in which
    /// key and at which tempo). New sections are made here, not composed.
    ArrangeSongSections,
    /// One section: its key and tempo, its lines' forms.
    ComposeSections,
    /// The Section Editor: the music on a section's lines, note by note
    /// on the staff, and its bars.
    SectionEditor,
    /// The song's harmony: the lead line's chords.
    SongHarmony,
    /// One track's own harmony: a sampled line's source, or a chord
    /// track's chords.
    TrackHarmony,
}

impl Mode {
    pub const ALL: [Mode; 5] = [
        Mode::ArrangeSongSections,
        Mode::ComposeSections,
        Mode::SectionEditor,
        Mode::SongHarmony,
        Mode::TrackHarmony,
    ];

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Mode::ArrangeSongSections => "Arrange Song Sections",
            Mode::ComposeSections => "Compose Sections",
            Mode::SectionEditor => "Section Editor",
            Mode::SongHarmony => "Song Harmony",
            Mode::TrackHarmony => "Track Harmony",
        }
    }

    /// The function key that picks it.
    #[must_use]
    pub const fn key_name(self) -> &'static str {
        match self {
            Mode::ArrangeSongSections => "F1",
            Mode::ComposeSections => "F2",
            Mode::SectionEditor => "F3",
            Mode::SongHarmony => "F4",
            Mode::TrackHarmony => "F5",
        }
    }

    /// The view the mode renders.
    #[must_use]
    pub const fn view(self) -> Zoom {
        match self {
            Mode::ArrangeSongSections => Zoom::Form,
            Mode::ComposeSections => Zoom::Tracks,
            Mode::SectionEditor | Mode::TrackHarmony => Zoom::Staff,
            Mode::SongHarmony => Zoom::Harmony,
        }
    }

    const fn tab(self) -> LibraryTab {
        match self {
            Mode::SectionEditor => LibraryTab::Rhythms,
            Mode::ArrangeSongSections
            | Mode::ComposeSections
            | Mode::SongHarmony
            | Mode::TrackHarmony => LibraryTab::Progressions,
        }
    }
}

/// The view a mode renders, from the whole form down to the staff a
/// musician reads to play along. Chosen by the mode, never on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Zoom {
    Form,
    Harmony,
    Tracks,
    Staff,
}

/// Where the arrow keys act.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Main,
    Library,
}

/// What is selected: an entry of the composition, a range of its bars, a
/// voice track, and (on the staff) one element of that track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub entry: usize,
    pub first_bar: usize,
    pub last_bar: usize,
    pub track: usize,
    /// The element's place among the track's elements in the section.
    pub element: usize,
    /// Which time through a repeated entry the staff shows the selection
    /// in, 1 the first. The bars are the section's, the same each time, so
    /// an edit changes every time through; this only says where to look.
    pub pass_number: u32,
}

impl Selection {
    #[must_use]
    pub fn bar_count(&self) -> usize {
        self.last_bar - self.first_bar + 1
    }

    #[must_use]
    pub fn contains_bar(&self, entry: usize, bar: usize) -> bool {
        entry == self.entry && (self.first_bar..=self.last_bar).contains(&bar)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Done,
    Problem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub kind: MessageKind,
    pub text: String,
    /// Counts the messages said so far, so the program can tell a new
    /// message from the same one still showing (to log each one once).
    pub number: u64,
}

/// A library row applied as a preview, not yet in the song.
pub struct Preview {
    pub label: String,
    pub text: String,
    pub song: Song,
    pub timeline: Timeline,
}

/// What a key or click asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    SetMode(Mode),
    /// The staff's bars wider (fewer across) or narrower (more across), by
    /// steps of a quarter.
    StaffBarWidth(i8),
    /// Between the main view and the library browser.
    SwitchPane,
    SetTab(LibraryTab),
    NextTab,
    Left,
    Right,
    Up,
    Down,
    /// Grow or shrink the bar range (Shift+arrows).
    ExtendLeft,
    ExtendRight,
    /// Apply the preview to the song (Enter).
    Commit,
    /// Drop the preview, or the bar range (Esc).
    Cancel,
    /// Move what is selected by an interval.
    Move(Step),
    /// The selected notes by tones of the lead's chord, up or down.
    ChordTone(i16),
    /// The selected section's key (or, in the form view, the entry's) by
    /// semitones, written notes moving with it.
    KeyBySemitones(i16),
    /// The same, round the circle of fifths.
    KeyByFifths(i16),
    /// Tempo by beats per minute.
    Tempo(i16),
    Repeat(i16),
    ToggleMute,
    MoveEntry(i16),
    DuplicateEntry,
    RemoveEntry,
    CycleAccent,
    /// The selected rest becomes a note (or a note a rest), same length.
    ToggleRest,
    /// The selected track's line in the section changes between coupled
    /// and sampled. The first press asks; the second converts.
    ConvertLineForm,
    BassFromRoots,
    NewSectionFromProgression,
    Undo,
    Redo,
    SelectEntry(usize),
    SelectBar {
        entry: usize,
        bar: usize,
    },
    SelectTrack(usize),
    SelectElement {
        entry: usize,
        bar: usize,
        track: usize,
        element: usize,
        pass_number: u32,
    },
    SelectLibraryRow(usize),
    /// A key in F1 (Arrange Song Sections).
    Arrange(ArrangeKey),
    /// A click in F1.
    ArrangeClick {
        target: ArrangeTarget,
        double: bool,
    },
}

pub struct Session {
    document: SongDocument,
    libraries: Libraries,
    timeline: Timeline,
    preview: Option<Preview>,
    pub mode: Mode,
    pub zoom: Zoom,
    pub pane: Pane,
    pub tab: LibraryTab,
    pub selection: Selection,
    /// How much wider (positive) or narrower than the screen's own choice
    /// the staff draws its bars, in steps of ×1.25.
    pub staff_bar_width_steps: i8,
    /// The row under the cursor in each library: progressions, rhythms.
    cursors: [usize; 2],
    message: Option<Message>,
    messages_said_count: u64,
    /// The section and track whose line form change was asked about, so
    /// the next press converts it; any other command forgets it.
    form_change_asked: Option<(String, String)>,
    /// F1's selected row and cell, and its open dialog.
    arrange: ArrangeEditor,
}

/// Bars from about half (1.25^-3 ≈ 0.51) to 2.4 times (1.25^4 ≈ 2.44)
/// their usual width. Narrower than half, eighth notes crowd together.
const STAFF_BAR_NARROWEST_STEPS: i8 = -3;
const STAFF_BAR_WIDEST_STEPS: i8 = 4;

const fn tab_index(tab: LibraryTab) -> usize {
    match tab {
        LibraryTab::Progressions => 0,
        LibraryTab::Rhythms => 1,
    }
}

impl Session {
    #[must_use]
    pub fn new(document: SongDocument, libraries: Libraries) -> Session {
        let timeline: Timeline = Timeline::build(document.song(), document.source());
        // Start on the first entry that is not the count-in: that is where
        // the music is.
        let entry: usize = timeline
            .entries
            .iter()
            .position(|entry: &EntryView| !entry.count_in && !entry.muted)
            .unwrap_or(0);
        let mut session: Session = Session {
            document,
            libraries,
            timeline,
            preview: None,
            mode: Mode::SongHarmony,
            zoom: Mode::SongHarmony.view(),
            pane: Pane::Main,
            tab: LibraryTab::Progressions,
            selection: Selection {
                entry,
                first_bar: 0,
                last_bar: 0,
                track: 0,
                element: 0,
                pass_number: 1,
            },
            cursors: [0, 0],
            staff_bar_width_steps: 0,
            message: None,
            messages_said_count: 0,
            form_change_asked: None,
            arrange: ArrangeEditor::default(),
        };
        session.cursors[0] = session.first_item_row(LibraryTab::Progressions);
        session.cursors[1] = session.first_item_row(LibraryTab::Rhythms);
        session
    }

    #[must_use]
    pub fn document(&self) -> &SongDocument {
        &self.document
    }

    #[must_use]
    pub fn libraries(&self) -> &Libraries {
        &self.libraries
    }

    /// What the screen shows: the preview when there is one.
    #[must_use]
    pub fn timeline(&self) -> &Timeline {
        self.preview
            .as_ref()
            .map_or(&self.timeline, |preview: &Preview| &preview.timeline)
    }

    /// What Play plays: the preview when there is one, so a library row
    /// can be heard before it is kept.
    #[must_use]
    pub fn playing_song(&self) -> &Song {
        self.preview
            .as_ref()
            .map_or(self.document.song(), |preview: &Preview| &preview.song)
    }

    #[must_use]
    pub fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

    #[must_use]
    pub fn message(&self) -> Option<&Message> {
        self.message.as_ref()
    }

    pub fn say(&mut self, kind: MessageKind, text: String) {
        self.messages_said_count += 1;
        self.message = Some(Message {
            kind,
            text,
            number: self.messages_said_count,
        });
    }

    /// The meter of the selected bars, which the rhythm browser matches.
    #[must_use]
    pub fn selected_meter(&self) -> Option<TimeSignature> {
        let entry: &EntryView = self.timeline.entries.get(self.selection.entry)?;
        entry
            .bars
            .get(self.selection.first_bar)
            .map(|bar| bar.meter)
            .or(entry.meter)
    }

    #[must_use]
    pub fn library_rows(&self) -> Vec<LibraryRow> {
        self.libraries.rows(self.tab, self.selected_meter())
    }

    #[must_use]
    pub fn library_cursor(&self) -> usize {
        self.cursors[tab_index(self.tab)]
    }

    fn first_item_row(&self, tab: LibraryTab) -> usize {
        self.libraries
            .rows(tab, self.selected_meter())
            .iter()
            .position(|row: &LibraryRow| !matches!(row, LibraryRow::Heading(_)))
            .unwrap_or(0)
    }

    /// The text after an outside change to the file.
    pub fn reload(&mut self, text: String) {
        match self.document.reload(text) {
            Ok(()) => {
                self.after_document_change();
                self.say(MessageKind::Done, String::from("Reloaded the song file."));
            }
            Err(refused) => self.say(
                MessageKind::Problem,
                format!(
                    "The file changed but has mistakes; still showing the last good version. {}",
                    refused.messages.join(" ")
                ),
            ),
        }
    }

    /// After the text has been saved.
    pub fn mark_saved(&mut self) {
        self.document.mark_saved();
    }

    /// Carries out one command.
    pub fn handle(&mut self, command: Command) {
        if command == Command::ConvertLineForm {
            self.convert_line_form();
            return;
        }
        self.form_change_asked = None;
        match command {
            Command::SetMode(mode) => self.set_mode(mode),
            Command::StaffBarWidth(steps) => self.change_staff_bar_width(steps),
            Command::SwitchPane => {
                self.pane = match self.pane {
                    Pane::Main => Pane::Library,
                    Pane::Library => Pane::Main,
                };
                if self.pane == Pane::Library {
                    self.refresh_preview();
                } else {
                    self.drop_preview();
                }
            }
            Command::SetTab(tab) => {
                self.tab = tab;
                self.refresh_preview_if_browsing();
            }
            Command::NextTab => {
                self.tab = match self.tab {
                    LibraryTab::Progressions => LibraryTab::Rhythms,
                    LibraryTab::Rhythms => LibraryTab::Progressions,
                };
                self.refresh_preview_if_browsing();
            }
            Command::Left | Command::Right | Command::Up | Command::Down => self.arrow(command),
            Command::ExtendLeft => self.extend(-1),
            Command::ExtendRight => self.extend(1),
            Command::Commit => self.commit(),
            Command::Cancel => {
                if self.preview.is_some() {
                    self.drop_preview();
                    self.say(MessageKind::Done, String::from("Preview dropped."));
                } else {
                    self.selection.last_bar = self.selection.first_bar;
                    self.pane = Pane::Main;
                }
            }
            Command::Undo => {
                self.drop_preview();
                match self.document.undo() {
                    Some(what) => {
                        self.after_document_change();
                        self.say(MessageKind::Done, format!("Undone: {what}."));
                    }
                    None => self.say(MessageKind::Problem, String::from("Nothing to undo.")),
                }
            }
            Command::Redo => {
                self.drop_preview();
                match self.document.redo() {
                    Some(what) => {
                        self.after_document_change();
                        self.say(MessageKind::Done, format!("Redone: {what}."));
                    }
                    None => self.say(MessageKind::Problem, String::from("Nothing to redo.")),
                }
            }
            Command::SelectEntry(entry) => self.select(entry, 0, None, None),
            Command::SelectBar { entry, bar } => self.select(entry, bar, None, None),
            Command::SelectTrack(track) => {
                self.selection.track = track.min(self.timeline.tracks.len().saturating_sub(1));
                self.selection.element = self.first_element_in(self.selection.first_bar);
                self.refresh_preview_if_browsing();
            }
            Command::SelectElement {
                entry,
                bar,
                track,
                element,
                pass_number,
            } => self.select_element(entry, bar, track, element, pass_number),
            Command::SelectLibraryRow(row) => {
                let rows: Vec<LibraryRow> = self.library_rows();
                if rows
                    .get(row)
                    .is_some_and(|row: &LibraryRow| !matches!(row, LibraryRow::Heading(_)))
                {
                    self.cursors[tab_index(self.tab)] = row;
                    self.pane = Pane::Library;
                    self.refresh_preview();
                }
            }
            Command::Arrange(_) | Command::ArrangeClick { .. } => self.arrange_command(command),
            musical => self.musical(musical),
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        self.drop_preview();
        self.arrange.close_dialog();
        if mode == Mode::ArrangeSongSections {
            self.arrange_select_entry_row();
        }
        self.mode = mode;
        self.zoom = mode.view();
        self.tab = mode.tab();
        self.say(
            MessageKind::Done,
            format!("{} {}.", mode.key_name(), mode.title()),
        );
    }

    /// A key or click in F1, handed to its editor; what it asks is written.
    fn arrange_command(&mut self, command: Command) {
        let rows: Vec<ArrangeRow> = self.arrange_rows();
        let context: ArrangeContext =
            arrange_context(&rows, &self.timeline, self.document.source());
        let request: Option<ArrangeRequest> = match command {
            Command::Arrange(key) => self.arrange.key(key, &context),
            Command::ArrangeClick { target, double } => {
                self.arrange.click(target, double, &context)
            }
            _ => None,
        };
        self.carry_out_arrange(request);
    }

    // ---- F1 Arrange Song Sections ---------------------------------------

    /// The composition's rows, as F1 shows them.
    #[must_use]
    pub fn arrange_rows(&self) -> Vec<ArrangeRow> {
        arrangement_rows(self.document.source(), self.document.song(), &self.timeline)
    }

    /// F1's selected row and cell, and its open dialog.
    #[must_use]
    pub fn arrange(&self) -> &ArrangeEditor {
        &self.arrange
    }

    /// Whether F1 has a dialog open: then typed text goes to it.
    #[must_use]
    pub fn arrange_has_dialog(&self) -> bool {
        self.mode == Mode::ArrangeSongSections && self.arrange.has_dialog()
    }

    /// Text typed while an F1 dialog is open.
    pub fn type_text(&mut self, typed: &str) {
        if self.arrange_has_dialog() {
            self.arrange.type_text(typed);
        }
    }

    /// Puts F1's selection on the row of the selected entry.
    fn arrange_select_entry_row(&mut self) {
        let rows: Vec<ArrangeRow> = self.arrange_rows();
        if let Some(row) = rows.iter().position(|row: &ArrangeRow| {
            matches!(row, ArrangeRow::Entry(entry) if entry.entry_index == self.selection.entry)
        }) {
            self.arrange.row = row;
        }
        self.arrange.clamp(&rows);
    }

    /// The selected entry follows F1's row, so Play and the form strip use
    /// it.
    fn follow_arrange_row(&mut self) {
        let rows: Vec<ArrangeRow> = self.arrange_rows();
        self.arrange.clamp(&rows);
        if let Some(entry) = self.arrange.selected_entry(&rows) {
            self.selection.entry = entry;
            self.clamp_selection();
        }
    }

    fn carry_out_arrange(&mut self, request: Option<ArrangeRequest>) {
        match request {
            None => {}
            Some(ArrangeRequest::Say(kind, text)) => self.say(kind, text),
            Some(ArrangeRequest::Edit { edit, then_row }) => {
                let written: Result<(String, String), String> = write_arrange_edit(
                    &edit,
                    self.document.text(),
                    self.document.source(),
                    self.document.song(),
                );
                match written {
                    Ok((text, what)) => {
                        let before: String = self.document.text().to_string();
                        self.take(text, &what);
                        if self.document.text() != before {
                            // Kept: a dialog left open for a correction
                            // (the new-section form) has done its work.
                            self.arrange.close_dialog();
                            if let Some(row) = then_row {
                                self.arrange.row = row;
                            }
                        }
                    }
                    Err(why) => self.say(MessageKind::Problem, format!("{why}.")),
                }
            }
        }
        self.follow_arrange_row();
    }

    fn musical(&mut self, command: Command) {
        self.drop_preview();
        let result: Result<(String, String), String> = match command {
            Command::Move(step) => self.move_selected(step),
            Command::ChordTone(steps) => self.move_by_chord_tones(steps),
            Command::KeyBySemitones(semitones) => self.change_key(semitones, false),
            Command::KeyByFifths(fifths) => self.change_key(fifths, true),
            Command::Tempo(beats_per_minute) => self.change_tempo(beats_per_minute),
            Command::Repeat(delta) => self.change_repeat(delta),
            Command::ToggleMute => self.toggle_mute(),
            Command::MoveEntry(delta) => self.move_selected_entry(delta),
            Command::DuplicateEntry => duplicate_entry(
                self.document.text(),
                self.document.source(),
                self.selection.entry,
            )
            .map(|text| (text, String::from("Entry written again after itself"))),
            Command::RemoveEntry => remove_entry(
                self.document.text(),
                self.document.source(),
                self.selection.entry,
            )
            .map(|text| {
                (
                    text,
                    String::from("Entry removed from the composition (its section is kept)"),
                )
            }),
            Command::CycleAccent => self.accent_selected(),
            Command::ToggleRest => self.toggle_selected_rest(),
            Command::BassFromRoots => self.with_target(|target| {
                bass_from_roots(target).map(|text| {
                    (
                        text,
                        String::from("Bass line set to the roots of the lead's chords"),
                    )
                })
            }),
            Command::NewSectionFromProgression => self.new_section(),
            _ => return,
        };
        match result {
            Ok((text, what)) => self.take(text, &what),
            Err(why) => self.say(MessageKind::Problem, format!("{why}.")),
        }
        if command == Command::MoveEntry(1) || command == Command::MoveEntry(-1) {
            let delta: isize = if command == Command::MoveEntry(1) {
                1
            } else {
                -1
            };
            if let Some(entry) = self.selection.entry.checked_add_signed(delta)
                && entry < self.timeline.entries.len()
                && self
                    .message
                    .as_ref()
                    .is_some_and(|message| message.kind == MessageKind::Done)
            {
                self.selection.entry = entry;
            }
        }
    }

    /// Edits the song's text with `edit` (given the text, where things are
    /// in it, and the song as read) and takes the result, saying `what` was
    /// done, or says `failed` and why not. True when the text changed.
    pub fn edit_text(
        &mut self,
        what: &str,
        failed: &str,
        edit: impl FnOnce(&str, &SongSource, &Song) -> Result<String, String>,
    ) -> bool {
        let before: String = self.document.text().to_string();
        match edit(
            self.document.text(),
            self.document.source(),
            self.document.song(),
        ) {
            Ok(text) => match self.document.replace(text, what) {
                Ok(()) => {
                    self.after_document_change();
                    self.say(MessageKind::Done, format!("{what}."));
                }
                Err(refused) => {
                    // The reader's reasons name text lines, which the
                    // dialog that asked does not show: the dialog says only
                    // that the song would not accept it, and the reasons go
                    // to the terminal.
                    eprintln!("{what}: refused. {}", refused.messages.join("; "));
                    self.say(
                        MessageKind::Problem,
                        format!("{failed}. The song would not accept it."),
                    );
                }
            },
            Err(problem) => self.say(MessageKind::Problem, format!("{failed}. {problem}")),
        }
        self.document.text() != before
    }

    /// Takes a new text into the song, or says why it was refused.
    fn take(&mut self, text: String, what: &str) {
        match self.document.replace(text, what) {
            Ok(()) => {
                self.after_document_change();
                self.say(MessageKind::Done, format!("{what}."));
            }
            Err(refused) => self.say(
                MessageKind::Problem,
                format!("{what}: not done. {}", refused.messages.join(" ")),
            ),
        }
    }

    fn after_document_change(&mut self) {
        self.timeline = Timeline::build(self.document.song(), self.document.source());
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let entries: usize = self.timeline.entries.len();
        self.selection.entry = self.selection.entry.min(entries.saturating_sub(1));
        let bars: usize = self
            .timeline
            .entries
            .get(self.selection.entry)
            .map_or(1, EntryView::bar_count)
            .max(1);
        self.selection.first_bar = self.selection.first_bar.min(bars - 1);
        self.selection.last_bar = self
            .selection
            .last_bar
            .clamp(self.selection.first_bar, bars - 1);
        self.selection.track = self
            .selection
            .track
            .min(self.timeline.tracks.len().saturating_sub(1));
    }

    fn select_element(
        &mut self,
        entry: usize,
        bar: usize,
        track: usize,
        element: usize,
        pass_number: u32,
    ) {
        self.select(entry, bar, Some(track), Some(element));
        self.selection.pass_number = pass_number;
    }

    fn select(&mut self, entry: usize, bar: usize, track: Option<usize>, element: Option<usize>) {
        self.selection.entry = entry;
        self.selection.first_bar = bar;
        self.selection.last_bar = bar;
        self.selection.pass_number = 1;
        if let Some(track) = track {
            self.selection.track = track;
        }
        self.clamp_selection();
        self.selection.element =
            element.unwrap_or_else(|| self.first_element_in(self.selection.first_bar));
        self.refresh_preview_if_browsing();
    }

    fn drop_preview(&mut self) {
        self.preview = None;
    }

    fn refresh_preview_if_browsing(&mut self) {
        if self.pane == Pane::Library {
            self.refresh_preview();
        } else {
            self.drop_preview();
        }
    }

    // ---- Navigation -------------------------------------------------

    fn arrow(&mut self, command: Command) {
        if self.pane == Pane::Library {
            match command {
                Command::Up => self.move_cursor(-1),
                Command::Down => self.move_cursor(1),
                Command::Left | Command::Right => self.handle(Command::NextTab),
                _ => {}
            }
            return;
        }
        self.drop_preview();
        match (self.zoom, command) {
            (Zoom::Form, Command::Left) | (Zoom::Form | Zoom::Harmony, Command::Up) => {
                self.step_entry(-1);
            }
            (Zoom::Form, Command::Right) | (Zoom::Form | Zoom::Harmony, Command::Down) => {
                self.step_entry(1);
            }
            // Note by note only where the notes are edited; the harmony
            // of a track is edited bar by bar.
            (Zoom::Staff, Command::Left) if self.mode == Mode::SectionEditor => {
                self.step_element(-1);
            }
            (Zoom::Staff, Command::Right) if self.mode == Mode::SectionEditor => {
                self.step_element(1);
            }
            (Zoom::Harmony | Zoom::Tracks | Zoom::Staff, Command::Left) => self.step_bar(-1),
            (Zoom::Harmony | Zoom::Tracks | Zoom::Staff, Command::Right) => self.step_bar(1),
            (Zoom::Tracks | Zoom::Staff, Command::Up) => self.step_track(-1),
            (Zoom::Tracks | Zoom::Staff, Command::Down) => self.step_track(1),
            _ => {}
        }
    }

    fn move_cursor(&mut self, delta: isize) {
        let rows: Vec<LibraryRow> = self.library_rows();
        let mut row: usize = self.library_cursor();
        loop {
            let Some(next) = row
                .checked_add_signed(delta)
                .filter(|next: &usize| *next < rows.len())
            else {
                return;
            };
            row = next;
            if !matches!(rows[row], LibraryRow::Heading(_)) {
                break;
            }
        }
        self.cursors[tab_index(self.tab)] = row;
        self.refresh_preview();
    }

    fn step_entry(&mut self, delta: isize) {
        if let Some(entry) = self.selection.entry.checked_add_signed(delta)
            && entry < self.timeline.entries.len()
        {
            self.select(entry, 0, None, None);
        }
    }

    fn step_bar(&mut self, delta: isize) {
        let bars: usize = self
            .timeline
            .entries
            .get(self.selection.entry)
            .map_or(0, EntryView::bar_count);
        let pass_number: u32 = self.selection.pass_number;
        let repeat_count: u32 = self
            .timeline
            .entries
            .get(self.selection.entry)
            .map_or(1, |entry: &EntryView| entry.repeat_count.max(1));
        match self.selection.first_bar.checked_add_signed(delta) {
            Some(bar) if bar < bars => {
                self.select(self.selection.entry, bar, None, None);
                self.selection.pass_number = pass_number;
            }
            // The staff writes repeats out, so there the selection goes
            // through each time a repeated entry plays before leaving it.
            _ if self.zoom == Zoom::Staff && delta > 0 && pass_number < repeat_count => {
                self.select(self.selection.entry, 0, None, None);
                self.selection.pass_number = pass_number + 1;
            }
            _ if self.zoom == Zoom::Staff && delta < 0 && pass_number > 1 => {
                self.select(self.selection.entry, bars.saturating_sub(1), None, None);
                self.selection.pass_number = pass_number - 1;
            }
            _ => {
                // Off the end of the section: on to the next entry's first
                // bar, or back to the previous entry's last.
                let Some(entry) = self
                    .selection
                    .entry
                    .checked_add_signed(delta)
                    .filter(|entry| *entry < self.timeline.entries.len())
                else {
                    return;
                };
                let bar: usize = if delta < 0 {
                    self.timeline.entries[entry].bar_count().saturating_sub(1)
                } else {
                    0
                };
                self.select(entry, bar, None, None);
                // Back into a repeated entry on the staff: its last time.
                if self.zoom == Zoom::Staff && delta < 0 {
                    self.selection.pass_number = self.timeline.entries[entry].repeat_count.max(1);
                }
            }
        }
    }

    fn extend(&mut self, delta: isize) {
        let bars: usize = self
            .timeline
            .entries
            .get(self.selection.entry)
            .map_or(0, EntryView::bar_count);
        if let Some(last) = self.selection.last_bar.checked_add_signed(delta)
            && last < bars
            && last >= self.selection.first_bar
        {
            self.selection.last_bar = last;
            self.refresh_preview_if_browsing();
        }
    }

    fn step_track(&mut self, delta: isize) {
        if let Some(track) = self.selection.track.checked_add_signed(delta)
            && track < self.timeline.tracks.len()
        {
            self.handle(Command::SelectTrack(track));
        }
    }

    /// The selected track's elements in the selected entry, as (bar,
    /// element index in the line).
    fn track_elements(&self) -> Vec<(usize, usize)> {
        let Some(entry) = self.timeline.entries.get(self.selection.entry) else {
            return Vec::new();
        };
        entry
            .bars
            .iter()
            .enumerate()
            .flat_map(|(bar_index, bar)| {
                bar.tracks
                    .get(self.selection.track)
                    .into_iter()
                    .flat_map(move |track_bar| {
                        track_bar
                            .elements
                            .iter()
                            .map(move |element| (bar_index, element.line_element_index))
                    })
            })
            .collect()
    }

    fn first_element_in(&self, bar: usize) -> usize {
        self.track_elements()
            .iter()
            .find(|(bar_index, _)| *bar_index == bar)
            .map_or(0, |(_, element)| *element)
    }

    fn step_element(&mut self, delta: isize) {
        let elements: Vec<(usize, usize)> = self.track_elements();
        let position: Option<usize> = elements
            .iter()
            .position(|(_, element)| *element == self.selection.element);
        let next: Option<usize> = position.and_then(|position| position.checked_add_signed(delta));
        if let Some((bar, element)) = next.and_then(|next| elements.get(next)) {
            self.selection.first_bar = *bar;
            self.selection.last_bar = *bar;
            self.selection.element = *element;
        } else {
            self.step_bar(delta);
            if delta < 0 {
                self.selection.element = self
                    .track_elements()
                    .last()
                    .map_or(0, |(_, element)| *element);
            }
        }
    }

    // ---- Preview and commit -------------------------------------------

    fn refresh_preview(&mut self) {
        let rows: Vec<LibraryRow> = self.library_rows();
        let Some(row) = rows.get(self.library_cursor()).cloned() else {
            self.drop_preview();
            return;
        };
        let result: Result<(String, String), String> = match row {
            LibraryRow::Heading(_) => return,
            LibraryRow::Progression(index) => {
                let Some(progression) = self.libraries.progression(index).cloned() else {
                    return;
                };
                let standard = self.libraries.numeral_standard();
                self.with_target(|target| {
                    apply_progression(target, &progression, standard)
                        .map(|text| (text, format!("Progression \"{}\"", progression.name())))
                })
            }
            LibraryRow::Rhythm { index, fits } => {
                let Some(pattern) = self.libraries.rhythm(index).cloned() else {
                    return;
                };
                if fits {
                    let rhythm: RhythmBars = RhythmBars::of_pattern(&pattern);
                    self.with_target(|target| {
                        apply_rhythm(target, &rhythm)
                            .map(|text| (text, format!("Rhythm \"{}\"", pattern.name())))
                    })
                } else {
                    Err(meter_mismatch(&pattern, self.selected_meter()))
                }
            }
        };
        match result {
            Ok((text, label)) => match read_song(self.document.file_name(), &text) {
                Ok(song) => {
                    let timeline: Timeline = Timeline::build(&song, &SongSource::read(&text));
                    self.say(
                        MessageKind::Done,
                        format!("Preview: {label} on {}. Enter applies it, Esc drops it, Space plays it.", self.where_text()),
                    );
                    self.preview = Some(Preview {
                        label,
                        text,
                        song,
                        timeline,
                    });
                }
                Err(messages) => {
                    self.preview = None;
                    self.say(
                        MessageKind::Problem,
                        format!("{label} cannot go here: {}", messages.join(" ")),
                    );
                }
            },
            Err(why) => {
                self.preview = None;
                self.say(MessageKind::Problem, format!("{why}."));
            }
        }
    }

    fn commit(&mut self) {
        let Some(preview) = self.preview.take() else {
            if self.pane == Pane::Library {
                self.refresh_preview();
                if self.preview.is_some() {
                    self.commit();
                }
            }
            return;
        };
        let what: String = format!("Applied: {} on {}", preview.label, self.where_text());
        self.take(preview.text, &what);
        self.pane = Pane::Main;
    }

    /// What an edit changes now, for the header: always in view, so there
    /// is no doubt where a key press or an applied library row goes.
    #[must_use]
    pub fn editing_text(&self) -> String {
        let entry: Option<&EntryView> = self.timeline.entries.get(self.selection.entry);
        let section: &str = entry.map_or("?", |entry: &EntryView| entry.section_name.as_str());
        let bars: String = if self.selection.bar_count() == 1 {
            format!("bar {}", self.selection.first_bar + 1)
        } else {
            format!(
                "bars {}–{}",
                self.selection.first_bar + 1,
                self.selection.last_bar + 1
            )
        };
        let track: &str = self.target_track_name().unwrap_or("?");
        let sampled: bool = self
            .selected_section()
            .is_some_and(|section: &Section| is_sampled(section, track));
        match self.mode {
            Mode::ArrangeSongSections => format!(
                "Editing: the composition, entry {} \"{section}\"",
                self.selection.entry + 1
            ),
            Mode::ComposeSections => format!("Editing: section \"{section}\""),
            Mode::SectionEditor => {
                let form: &str = if sampled { "sampled" } else { "coupled" };
                format!(
                    "Editing: {track} line ({form}), \"{section}\" {bars}, note {}",
                    self.selection.element + 1
                )
            }
            Mode::SongHarmony => {
                format!("Editing: the song's harmony ({track}, lead), \"{section}\" {bars}")
            }
            Mode::TrackHarmony => {
                let what: &str = if sampled {
                    "its sample source"
                } else {
                    "its chords or notes"
                };
                format!("Editing: {track} harmony ({what}), \"{section}\" {bars}")
            }
        }
    }

    /// "Verse, bars 1–2, Keys" for messages.
    fn where_text(&self) -> String {
        let section: &str = self
            .timeline
            .entries
            .get(self.selection.entry)
            .map_or("?", |entry: &EntryView| entry.section_name.as_str());
        let bars: String = if self.selection.bar_count() == 1 {
            format!("bar {}", self.selection.first_bar + 1)
        } else {
            format!(
                "bars {}–{}",
                self.selection.first_bar + 1,
                self.selection.last_bar + 1
            )
        };
        let track: &str = self.target_track_name().unwrap_or("?");
        format!("\"{section}\" {bars}, {track}")
    }

    // ---- Targets ---------------------------------------------------------

    /// The section as the selected entry plays it.
    fn selected_section(&self) -> Option<&Section> {
        let entry: &EntryView = self.timeline.entries.get(self.selection.entry)?;
        let playing_before: usize = self.timeline.entries[..self.selection.entry]
            .iter()
            .filter(|entry: &&EntryView| !entry.muted)
            .count();
        let sections: &[Section] = self.document.song().sections();
        if entry.muted {
            sections
                .iter()
                .find(|section: &&Section| section.name() == entry.section_name)
        } else {
            sections.get(playing_before)
        }
    }

    /// The track an edit writes: the lead in Song Harmony, else the
    /// selected track.
    fn target_track(&self) -> Option<&Track> {
        let voice_tracks: Vec<&Track> = self
            .document
            .song()
            .tracks()
            .iter()
            .filter(|track: &&Track| matches!(track.kind(), TrackKind::Voice { .. }))
            .collect();
        if self.mode == Mode::SongHarmony {
            return voice_tracks
                .iter()
                .find(|track| matches!(track.kind(), TrackKind::Voice { lead: true, .. }))
                .copied();
        }
        voice_tracks.get(self.selection.track).copied()
    }

    fn target_track_name(&self) -> Option<&str> {
        self.target_track().map(Track::name)
    }

    fn with_target(
        &self,
        operation: impl FnOnce(&Target<'_>) -> Result<(String, String), String>,
    ) -> Result<(String, String), String> {
        let section: &Section = self
            .selected_section()
            .ok_or_else(|| String::from("no section is selected"))?;
        let track: &Track = self
            .target_track()
            .ok_or_else(|| String::from("the song has no voice track to write"))?;
        let target: Target = Target {
            song: self.document.song(),
            text: self.document.text(),
            source: self.document.source(),
            section,
            first_bar: self.selection.first_bar,
            bar_count: self.selection.bar_count(),
            track,
        };
        operation(&target)
    }

    // ---- Musical edits -----------------------------------------------------

    fn move_selected(&self, step: Step) -> Result<(String, String), String> {
        match self.mode {
            // The entry's key (Arrange) or the section's, notes moving
            // with it (Compose); `change_key` tells them apart by the view.
            Mode::ArrangeSongSections | Mode::ComposeSections => match step {
                Step::Chromatic(semitones) => self.change_key(semitones, false),
                Step::Diatonic(steps) => self.change_key(steps, true),
                Step::Octave(_) => Err(String::from(
                    "a key has no octave; use Shift (semitones) or Ctrl (fifths)",
                )),
            },
            Mode::SongHarmony => self.with_lead_target(|target| {
                move_contents(target, step, None).map(|text| {
                    (
                        text,
                        format!("Chords of {} {}", self.where_text(), step.describe()),
                    )
                })
            }),
            Mode::TrackHarmony => self.with_target(|target| {
                move_contents(target, step, None).map(|text| {
                    (
                        text,
                        format!("Harmony of {} {}", self.where_text(), step.describe()),
                    )
                })
            }),
            Mode::SectionEditor => {
                let element: usize = self.selection.element;
                self.with_target(|target| {
                    move_contents(target, step, Some(element))
                        .map(|text| (text, format!("Note {} {}", element + 1, step.describe())))
                })
            }
        }
    }

    fn move_by_chord_tones(&self, steps: i16) -> Result<(String, String), String> {
        let direction: &str = if steps >= 0 { "up" } else { "down" };
        let tone: &str = if steps.unsigned_abs() == 1 {
            "chord tone"
        } else {
            "chord tones"
        };
        let words: String = format!("{direction} {} {tone}", steps.unsigned_abs());
        if self.mode != Mode::SectionEditor {
            return Err(String::from(
                "chord tones move notes: use F3 Section Editor",
            ));
        }
        let element: usize = self.selection.element;
        self.with_target(|target| {
            move_to_chord_tones(target, steps, Some(element))
                .map(|text| (text, format!("Note {} {words}", element + 1)))
        })
    }

    fn with_lead_target(
        &self,
        operation: impl FnOnce(&Target<'_>) -> Result<(String, String), String>,
    ) -> Result<(String, String), String> {
        let section: &Section = self
            .selected_section()
            .ok_or_else(|| String::from("no section is selected"))?;
        let track: &Track = self
            .document
            .song()
            .tracks()
            .iter()
            .find(|track: &&Track| matches!(track.kind(), TrackKind::Voice { lead: true, .. }))
            .ok_or_else(|| String::from("the song has no lead track"))?;
        let target: Target = Target {
            song: self.document.song(),
            text: self.document.text(),
            source: self.document.source(),
            section,
            first_bar: self.selection.first_bar,
            bar_count: self.selection.bar_count(),
            track,
        };
        operation(&target)
    }

    /// The form view moves the entry's own key; every other view moves the
    /// section, its written notes with it.
    fn change_key(&self, amount: i16, by_fifths: bool) -> Result<(String, String), String> {
        let entry: &EntryView = self
            .timeline
            .entries
            .get(self.selection.entry)
            .ok_or_else(|| String::from("no entry is selected"))?;
        let key: Key = entry
            .key
            .ok_or_else(|| format!("\"{}\" has no key to move", entry.section_name))?;
        let new_key: Key = if by_fifths {
            key_by_fifths(key, amount)
        } else {
            transposed_key(key, amount)
        };
        let source: &SongSource = self.document.source();
        let text: &str = self.document.text();
        if self.zoom == Zoom::Form {
            let new_text: String = set_entry_key(
                text,
                source,
                self.selection.entry,
                Some(new_key.to_string()),
            )?;
            return Ok((
                new_text,
                format!(
                    "Entry {} \"{}\" now in {new_key} (its written notes stay; zoom in to move the section with its notes)",
                    self.selection.entry + 1,
                    entry.section_name
                ),
            ));
        }
        if entry.key_written_on_entry {
            return Err(format!(
                "this entry of \"{}\" writes its own key; change it in the form view (F5), or take the entry's key off",
                entry.section_name
            ));
        }
        let section: &Section = self
            .selected_section()
            .ok_or_else(|| String::from("no section is selected"))?;
        let semitones: i16 = semitones_between(key, new_key);
        let moved_notes: String =
            transpose_section_notes(self.document.song(), text, section, new_key, semitones)?;
        let new_source: SongSource = SongSource::read(&moved_notes);
        let new_text: String = set_section_key(
            &moved_notes,
            &new_source,
            section.name(),
            &new_key.to_string(),
        )?;
        Ok((
            new_text,
            format!(
                "Section \"{}\" now in {new_key}, written notes moved with it",
                section.name()
            ),
        ))
    }

    fn change_tempo(&self, delta: i16) -> Result<(String, String), String> {
        let entry: &EntryView = self
            .timeline
            .entries
            .get(self.selection.entry)
            .ok_or_else(|| String::from("no entry is selected"))?;
        // milli-beats/min ÷ 1000 → beats/min, then the change.
        let beats_per_minute: i64 =
            i64::try_from(entry.milli_beats_per_minute / 1000).unwrap_or(0) + i64::from(delta);
        if beats_per_minute < 1 {
            return Err(String::from("the tempo cannot go below 1 beat/min"));
        }
        let number: String = beats_per_minute.to_string();
        let source: &SongSource = self.document.source();
        let text: &str = self.document.text();
        if self.zoom == Zoom::Form {
            let new_text: String = set_entry_tempo(
                text,
                source,
                self.selection.entry,
                Some(format!("{number} beats/min")),
            )?;
            return Ok((
                new_text,
                format!(
                    "Entry {} \"{}\" at {number} beats/min",
                    self.selection.entry + 1,
                    entry.section_name
                ),
            ));
        }
        let new_text: String = set_section_tempo(text, source, &entry.section_name, &number)?;
        Ok((
            new_text,
            format!("Section \"{}\" at {number} beats/min", entry.section_name),
        ))
    }

    fn change_repeat(&self, delta: i16) -> Result<(String, String), String> {
        let entry: &EntryView = self
            .timeline
            .entries
            .get(self.selection.entry)
            .ok_or_else(|| String::from("no entry is selected"))?;
        let repeat: i64 = i64::from(entry.repeat_count) + i64::from(delta);
        let repeat: u32 = u32::try_from(repeat.max(0)).unwrap_or(0);
        let text: String = set_entry_repeat(
            self.document.text(),
            self.document.source(),
            self.selection.entry,
            repeat,
        )?;
        let what: String = if repeat == 0 {
            format!(
                "Entry {} \"{}\" muted (repeat 0)",
                self.selection.entry + 1,
                entry.section_name
            )
        } else {
            format!(
                "Entry {} \"{}\" plays {repeat} times",
                self.selection.entry + 1,
                entry.section_name
            )
        };
        Ok((text, what))
    }

    fn toggle_mute(&self) -> Result<(String, String), String> {
        let entry: &EntryView = self
            .timeline
            .entries
            .get(self.selection.entry)
            .ok_or_else(|| String::from("no entry is selected"))?;
        let repeat: u32 = u32::from(entry.muted);
        let text: String = set_entry_repeat(
            self.document.text(),
            self.document.source(),
            self.selection.entry,
            repeat,
        )?;
        let what: String = if entry.muted { "restored" } else { "muted" }.to_string();
        Ok((
            text,
            format!(
                "Entry {} \"{}\" {what}",
                self.selection.entry + 1,
                entry.section_name
            ),
        ))
    }

    fn move_selected_entry(&self, delta: i16) -> Result<(String, String), String> {
        let text: String = move_entry(
            self.document.text(),
            self.document.source(),
            self.selection.entry,
            isize::from(delta),
        )?;
        let direction: &str = if delta < 0 { "earlier" } else { "later" };
        Ok((text, format!("Entry moved {direction}")))
    }

    fn accent_selected(&self) -> Result<(String, String), String> {
        if self.mode != Mode::SectionEditor {
            return Err(String::from(
                "accents are set note by note: use F3 Section Editor",
            ));
        }
        let element: usize = self.selection.element;
        self.with_target(|target| {
            cycle_accent(target, element)
                .map(|text| (text, format!("Accent on note {}", element + 1)))
        })
    }

    fn change_staff_bar_width(&mut self, steps: i8) {
        self.staff_bar_width_steps = self
            .staff_bar_width_steps
            .saturating_add(steps)
            .clamp(STAFF_BAR_NARROWEST_STEPS, STAFF_BAR_WIDEST_STEPS);
    }

    /// Asks first, then converts: a conversion rewrites the track's whole
    /// line in the section, so one key press alone never does it.
    fn convert_line_form(&mut self) {
        let Some((section_name, track_name, sampled)) =
            self.selected_section().and_then(|section: &Section| {
                self.target_track().map(|track: &Track| {
                    (
                        section.name().to_string(),
                        track.name().to_string(),
                        is_sampled(section, track.name()),
                    )
                })
            })
        else {
            self.say(
                MessageKind::Problem,
                String::from("Select a track's bar first."),
            );
            return;
        };
        let asked: (String, String) = (section_name.clone(), track_name.clone());
        if self.form_change_asked.as_ref() != Some(&asked) {
            let question: String = if sampled {
                format!(
                    "\"{track_name}\" in \"{section_name}\" is sampled. Press C again to make it coupled: the derived notes are composed into its line and its sample source goes."
                )
            } else {
                format!(
                    "\"{track_name}\" in \"{section_name}\" is coupled. Press C again to make it sampled: a sample source holding its notes, and a rhythm line."
                )
            };
            self.form_change_asked = Some(asked);
            self.say(MessageKind::Done, question);
            return;
        }
        self.form_change_asked = None;
        let result: Result<(String, String), String> = self.with_target(|target| {
            if sampled {
                make_coupled(target).map(|text| {
                    (
                        text,
                        format!("\"{track_name}\" in \"{section_name}\" is now coupled"),
                    )
                })
            } else {
                make_sampled(target).map(|text| {
                    (
                        text,
                        format!("\"{track_name}\" in \"{section_name}\" is now sampled"),
                    )
                })
            }
        });
        match result {
            Ok((text, what)) => self.take(text, &what),
            Err(why) => self.say(MessageKind::Problem, format!("{why}.")),
        }
    }

    fn toggle_selected_rest(&self) -> Result<(String, String), String> {
        if self.mode != Mode::SectionEditor {
            return Err(String::from(
                "rests are changed note by note: use F3 Section Editor",
            ));
        }
        let element: usize = self.selection.element;
        self.with_target(|target| {
            toggle_rest(target, element).map(|(text, now_rest)| {
                let became: &str = if now_rest { "a rest" } else { "a note" };
                (text, format!("Element {} is now {became}", element + 1))
            })
        })
    }

    fn new_section(&self) -> Result<(String, String), String> {
        let rows: Vec<LibraryRow> = self.libraries.rows(LibraryTab::Progressions, None);
        let progression: &Progression = match rows.get(self.cursors[0]) {
            Some(LibraryRow::Progression(index)) => self
                .libraries
                .progression(*index)
                .ok_or_else(|| String::from("the progression is not in the library"))?,
            _ => {
                return Err(String::from(
                    "choose a progression in the library first (Tab, then the Progressions tab)",
                ));
            }
        };
        let entry: &EntryView = self
            .timeline
            .entries
            .get(self.selection.entry)
            .ok_or_else(|| String::from("no entry is selected"))?;
        let meter: TimeSignature = entry.meter.unwrap_or(progression.time_signature());
        let (text, name): (String, String) = new_section_from_progression(
            self.document.song(),
            self.document.text(),
            self.document.source(),
            self.selection.entry,
            progression,
            self.libraries.numeral_standard(),
            meter,
            entry.key,
        )?;
        Ok((
            text,
            format!(
                "New section \"{name}\" from \"{}\" after entry {}",
                progression.name(),
                self.selection.entry + 1
            ),
        ))
    }
}

/// What F1's editor reads of the song. A free function, so the editor (a
/// field of the session) can be borrowed apart from what it reads.
fn arrange_context<'a>(
    rows: &'a [ArrangeRow],
    timeline: &'a Timeline,
    source: &SongSource,
) -> ArrangeContext<'a> {
    ArrangeContext {
        rows,
        timeline,
        section_names: source
            .sections
            .iter()
            .map(|section| section.name.clone())
            .collect(),
    }
}

fn meter_mismatch(pattern: &RhythmPattern, meter: Option<TimeSignature>) -> String {
    let rhythm_meter: TimeSignature = pattern.time_signature();
    match meter {
        Some(meter) => format!(
            "\"{}\" is in {}/{} and these bars are in {}/{}; choose a rhythm in {}/{}",
            pattern.name(),
            rhythm_meter.pulses_per_bar(),
            rhythm_meter.pulses_per_whole_note(),
            meter.pulses_per_bar(),
            meter.pulses_per_whole_note(),
            meter.pulses_per_bar(),
            meter.pulses_per_whole_note()
        ),
        None => format!("\"{}\" does not fit these bars", pattern.name()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = include_str!("../../../songs_canonical/composer/composer_demo.song");
    const PROGRESSIONS: &str =
        include_str!("../../../songs_canonical/composer/composer_progressions.library");
    const RHYTHMS: &str =
        include_str!("../../../songs_canonical/composer/composer_rhythms.library");

    fn session() -> Session {
        let document: SongDocument =
            SongDocument::new("composer_demo.song", DEMO.to_string()).unwrap();
        let libraries: Libraries =
            Libraries::read("p.library", PROGRESSIONS, "r.library", RHYTHMS).unwrap();
        Session::new(document, libraries)
    }

    fn symbols(session: &Session, entry: usize) -> Vec<String> {
        session.timeline().entries[entry]
            .bars
            .iter()
            .map(|bar| {
                bar.chords
                    .iter()
                    .map(|chord| chord.symbol.clone())
                    .collect::<Vec<String>>()
                    .join(" ")
            })
            .collect()
    }

    fn expect_done(session: &Session) {
        let message: &Message = session.message().unwrap();
        assert_eq!(message.kind, MessageKind::Done, "{}", message.text);
    }

    #[test]
    fn browsing_a_progression_previews_it_and_enter_applies_it() {
        let mut session: Session = session();
        assert_eq!(session.selection.entry, 1);
        session.handle(Command::ExtendRight);
        session.handle(Command::ExtendRight);
        session.handle(Command::ExtendRight);
        session.handle(Command::SwitchPane);
        expect_done(&session);
        // The first progression is ii-V-I, over the verse's own rhythm.
        assert_eq!(symbols(&session, 1), vec!["Dm7", "G7", "CM7", "CM7"]);
        assert!(session.document().text().contains("| I:h / | vi:q / / / |"));
        session.handle(Command::Down);
        expect_done(&session);
        session.handle(Command::Commit);
        expect_done(&session);
        assert!(session.preview().is_none());
        // The minor ii-V-i, applied.
        assert_eq!(symbols(&session, 1), vec!["Dø7", "G7", "Cm7", "Cm7"]);
        session.handle(Command::Undo);
        assert_eq!(symbols(&session, 1), vec!["C", "Am", "Dm7", "G7"]);
    }

    #[test]
    fn a_rhythm_keeps_the_chords_and_a_bass_follows_the_lead() {
        let mut session: Session = session();
        session.handle(Command::SetMode(Mode::SectionEditor));
        session.handle(Command::SelectTrack(1));
        session.handle(Command::SwitchPane);
        // Rhythms in 4/4 come first: the whole-note pulse.
        session.handle(Command::Commit);
        expect_done(&session);
        assert!(
            session
                .document()
                .text()
                .contains("line \"Bass\"  | C2:w> | A2:q E2 A2 E2 |")
        );
        session.handle(Command::SelectTrack(0));
        session.handle(Command::SwitchPane);
        session.handle(Command::Down);
        session.handle(Command::Commit);
        expect_done(&session);
        assert!(
            session
                .document()
                .text()
                .contains("line \"Keys\"  | I:q> / / / | vi / / / |")
        );
    }

    #[test]
    fn chords_move_by_degree_and_sections_by_key() {
        let mut session: Session = session();
        session.handle(Command::Move(Step::Diatonic(1)));
        expect_done(&session);
        assert_eq!(symbols(&session, 1)[0], "Dm");
        session.handle(Command::KeyBySemitones(2));
        expect_done(&session);
        // The section moved to D major with its bass: C2 is now D2.
        assert_eq!(symbols(&session, 1)[1], "Bm");
        assert!(
            session
                .document()
                .text()
                .contains("line \"Bass\"  | D2:h A2 | B2:q F#2 B2 F#2 |")
        );
        session.handle(Command::SetMode(Mode::ArrangeSongSections));
        session.handle(Command::Repeat(1));
        expect_done(&session);
        assert!(session.document().text().contains("\"Verse\"  repeat 3"));
        session.handle(Command::ToggleMute);
        assert!(session.timeline().entries[1].muted);
    }

    #[test]
    fn bass_notes_move_through_the_leads_chord_tones() {
        let mut session: Session = session();
        session.handle(Command::SetMode(Mode::SectionEditor));
        let select = |element: usize, bar: usize| Command::SelectElement {
            entry: 1,
            bar,
            track: 1,
            element,
            pass_number: 1,
        };
        // Over the I chord (C E G), C2 up one tone is E2 ...
        session.handle(select(0, 0));
        session.handle(Command::ChordTone(1));
        expect_done(&session);
        assert!(
            session
                .document()
                .text()
                .contains("line \"Bass\"  | E2:h G2 |")
        );
        // ... and G2 down two is C2 (passing E2).
        session.handle(select(1, 0));
        session.handle(Command::ChordTone(-2));
        expect_done(&session);
        assert!(
            session
                .document()
                .text()
                .contains("line \"Bass\"  | E2:h C2 |")
        );
        // Over ii7 (D F A C), D2 up one tone is F2.
        session.handle(select(6, 2));
        session.handle(Command::ChordTone(1));
        expect_done(&session);
        assert!(session.document().text().contains("| F2:h A2 |"));
        // The lead plays chords, which chord tones do not move.
        session.handle(Command::SelectTrack(0));
        session.handle(Command::ChordTone(1));
        assert_eq!(session.message().unwrap().kind, MessageKind::Problem);
    }

    #[test]
    fn a_rest_becomes_the_chords_root_and_back() {
        let text: &str = include_str!("../../../songs_canonical/composer/rest_to_note.song");
        let document: SongDocument =
            SongDocument::new("rest_to_note.song", text.to_string()).unwrap();
        let libraries: Libraries =
            Libraries::read("p.library", PROGRESSIONS, "r.library", RHYTHMS).unwrap();
        let mut session: Session = Session::new(document, libraries);
        session.handle(Command::SetMode(Mode::SectionEditor));
        let select = |track: usize, bar: usize, element: usize| Command::SelectElement {
            entry: 0,
            bar,
            track,
            element,
            pass_number: 1,
        };
        // The bass rest on beat 3 of the C bar becomes C2, a quarter note
        // (written `/`, the same note again).
        session.handle(select(1, 0, 1));
        session.handle(Command::ToggleRest);
        expect_done(&session);
        assert!(
            session
                .document()
                .text()
                .contains("line \"Bass\"  | C2:h /:q G2 |")
        );
        // And back to a rest.
        session.handle(Command::ToggleRest);
        expect_done(&session);
        assert!(
            session
                .document()
                .text()
                .contains("line \"Bass\"  | C2:h r:q G2 |")
        );
        // The keys rest in the G bar becomes the lead's chord there, V.
        session.handle(select(0, 1, 2));
        session.handle(Command::ToggleRest);
        expect_done(&session);
        assert!(
            session
                .document()
                .text()
                .contains("line \"Keys\"  | I:w | V:h / |")
        );
    }

    #[test]
    fn the_staff_steps_through_each_time_a_repeated_entry_plays() {
        let mut session: Session = session();
        session.handle(Command::SetMode(Mode::SectionEditor));
        session.handle(Command::SelectTrack(1));
        let place = |session: &Session| {
            (
                session.selection.entry,
                session.selection.pass_number,
                session.selection.element,
            )
        };
        assert_eq!(place(&session), (1, 1, 0));
        // The verse (twice) has 12 bass notes: past the last, the second
        // time through starts ...
        for _ in 0..12 {
            session.handle(Command::Right);
        }
        assert_eq!(place(&session), (1, 2, 0));
        // ... and after it, the chorus.
        for _ in 0..12 {
            session.handle(Command::Right);
        }
        assert_eq!(place(&session), (2, 1, 0));
        // Back from the chorus: the verse's last note, second time through.
        session.handle(Command::Left);
        assert_eq!(place(&session), (1, 2, 11));
        // Left from the second time's first note: the first time's last.
        for _ in 0..12 {
            session.handle(Command::Left);
        }
        assert_eq!(place(&session), (1, 1, 11));
    }

    /// What a track plays in an entry, as the staff shows it: each
    /// element's bar, place and pitches.
    fn played(session: &Session, entry: usize, track: usize) -> Vec<(usize, u64, Vec<String>)> {
        session.timeline().entries[entry]
            .bars
            .iter()
            .enumerate()
            .flat_map(|(bar_index, bar)| {
                bar.tracks[track].elements.iter().map(move |element| {
                    (
                        bar_index,
                        element.offset_ticks,
                        element.pitches.iter().map(ToString::to_string).collect(),
                    )
                })
            })
            .collect()
    }

    #[test]
    fn a_line_converts_between_coupled_and_sampled_keeping_its_notes() {
        let mut session: Session = session();
        session.handle(Command::SetMode(Mode::SectionEditor));
        session.handle(Command::SelectTrack(1));
        let before: Vec<(usize, u64, Vec<String>)> = played(&session, 1, 1);
        // The first press only asks.
        session.handle(Command::ConvertLineForm);
        expect_done(&session);
        assert!(
            !session
                .document()
                .text()
                .contains("\"Bass\"  sample_source")
        );
        session.handle(Command::ConvertLineForm);
        expect_done(&session);
        let text: &str = session.document().text();
        assert!(
            text.contains(
                "line \"Bass\"  sample_source  | C2 G2 | A2 E2 A2 E2 | D2 A2 | G2 D2 G2 B2 |"
            ),
            "{text}"
        );
        assert!(
            text.contains("line \"Bass\"  | :h :h | :q :q :q :q | :h :h | :q :q :q :q |"),
            "{text}"
        );
        assert_eq!(played(&session, 1, 1), before);
        // And back: the same notes, composed into the line again.
        session.handle(Command::ConvertLineForm);
        session.handle(Command::ConvertLineForm);
        expect_done(&session);
        assert!(
            !session
                .document()
                .text()
                .contains("\"Bass\"  sample_source")
        );
        assert_eq!(played(&session, 1, 1), before);
        // A change in the source while a note sounds cannot be coupled.
        let text: &str = include_str!("../../../songs_canonical/composer/sampled_hold.song");
        let document: SongDocument =
            SongDocument::new("sampled_hold.song", text.to_string()).unwrap();
        let libraries: Libraries =
            Libraries::read("p.library", PROGRESSIONS, "r.library", RHYTHMS).unwrap();
        let mut held: Session = Session::new(document, libraries);
        held.handle(Command::SetMode(Mode::SectionEditor));
        held.handle(Command::SelectTrack(1));
        held.handle(Command::ConvertLineForm);
        held.handle(Command::ConvertLineForm);
        assert_eq!(held.message().unwrap().kind, MessageKind::Problem);
        assert!(held.message().unwrap().text.contains("track and hold"));
    }

    #[test]
    fn a_rhythm_applied_to_a_coupled_bass_keeps_its_composed_notes() {
        let mut session: Session = session();
        session.handle(Command::SetMode(Mode::SectionEditor));
        session.handle(Command::SelectTrack(1));
        session.handle(Command::SwitchPane);
        // Rhythms in 4/4 come first: the whole-note pulse, then quarters.
        session.handle(Command::Down);
        session.handle(Command::Commit);
        expect_done(&session);
        // The bar held C2 and G2 as half notes; the quarters play them,
        // not the lead's root (C2) four times.
        let text: &str = session.document().text();
        assert!(text.contains("line \"Bass\"  | C2:q> / G2 / |"), "{text}");
    }

    #[test]
    fn the_bridge_takes_a_progression_into_its_source() {
        let mut session: Session = session();
        session.handle(Command::SelectEntry(3));
        session.handle(Command::ExtendRight);
        session.handle(Command::SetMode(Mode::SectionEditor));
        session.handle(Command::SetTab(LibraryTab::Progressions));
        session.handle(Command::SwitchPane);
        session.handle(Command::Commit);
        expect_done(&session);
        assert!(
            session
                .document()
                .text()
                .contains("line \"Keys\"  sample_source  | ii7 | V7 |")
        );
    }

    #[test]
    fn a_new_section_comes_from_a_progression() {
        let mut session: Session = session();
        session.handle(Command::NewSectionFromProgression);
        expect_done(&session);
        let names: Vec<&str> = session
            .timeline()
            .entries
            .iter()
            .map(|entry| entry.section_name.as_str())
            .collect();
        assert_eq!(names[2], "ii-V-I");
        assert_eq!(symbols(&session, 2), vec!["Dm7", "G7", "CM7", "CM7"]);
    }
}

#[cfg(test)]
mod arrange_tests {
    use super::*;
    use crate::arrange_editor::ArrangeKey;

    const ARRANGE: &str = include_str!("../../../songs_canonical/composer/arrange_song.song");

    fn press(session: &mut Session, keys: &[ArrangeKey]) {
        for key in keys {
            session.handle(Command::Arrange(*key));
            if let Some(message) = session.message() {
                assert_eq!(message.kind, MessageKind::Done, "{}", message.text);
            }
        }
    }

    const AFTER_F1: &str =
        include_str!("../../../songs_canonical/composer/arrange_song_after_f1.song");

    /// The text after the two comment lines every canonical file starts
    /// with, which say which test reads it.
    fn without_header(text: &str) -> String {
        text.lines().skip(2).collect::<Vec<&str>>().join("\n")
    }

    /// A series of F1 keys writes exactly the expected text, which the
    /// song file's reader then plays as the text player would.
    #[test]
    fn f1_edits_write_the_song_composition() {
        use ArrangeKey::{Backspace, Delete, Down, End, Enter, Left, NewSection, Plus, Right, Up};
        let document: SongDocument =
            SongDocument::new("arrange_song.song", ARRANGE.to_string()).unwrap();
        let libraries: Libraries = Libraries::read(
            "p.library",
            include_str!("../../../songs_canonical/composer/composer_progressions.library"),
            "r.library",
            include_str!("../../../songs_canonical/composer/composer_rhythms.library"),
        )
        .unwrap();
        let mut session: Session = Session::new(document, libraries);
        session.handle(Command::SetMode(Mode::ArrangeSongSections));
        press(&mut session, &[Right, Plus, Up, Enter, Right, Enter]);
        press(&mut session, &[Down, Down, Delete]);
        press(
            &mut session,
            &[Right, Right, Enter, Backspace, Backspace, Backspace],
        );
        session.type_text("112.5");
        press(
            &mut session,
            &[
                Enter, Down, Plus, Down, Down, Left, Left, Left, Left, Enter, End,
            ],
        );
        session.type_text(" B");
        press(&mut session, &[Enter, NewSection]);
        session.type_text("Outro");
        press(&mut session, &[Down]);
        session.type_text("-");
        press(&mut session, &[Down, Right, Down]);
        session.type_text("80");
        press(&mut session, &[Enter]);
        assert_eq!(
            without_header(session.document().text()),
            without_header(AFTER_F1)
        );
        assert!(read_song("arrange_song_after_f1.song", AFTER_F1).is_ok());
        // Undo takes back the new section in one step.
        session.handle(Command::Undo);
        assert!(!session.document().text().contains("Outro"));
    }

    #[test]
    fn f1_refuses_what_the_notation_does_not_allow() {
        use ArrangeKey::{Delete, Down, Enter, Right};
        let document: SongDocument =
            SongDocument::new("arrange_song.song", ARRANGE.to_string()).unwrap();
        let libraries: Libraries = Libraries::read(
            "p.library",
            include_str!("../../../songs_canonical/composer/composer_progressions.library"),
            "r.library",
            include_str!("../../../songs_canonical/composer/composer_rhythms.library"),
        )
        .unwrap();
        let mut session: Session = Session::new(document, libraries);
        session.handle(Command::SetMode(Mode::ArrangeSongSections));
        let problem = |session: &Session| {
            session
                .message()
                .is_some_and(|message| message.kind == MessageKind::Problem)
        };
        // The Riff has music, so F1 does not change its meter.
        session.handle(Command::Arrange(Down));
        for _ in 0..3 {
            session.handle(Command::Arrange(Right));
        }
        session.handle(Command::Arrange(Enter));
        assert!(problem(&session));
        assert!(!session.arrange_has_dialog());
        // The Intro writes no key of its own, so there is none to clear.
        session.handle(Command::Arrange(ArrangeKey::Up));
        session.handle(Command::Arrange(ArrangeKey::Left));
        session.handle(Command::Arrange(Delete));
        assert!(problem(&session));
        assert_eq!(session.document().text(), ARRANGE);
    }
}
