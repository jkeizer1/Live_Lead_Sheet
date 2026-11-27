//! What each F1 edit writes into the song's text, and the words that say
//! it was done. The session keeps the new text only when the song file's
//! reader accepts it.

use vocabulary::song::Song;

use crate::arrange_editor::ArrangeEdit;
use crate::composition_edits::{
    add_change_after, add_entry_after, add_new_section, meter_words, move_row, remove_change,
    rename_section, set_change, set_section_meter, set_start_setting,
};
use crate::edits::{
    duplicate_entry, remove_entry, set_entry_key, set_entry_repeat, set_entry_tempo,
};
use crate::key_signature::key_text;
use crate::song_source::{CompositionRowSource, SongSource};

/// The change line at `line_index`, as written.
fn change_at(source: &SongSource, line_index: usize) -> Option<(Option<String>, Option<String>)> {
    source
        .composition
        .as_ref()?
        .rows
        .iter()
        .find_map(|row: &CompositionRowSource| match row {
            CompositionRowSource::Change(change) if change.index == line_index => {
                Some((change.key_words.clone(), change.tempo_words.clone()))
            }
            _ => None,
        })
}

fn entry_name(source: &SongSource, entry: usize) -> String {
    source
        .composition
        .as_ref()
        .and_then(|composition| composition.entries.get(entry))
        .map_or_else(|| String::from("?"), |written| written.section_name.clone())
}

