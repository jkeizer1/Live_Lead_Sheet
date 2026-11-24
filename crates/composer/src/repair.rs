//! Repairing a song and its studio that the readers refused. Nothing here
//! opens a window or a file: the app hands in the file texts, the screen
//! draws the problem list, and the app saves what comes back.
//!
//! Every change rewrites a file's text and reads both files again, so the
//! list always shows what is still wrong. A line can be edited, a line can
//! be inserted after a problem line (for a missing line, such as the lead
//! track), or a line can be tossed. A tossed line is noted in the file's
//! comment block at the top, so nothing the musician typed vanishes.
//! Every change can be undone.

use song_text::song_file::parse_song;
use song_text::studio_file::parse_studio;
use song_text::text_line::leading_comments;

use crate::scroll_list::ScrollList;
use crate::text_field::TextField;

/// How many problems the screen lists at once.
pub const VISIBLE_PROBLEMS: usize = 12;

/// Which reader a file belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairFileKind {
    Song,
    Studio,
}

/// One file being repaired: its name for messages and its current text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairFile {
    pub kind: RepairFileKind,
    pub file_name: String,
    pub text: String,
}

/// One thing a reader refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// Index into `Repair::files`.
    pub file_index: usize,
    /// 1-based, as the reader counts.
    pub line_number: usize,
    pub message: String,
    /// The text of that line now, empty if the line is past the file's end.
    pub line_text: String,
}

/// A place on the repair screen a click can land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairTarget {
    Row(usize),
    /// The scroll bar above (`up`) or below its thumb: one page.
    Page {
        up: bool,
    },
    Edit,
    Insert,
    Toss,
    TossAll,
    Undo,
    Save,
    Accept,
    Cancel,
}

/// What the typed line will do when it is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    /// Replace the problem line.
    Replace,
    /// Add a new line after the problem line.
    InsertAfter,
}

/// The line being typed, and what it will do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineEdit {
    pub mode: EditMode,
    pub field: TextField,
}

#[derive(Debug, Clone)]
pub struct Repair {
    files: Vec<RepairFile>,
    original_texts: Vec<String>,
    problems: Vec<Problem>,
    list: ScrollList,
    editing: Option<LineEdit>,
    /// The file texts before each change, latest last.
    undo: Vec<Vec<String>>,
    /// The date put in a tossed-line note, as the musician writes dates.
    today: String,
}

impl Repair {
    /// Reads the files and lists what is wrong with them. `today` goes in
    /// the note for each tossed line.
    #[must_use]
    pub fn new(files: Vec<RepairFile>, today: &str) -> Repair {
        let original_texts: Vec<String> = files.iter().map(|f| f.text.clone()).collect();
        let mut repair: Repair = Repair {
            files,
            original_texts,
            problems: Vec::new(),
            list: ScrollList::new(VISIBLE_PROBLEMS),
            editing: None,
            undo: Vec::new(),
            today: today.to_string(),
        };
        repair.check();
        repair.list.reset(repair.problems.len());
        repair
    }

    #[must_use]
    pub fn files(&self) -> &[RepairFile] {
        &self.files
    }