/// The new text for `edit`, and what was done in words.
///
/// # Errors
///
/// When the edit cannot be written (the message says why).
// One arm per kind of edit, each a call and its words; split up, the list
// of what F1 can write would be scattered.
#[allow(clippy::too_many_lines)]
pub fn write_arrange_edit(
    edit: &ArrangeEdit,
    text: &str,
    source: &SongSource,
    song: &Song,
) -> Result<(String, String), String> {
    match edit {
        ArrangeEdit::StartKey(key) => Ok((
            set_start_setting(text, source, "key", &key_text(*key))?,
            format!("The song starts in {key}"),
        )),
        ArrangeEdit::StartMeter(meter) => Ok((
            set_start_setting(text, source, "meter", &meter_words(*meter))?,
            format!(
                "The song's tempo counts the beats of {}",
                meter_words(*meter)
            ),
        )),
        ArrangeEdit::StartTempo(words) => Ok((
            set_start_setting(text, source, "tempo", words)?,
            format!("The song starts at {words}"),
        )),
        ArrangeEdit::ChangeKey { line_index, key } => {
            let (_, tempo) = change_at(source, *line_index)
                .ok_or_else(|| String::from("that row is not a key or tempo change"))?;
            let key_words: Option<String> = key.map(key_text);
            let what: String = match (&key_words, &tempo) {
                (Some(key), _) => format!("From here on the song is in {key}"),
                (None, Some(_)) => String::from("The change line no longer changes the key"),
                (None, None) => String::from("Change line removed"),
            };
            Ok((
                set_change(
                    text,
                    source,
                    *line_index,
                    key_words.as_deref(),
                    tempo.as_deref(),
                )?,
                what,
            ))
        }
        ArrangeEdit::ChangeTempo {
            line_index,
            tempo_words,
        } => {
            let (key, _) = change_at(source, *line_index)
                .ok_or_else(|| String::from("that row is not a key or tempo change"))?;
            let what: String = match (tempo_words, &key) {
                (Some(words), _) => format!("From here on the song plays at {words}"),
                (None, Some(_)) => String::from("The change line no longer changes the tempo"),
                (None, None) => String::from("Change line removed"),
            };
            Ok((
                set_change(
                    text,
                    source,
                    *line_index,
                    key.as_deref(),
                    tempo_words.as_deref(),
                )?,
                what,
            ))
        }
        ArrangeEdit::EntryKey { entry, key } => {
            let name: String = entry_name(source, *entry);
            let what: String = match key {
                Some(key) => format!("Entry \"{name}\" plays in {key}"),
                None => format!("Entry \"{name}\" plays in the key it inherits"),
            };
            Ok((
                set_entry_key(text, source, *entry, key.map(key_text))?,
                what,
            ))
        }
        ArrangeEdit::SectionInOtherMode {
            entry,
            new_name,
            key,
        } => Ok((
            crate::mode_section::section_in_other_mode(text, source, song, *entry, new_name, *key)?,
            format!(
                "New section \"{new_name}\" in {key}, each note on its scale step; the entry \
                 plays it. Look it over in F3"
            ),
        )),
        ArrangeEdit::EntrySectionKey { entry } => {
            let name: String = entry_name(source, *entry);
            Ok((
                set_entry_key(
                    text,
                    source,
                    *entry,
                    Some(String::from(crate::arrangement::SECTION_KEY_WORD)),
                )?,
                format!("Entry \"{name}\" plays its section's own key"),
            ))
        }
        ArrangeEdit::EntryTempo { entry, tempo_words } => {
            let name: String = entry_name(source, *entry);
            let what: String = match tempo_words {
                Some(words) => format!("Entry \"{name}\" plays at {words}"),
                None => format!("Entry \"{name}\" plays at the tempo it inherits"),
            };
            Ok((
                set_entry_tempo(text, source, *entry, tempo_words.clone())?,
                what,
            ))
        }
        ArrangeEdit::EntryRepeat {
            entry,
            repeat_count,
        } => {
            let name: String = entry_name(source, *entry);
            let what: String = match repeat_count {
                0 => format!("Entry \"{name}\" left out (repeat 0)"),
                1 => format!("Entry \"{name}\" plays once"),
                count => format!("Entry \"{name}\" plays {count} times"),
            };
            Ok((set_entry_repeat(text, source, *entry, *repeat_count)?, what))
        }
        ArrangeEdit::SectionMeter {
            section_name,
            meter,
        } => Ok((
            set_section_meter(text, source, song, section_name, *meter)?,
            format!("Section \"{section_name}\" is in {}", meter_words(*meter)),
        )),
        ArrangeEdit::Rename { old_name, new_name } => Ok((
            rename_section(text, source, old_name, new_name)?,
            format!("Section \"{old_name}\" is now \"{new_name}\", in every entry"),
        )),
        ArrangeEdit::RemoveEntry(entry) => {
            let name: String = entry_name(source, *entry);
            Ok((
                remove_entry(text, source, *entry)?,
                format!("Entry \"{name}\" removed (the section is kept)"),
            ))
        }
        ArrangeEdit::RemoveChange(line_index) => Ok((
            remove_change(text, source, *line_index)?,
            String::from("Change line removed"),
        )),
        ArrangeEdit::MoveRow { row, delta } => {
            let direction: &str = if *delta < 0 { "up" } else { "down" };
            Ok((
                move_row(text, source, *row, *delta)?,
                format!("Row moved {direction}"),
            ))
        }
        ArrangeEdit::Duplicate(entry) => {
            let name: String = entry_name(source, *entry);
            Ok((
                duplicate_entry(text, source, *entry)?,
                format!("Entry \"{name}\" written again after itself"),
            ))
        }
        ArrangeEdit::AddChange { after_row, key } => Ok((
            add_change_after(text, source, *after_row, *key)?,
            String::from("Key change added. Choose its key, or give it a tempo"),
        )),
        ArrangeEdit::AddEntry {
            after_row,
            section_name,
        } => Ok((
            add_entry_after(text, source, *after_row, section_name)?,
            format!("Entry \"{section_name}\" added"),
        )),
        ArrangeEdit::NewSection { after_row, section } => Ok((
            add_new_section(text, source, song, *after_row, section)?,
            format!(
                "Section \"{}\" made, with one bar of rest. Compose it in the Section Editor (F3)",
                section.name
            ),
        )),
    }
}