    #[must_use]
    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }

    #[must_use]
    pub fn list(&self) -> &ScrollList {
        &self.list
    }

    pub fn list_mut(&mut self) -> &mut ScrollList {
        &mut self.list
    }

    #[must_use]
    pub fn editing(&self) -> Option<&LineEdit> {
        self.editing.as_ref()
    }

    pub fn editing_mut(&mut self) -> Option<&mut LineEdit> {
        self.editing.as_mut()
    }

    /// Both files read cleanly: the song can be opened.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.problems.is_empty()
    }

    /// A file's text differs from what was loaded, so it needs saving.
    #[must_use]
    pub fn changed(&self) -> bool {
        self.files
            .iter()
            .zip(&self.original_texts)
            .any(|(file, original): (&RepairFile, &String)| file.text != *original)
    }

    /// One file's text differs from what was loaded.
    #[must_use]
    pub fn is_file_changed(&self, file_index: usize) -> bool {
        match (
            self.files.get(file_index),
            self.original_texts.get(file_index),
        ) {
            (Some(file), Some(original)) => file.text != *original,
            _ => false,
        }
    }

    /// The files were written: what they hold now is the new starting
    /// point for "changed" (undo still goes back through every change).
    pub fn mark_saved(&mut self) {
        self.original_texts = self
            .files
            .iter()
            .map(|f: &RepairFile| f.text.clone())
            .collect();
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn selected_problem(&self) -> Option<&Problem> {
        self.problems.get(self.list.selected())
    }

    /// Starts typing over the selected problem's line.
    pub fn begin_edit(&mut self) {
        if let Some(problem) = self.selected_problem() {
            self.editing = Some(LineEdit {
                mode: EditMode::Replace,
                field: TextField::new(&problem.line_text),
            });
        }
    }

    /// Starts typing a new line to go after the selected problem's line.
    pub fn begin_insert(&mut self) {
        if self.selected_problem().is_some() {
            self.editing = Some(LineEdit {
                mode: EditMode::InsertAfter,
                field: TextField::new(""),
            });
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing = None;
    }

    /// Puts the typed line in the file and reads the files again.
    pub fn accept_edit(&mut self) {
        let Some(edit) = self.editing.take() else {
            return;
        };
        let Some(problem) = self.selected_problem().cloned() else {
            return;
        };
        self.remember();
        let mut lines: Vec<String> = lines_of(&self.files[problem.file_index].text);
        let typed: String = edit.field.text().to_string();
        match edit.mode {
            EditMode::Replace => {
                if let Some(line) = lines.get_mut(problem.line_number - 1) {
                    *line = typed;
                }
            }
            EditMode::InsertAfter => {
                let at: usize = problem.line_number.min(lines.len());
                lines.insert(at, typed);
            }
        }
        self.files[problem.file_index].text = lines.join("\n");
        self.recheck();
    }

    /// Removes the selected problem's line and notes it in the file's
    /// comment block.
    pub fn toss_selected(&mut self) {
        let Some(problem) = self.selected_problem().cloned() else {
            return;
        };
        self.remember();
        self.toss_lines(problem.file_index, &[problem.line_number]);
        self.recheck();
    }

    /// Removes every problem line in one pass (the lines as the readers
    /// numbered them now, so removing one does not move another), each noted
    /// in its file's comment block. The app asks first: a problem can sit on
    /// a line the song needs.
    pub fn toss_all(&mut self) {
        if self.problems.is_empty() {
            return;
        }
        self.remember();
        for file_index in 0..self.files.len() {
            let line_numbers: Vec<usize> = self
                .problems
                .iter()
                .filter(|p: &&Problem| p.file_index == file_index)
                .map(|p: &Problem| p.line_number)
                .collect();
            self.toss_lines(file_index, &line_numbers);
        }
        self.recheck();
    }

    pub fn undo(&mut self) {
        if let Some(texts) = self.undo.pop() {
            for (file, text) in self.files.iter_mut().zip(texts) {
                file.text = text;
            }
            self.editing = None;
            self.recheck();
        }
    }

    fn remember(&mut self) {
        self.undo.push(
            self.files
                .iter()
                .map(|f: &RepairFile| f.text.clone())
                .collect(),
        );
    }

    /// Removes the numbered lines (1-based) from one file and adds a note
    /// for each at the end of the file's top comment block.
    fn toss_lines(&mut self, file_index: usize, line_numbers: &[usize]) {
        let mut lines: Vec<String> = lines_of(&self.files[file_index].text);
        let mut numbers: Vec<usize> = line_numbers.to_vec();
        numbers.sort_unstable();
        numbers.dedup();
        let mut notes: Vec<String> = Vec::new();
        // Highest first, so each removal leaves the lower numbers in place.
        for number in numbers.into_iter().rev() {
            if number >= 1 && number <= lines.len() {
                let removed: String = lines.remove(number - 1);
                notes.push(format!(
                    "# Tossed {} (line {number}): {}",
                    self.today,
                    removed.trim()
                ));
            }
        }
        notes.reverse();
        let text: String = lines.join("\n");
        self.files[file_index].text = with_notes(&text, &notes);
    }

    fn recheck(&mut self) {
        self.check();
        self.list.resize(self.problems.len());
    }

    fn check(&mut self) {
        let mut problems: Vec<Problem> = Vec::new();
        for (file_index, file) in self.files.iter().enumerate() {
            let errors = match file.kind {
                RepairFileKind::Song => parse_song(&file.file_name, &file.text).err(),
                RepairFileKind::Studio => parse_studio(&file.file_name, &file.text).err(),
            };
            let lines: Vec<String> = lines_of(&file.text);
            for error in errors.unwrap_or_default() {
                problems.push(Problem {
                    file_index,
                    line_number: error.line_number(),
                    message: error.message().to_string(),
                    line_text: lines
                        .get(error.line_number().saturating_sub(1))
                        .cloned()
                        .unwrap_or_default(),
                });
            }
        }
        self.problems = problems;
    }
}

fn lines_of(text: &str) -> Vec<String> {
    text.split('\n').map(str::to_string).collect()
}

/// `text` with the notes added after the last comment line of its top
/// comment block, or at the very top when it has none.
fn with_notes(text: &str, notes: &[String]) -> String {
    if notes.is_empty() {
        return text.to_string();
    }
    let block: &str = leading_comments(text);
    let content_end: usize = block.trim_end().len();
    let note_lines: String = notes.join("\n");
    if content_end == 0 {
        format!("{note_lines}\n{text}")
    } else {
        format!(
            "{}\n{note_lines}{}",
            &text[..content_end],
            &text[content_end..]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Repair, RepairFile, RepairFileKind};

    const JUNK_SONG: &str = include_str!("../../../songs_canonical/repair/junk_lines.song");
    const JUNK_STUDIO: &str = include_str!("../../../songs_canonical/repair/junk_lines.studio");
    const MISSING_TYPE: &str = include_str!("../../../songs_canonical/repair/missing_type.song");
    const CLEAN_SONG: &str = include_str!("../../../songs_canonical/repair/clean.song");
    const CLEAN_STUDIO: &str = include_str!("../../../songs_canonical/repair/clean.studio");

    fn pair(song: &str, studio: &str) -> Repair {
        Repair::new(
            vec![
                RepairFile {
                    kind: RepairFileKind::Song,
                    file_name: String::from("t.song"),
                    text: song.to_string(),
                },
                RepairFile {
                    kind: RepairFileKind::Studio,
                    file_name: String::from("t.studio"),
                    text: studio.to_string(),
                },
            ],
            "24 Nov 2026",
        )
    }

    #[test]
    fn a_clean_pair_has_no_problems() {
        let repair: Repair = pair(CLEAN_SONG, CLEAN_STUDIO);
        assert!(repair.is_clean(), "{:?}", repair.problems());
        assert!(!repair.changed());
    }

    #[test]
    fn problems_name_their_file_and_line() {
        let repair: Repair = pair(JUNK_SONG, JUNK_STUDIO);
        let found: Vec<(usize, &str)> = repair
            .problems()
            .iter()
            .map(|p| (p.file_index, p.line_text.trim()))
            .collect();
        assert_eq!(
            found,
            vec![
                (0, "banana split"),
                (0, "wobble 3"),
                (1, "settle_delay 5 ms")
            ]
        );
    }

    #[test]
    fn toss_removes_the_line_and_notes_it_at_the_top() {
        let mut repair: Repair = pair(JUNK_SONG, JUNK_STUDIO);
        repair.toss_selected();
        assert_eq!(repair.problems().len(), 2);
        let song: &str = &repair.files()[0].text;
        assert!(!song.contains("banana split\n  clef"), "{song}");
        let note: &str = song
            .lines()
            .find(|l| l.starts_with("# Tossed"))
            .expect("a note");
        assert_eq!(note, "# Tossed 24 Nov 2026 (line 15): banana split");
        // The note sits in the top comment block, before the song line.
        assert!(song.find("# Tossed").unwrap() < song.find("song \"").unwrap());
        assert!(repair.changed());
    }

    #[test]
    fn toss_all_clears_every_problem_in_one_pass() {
        let mut repair: Repair = pair(JUNK_SONG, JUNK_STUDIO);
        repair.toss_all();
        assert!(repair.is_clean(), "{:?}", repair.problems());
        assert_eq!(
            repair.files()[0]
                .text
                .lines()
                .filter(|l| l.starts_with("# Tossed"))
                .count(),
            2
        );
        assert!(repair.files()[1].text.contains("# Tossed 24 Nov 2026"));
    }

    #[test]
    fn undo_brings_back_the_texts_and_the_problems() {
        let mut repair: Repair = pair(JUNK_SONG, JUNK_STUDIO);
        repair.toss_all();
        repair.undo();
        assert_eq!(repair.files()[0].text, JUNK_SONG);
        assert_eq!(repair.files()[1].text, JUNK_STUDIO);
        assert_eq!(repair.problems().len(), 3);
        assert!(!repair.changed());
        assert!(!repair.can_undo());
    }

    #[test]
    fn editing_a_line_replaces_it() {
        let mut repair: Repair = pair(JUNK_SONG, JUNK_STUDIO);
        repair.begin_edit();
        let edit = repair.editing_mut().unwrap();
        for _ in 0.."banana split".len() {
            edit.field.backspace();
        }
        edit.field.insert("  type mono_voice");
        repair.accept_edit();
        // "lead" on a second track makes a different problem or none; the
        // banana line itself is gone either way.
        assert!(!repair.files()[0].text.contains("banana"));
    }

    #[test]
    fn inserting_a_line_repairs_a_structural_problem() {
        let mut repair: Repair = pair(MISSING_TYPE, CLEAN_STUDIO);
        assert_eq!(repair.problems()[0].line_text, "track \"Beat\"");
        repair.begin_insert();
        repair
            .editing_mut()
            .unwrap()
            .field
            .insert("  type mono_voice");
        repair.accept_edit();
        assert!(repair.is_clean(), "{:?}", repair.problems());
    }

    #[test]
    fn cancelling_an_edit_changes_nothing() {
        let mut repair: Repair = pair(JUNK_SONG, JUNK_STUDIO);
        repair.begin_edit();
        repair.editing_mut().unwrap().field.insert("x");
        repair.cancel_edit();
        assert!(repair.editing().is_none());
        assert!(!repair.changed());
    }
}
