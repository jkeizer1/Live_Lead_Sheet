//! The song's overall settings as edits of its text: the title, the chord
//! numeral standard, and each track's name, type, clef and lead. Each edit
//! gives the whole new text; the document reads it back and refuses it when
//! it would not load (EFS-6.3), so a bad choice (a second clock track, no
//! lead) is said, never saved.
//!
//! A track's name is how lines and the studio file refer to it, so renaming
//! and deleting a track reach into every section's lines; the studio side
//! is [`StudioChange`], which the app applies to the studio configuration.

use player::studio_configuration::{StudioConfiguration, TrackOutput};
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::song::{Clef, Track, TrackKind, VoiceKind};

use crate::edits::TextLines;
use crate::session::Session;
use crate::settings_view::SettingsCommand;
use crate::song_source::{SongSource, TrackSource, indentation};

/// The track types the editor offers (Song Notation, "Tracks and their
/// types"; the deferred types are not offered).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackType {
    PolyVoice,
    MonoVoice,
    ClockAndTransport,
    CueControl,
    MeterControl,
}

impl TrackType {
    pub const ALL: [TrackType; 5] = [
        TrackType::PolyVoice,
        TrackType::MonoVoice,
        TrackType::ClockAndTransport,
        TrackType::CueControl,
        TrackType::MeterControl,
    ];

    /// The word after `type` in the file.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            TrackType::PolyVoice => "poly_voice",
            TrackType::MonoVoice => "mono_voice",
            TrackType::ClockAndTransport => "clock_and_transport",
            TrackType::CueControl => "cue_control",
            TrackType::MeterControl => "meter_control",
        }
    }

    /// Only a voice has a clef and can be the lead.
    #[must_use]
    pub const fn is_voice(self) -> bool {
        matches!(self, TrackType::PolyVoice | TrackType::MonoVoice)
    }

    /// The next type in the list, after the last the first.
    #[must_use]
    pub fn next(self) -> TrackType {
        let at: usize = TrackType::ALL.iter().position(|t| *t == self).unwrap_or(0);
        TrackType::ALL[(at + 1) % TrackType::ALL.len()]
    }

    /// The type of a track the song holds.
    #[must_use]
    pub fn of(track: &Track) -> TrackType {
        match track.kind() {
            TrackKind::Voice {
                voice: VoiceKind::Poly,
                ..
            } => TrackType::PolyVoice,
            TrackKind::Voice {
                voice: VoiceKind::Mono,
                ..
            } => TrackType::MonoVoice,
            TrackKind::ClockAndTransport => TrackType::ClockAndTransport,
            TrackKind::CueControl { .. } => TrackType::CueControl,
            TrackKind::MeterControl => TrackType::MeterControl,
        }
    }
}

/// Every clef, in the order the editor steps through them.
pub const CLEFS: [Clef; 6] = [
    Clef::Treble,
    Clef::Bass,
    Clef::Alto,
    Clef::Tenor,
    Clef::Percussion,
    Clef::Grand,
];

/// The word after `clef` in the file.
#[must_use]
pub const fn clef_word(clef: Clef) -> &'static str {
    match clef {
        Clef::Treble => "treble",
        Clef::Bass => "bass",
        Clef::Alto => "alto",
        Clef::Tenor => "tenor",
        Clef::Percussion => "percussion",
        Clef::Grand => "grand",
    }
}

/// What a track edit changes in the studio configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudioChange {
    Rename { old_name: String, new_name: String },
    Remove { track_name: String },
}

/// A name that can be written between double quotes and read back.
fn check_name(what: &str, name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.contains(['"', '\n', '\r']) {
        return Err(format!(
            "A {what} name needs at least one character and no double quote."
        ));
    }
    Ok(())
}

fn track_of<'a>(source: &'a SongSource, name: &str) -> Result<&'a TrackSource, String> {
    source
        .track(name)
        .ok_or_else(|| format!("The song has no track \"{name}\"."))
}

/// Replaces the text of line `index`, keeping its indentation.
fn replace_body(lines: &mut TextLines, index: usize, body: &str) -> Result<(), String> {
    let line: &mut String = lines
        .lines
        .get_mut(index)
        .ok_or_else(|| format!("The song's text has no line {}.", index + 1))?;
    *line = format!("{}{body}", indentation(line));
    Ok(())
}

/// Removes the lines at `indices`, highest first so the rest keep their
/// places.
fn remove_indices(lines: &mut TextLines, indices: &[usize]) {
    let mut sorted: Vec<usize> = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    for index in sorted.into_iter().rev() {
        if index < lines.lines.len() {
            lines.lines.remove(index);
        }
    }
}

/// # Errors
///
/// When the title is empty or has a double quote, or the text has no
/// `song` line.
pub fn set_title(text: &str, source: &SongSource, title: &str) -> Result<String, String> {
    check_name("song", title)?;
    let index: usize = source
        .song_index
        .ok_or_else(|| String::from("The song has no song line."))?;
    let mut lines: TextLines = TextLines::of(text);
    replace_body(&mut lines, index, &format!("song \"{title}\""))?;
    Ok(lines.joined())
}

/// Sets, changes or (with `None`) removes the `chord_numeral_standard`
/// line. Removing it from a song that needs it is refused by the reader.
///
/// # Errors
///
/// When the text has no `song` line to put it after.
pub fn set_numeral_standard(
    text: &str,
    source: &SongSource,
    standard: Option<NumeralStandard>,
) -> Result<String, String> {
    let mut lines: TextLines = TextLines::of(text);
    match (standard, source.numeral_standard_index) {
        (Some(standard), Some(index)) => {
            replace_body(
                &mut lines,
                index,
                &format!("chord_numeral_standard {}", standard.text()),
            )?;
        }
        (Some(standard), None) => {
            let after: usize = source
                .song_index
                .ok_or_else(|| String::from("The song has no song line."))?;
            lines.lines.insert(
                after + 1,
                format!("chord_numeral_standard {}", standard.text()),
            );
        }
        (None, Some(index)) => remove_indices(&mut lines, &[index]),
        (None, None) => {}
    }
    Ok(lines.joined())
}

/// Renames a track in its block, in every section's line for it, and in
/// cues that send to it (`destination track "Name"`).
///
/// # Errors
///
/// When the track does not exist, or the new name is empty, has a double
/// quote, or is another track's.
pub fn rename_track(
    text: &str,
    source: &SongSource,
    old_name: &str,
    new_name: &str,
) -> Result<String, String> {
    check_name("track", new_name)?;
    let track: &TrackSource = track_of(source, old_name)?;
    if new_name != old_name && source.track(new_name).is_some() {
        return Err(format!("The song already has a track \"{new_name}\"."));
    }
    let mut lines: TextLines = TextLines::of(text);
    let old_quoted: String = format!("\"{old_name}\"");
    let new_quoted: String = format!("\"{new_name}\"");
    let mut section_line_starts: Vec<usize> = vec![track.header_index];
    for section in &source.sections {
        for line in &section.lines {
            if line.track_name == old_name {
                section_line_starts.push(line.first_index);
            }
        }
    }
    for index in section_line_starts {
        if let Some(line) = lines.lines.get_mut(index) {
            *line = line.replacen(&old_quoted, &new_quoted, 1);
        }
    }
    let to_old: String = format!("destination track {old_quoted}");
    let to_new: String = format!("destination track {new_quoted}");
    for line in &mut lines.lines {
        if line.trim_start().starts_with("cue ") && line.contains(&to_old) {
            *line = line.replace(&to_old, &to_new);
        }
    }
    Ok(lines.joined())
}

/// What a voice track made in Song settings plays: music. Song settings has
/// no control for `notes` yet, so a track for a sample player or a gate is
/// changed to `notes midi_note` in the text.
const NEW_VOICE_NOTES: &str = "notes pitched";

/// Changes a track's type. A voice needs its notes and a clef (`notes
/// pitched` and `clef treble` are written when there are none); a type that
/// is not a voice drops the notes, clef, `lead` and voice layer lines it
/// cannot have.
///
/// # Errors
///
/// When the track does not exist, or has no `type` line.
pub fn set_type(
    text: &str,
    source: &SongSource,
    track_name: &str,
    new_type: TrackType,
) -> Result<String, String> {
    let track: &TrackSource = track_of(source, track_name)?;
    let type_index: usize = track
        .type_index
        .ok_or_else(|| format!("Track \"{track_name}\" has no type line."))?;
    let mut lines: TextLines = TextLines::of(text);
    replace_body(&mut lines, type_index, &format!("type {}", new_type.word()))?;
    if new_type.is_voice() {
        let indent: String = lines
            .lines
            .get(type_index)
            .map(|line: &String| indentation(line).to_string())
            .unwrap_or_default();
        // Inserted below the type line, the clef first so the notes end up
        // above it.
        if track.clef_index.is_none() {
            lines
                .lines
                .insert(type_index + 1, format!("{indent}clef treble"));
        }
        if track.notes_index.is_none() {
            lines
                .lines
                .insert(type_index + 1, format!("{indent}{NEW_VOICE_NOTES}"));
        }
    } else {
        let mut dropped: Vec<usize> = track.voice_layer_indices.clone();
        dropped.extend(track.notes_index);
        dropped.extend(track.clef_index);
        dropped.extend(track.lead_index);
        remove_indices(&mut lines, &dropped);
    }
    Ok(lines.joined())
}

/// Changes a voice track's clef.
///
/// # Errors
///
/// When the track does not exist or has no clef (it is not a voice).
pub fn set_clef(
    text: &str,
    source: &SongSource,
    track_name: &str,
    clef: Clef,
) -> Result<String, String> {
    let track: &TrackSource = track_of(source, track_name)?;
    let index: usize = track
        .clef_index
        .ok_or_else(|| format!("Track \"{track_name}\" is not a voice, so it has no clef."))?;
    let mut lines: TextLines = TextLines::of(text);
    replace_body(&mut lines, index, &format!("clef {}", clef_word(clef)))?;
    Ok(lines.joined())
}

/// Makes a voice track the lead: the others lose `lead`.
///
/// # Errors
///
/// When the track does not exist or is not a voice.
pub fn make_lead(text: &str, source: &SongSource, track_name: &str) -> Result<String, String> {
    let track: &TrackSource = track_of(source, track_name)?;
    let after: usize = track.clef_index.ok_or_else(|| {
        format!("Track \"{track_name}\" is not a voice, so it cannot be the lead.")
    })?;
    if track.lead_index.is_some() {
        return Ok(text.to_string());
    }
    let mut lines: TextLines = TextLines::of(text);
    let others: Vec<usize> = source
        .tracks
        .iter()
        .filter(|other: &&TrackSource| other.name != track_name)
        .filter_map(|other: &TrackSource| other.lead_index)
        .collect();
    let indent: String = lines
        .lines
        .get(after)
        .map(|line: &String| indentation(line).to_string())
        .unwrap_or_default();
    lines.lines.insert(after + 1, format!("{indent}lead"));
    // The other leads sit above or below the new line; remove them by
    // their old places, shifted past the line just inserted.
    let shifted: Vec<usize> = others
        .into_iter()
        .map(|index: usize| if index > after { index + 1 } else { index })
        .collect();
    remove_indices(&mut lines, &shifted);
    Ok(lines.joined())
}

/// Adds a track after the last one. A voice gets `clef treble`, and `lead`
/// when the song has no voice yet (a song with voices needs exactly one
/// lead).
///
/// # Errors
///
/// When the name is empty, has a double quote, or is taken.
pub fn add_track(
    text: &str,
    source: &SongSource,
    song: &vocabulary::song::Song,
    name: &str,
    track_type: TrackType,
) -> Result<String, String> {
    check_name("track", name)?;
    if source.track(name).is_some() {
        return Err(format!("The song already has a track \"{name}\"."));
    }
    let indent: String = source
        .tracks
        .iter()
        .find_map(|track: &TrackSource| track.type_index)
        .and_then(|index: usize| text.lines().nth(index))
        .map_or_else(
            || String::from("  "),
            |line: &str| indentation(line).to_string(),
        );
    let mut block: Vec<String> = vec![
        String::new(),
        format!("track \"{name}\""),
        format!("{indent}type {}", track_type.word()),
    ];
    if track_type.is_voice() {
        block.push(format!("{indent}{NEW_VOICE_NOTES}"));
        block.push(format!("{indent}clef treble"));
        let song_has_voice: bool = song
            .tracks()
            .iter()
            .any(|track: &Track| matches!(track.kind(), TrackKind::Voice { .. }));
        if !song_has_voice {
            block.push(format!("{indent}lead"));
        }
    }
    let after: usize = source
        .tracks
        .last()
        .map(|track: &TrackSource| track.last_index)
        .or(source.numeral_standard_index)
        .or(source.song_index)
        .ok_or_else(|| String::from("The song has no song line."))?;
    let mut lines: TextLines = TextLines::of(text);
    lines.lines.splice(after + 1..=after, block);
    Ok(lines.joined())
}

/// Removes a track: its block and its line in every section. The reader
/// refuses the result when the track was the only clock or the lead of
/// other voices, and says so.
///
/// # Errors
///
/// When the track does not exist.
pub fn delete_track(text: &str, source: &SongSource, track_name: &str) -> Result<String, String> {
    let track: &TrackSource = track_of(source, track_name)?;
    let mut lines: TextLines = TextLines::of(text);
    let mut doomed: Vec<usize> = (track.header_index..=track.last_index).collect();
    // The blank line that separated the block from the next, too.
    if lines
        .lines
        .get(track.last_index + 1)
        .is_some_and(|line: &String| line.trim().is_empty())
    {
        doomed.push(track.last_index + 1);
    }
    for section in &source.sections {
        for line in &section.lines {
            if line.track_name == track_name {
                doomed.extend(line.first_index..=line.last_index);
            }
        }
    }
    remove_indices(&mut lines, &doomed);
    Ok(lines.joined())
}

/// The studio with the track's output renamed or removed.
///
/// # Errors
///
/// When the changed studio is not valid (the studio configuration's own
/// check).
pub fn studio_after(
    studio: &StudioConfiguration,
    change: &StudioChange,
) -> Result<StudioConfiguration, String> {
    let tracks: Vec<TrackOutput> = match change {
        StudioChange::Rename { old_name, new_name } => studio
            .tracks()
            .iter()
            .cloned()
            .map(|mut output: TrackOutput| {
                if output.track_name == *old_name {
                    output.track_name.clone_from(new_name);
                }
                output
            })
            .collect(),
        StudioChange::Remove { track_name } => studio
            .tracks()
            .iter()
            .filter(|output: &&TrackOutput| output.track_name != *track_name)
            .cloned()
            .collect(),
    };
    StudioConfiguration::new(
        studio.name(),
        studio.port_names().to_vec(),
        tracks,
        studio.destinations().to_vec(),
    )
    .map_err(|error| error.to_string())
}

/// Whether the song uses a track: a line for it in some section, or a cue
/// that sends to it. Changing the track's type or deleting it would have
/// to decide what becomes of that music, which this dialog does not do.
#[must_use]
pub fn track_in_use(text: &str, source: &SongSource, track_name: &str) -> bool {
    let sent_to: String = format!("destination track \"{track_name}\"");
    source.sections.iter().any(|section| {
        section
            .lines
            .iter()
            .any(|line| line.track_name == track_name)
    }) || text
        .lines()
        .any(|line| line.trim_start().starts_with("cue ") && line.contains(&sent_to))
}

/// The one reason a track in use cannot be changed or removed.
fn in_use_reason(track_name: &str, operation: &str) -> String {
    format!("Remove the song's use of {track_name}, then retry {operation}.")
}

/// Every track type, each with whether this track can change to it: the
/// current type is allowed (it is where the track is); another type is
/// refused when the song uses the track, or when the song would not read
/// with it. A refusal is one sentence at the level of the settings.
#[must_use]
pub fn type_options(session: &Session, track_index: usize) -> Vec<(TrackType, Result<(), String>)> {
    let document = session.document();
    let Some(track) = document.song().tracks().get(track_index) else {
        return Vec::new();
    };
    let name: &str = track.name();
    let current: TrackType = TrackType::of(track);
    let in_use: bool = track_in_use(document.text(), document.source(), name);
    TrackType::ALL
        .iter()
        .map(|candidate: &TrackType| {
            let allowed: Result<(), String> = if *candidate == current {
                Ok(())
            } else if in_use {
                Err(in_use_reason(name, "changing its type"))
            } else {
                match set_type(document.text(), document.source(), name, *candidate) {
                    Ok(text)
                        if crate::song_document::read_song(document.file_name(), &text).is_ok() =>
                    {
                        Ok(())
                    }
                    // The reader's own reasons describe song content, which
                    // this dialog does not go into.
                    _ => Err(format!("The song needs {name} as {}.", current.word())),
                }
            };
            (*candidate, allowed)
        })
        .collect()
}

/// `type_options` for every track, in order.
#[must_use]
pub fn all_type_options(session: &Session) -> Vec<Vec<(TrackType, Result<(), String>)>> {
    (0..session.document().song().tracks().len())
        .map(|track_index: usize| type_options(session, track_index))
        .collect()
}

/// Moves a track's block one place up or down among the tracks, keeping
/// whatever sits between the two blocks (blank lines, comments) where it is.
///
/// # Errors
///
/// When the track does not exist or is already at that end.
pub fn move_track(
    text: &str,
    source: &SongSource,
    track_name: &str,
    up: bool,
) -> Result<String, String> {
    let at: usize = source
        .tracks
        .iter()
        .position(|track: &TrackSource| track.name == track_name)
        .ok_or_else(|| format!("The song has no track \"{track_name}\"."))?;
    let other: usize = if up {
        at.checked_sub(1)
            .ok_or_else(|| format!("{track_name} is already the first track."))?
    } else if at + 1 < source.tracks.len() {
        at + 1
    } else {
        return Err(format!("{track_name} is already the last track."));
    };
    let (first, second): (&TrackSource, &TrackSource) = if up {
        (&source.tracks[other], &source.tracks[at])
    } else {
        (&source.tracks[at], &source.tracks[other])
    };
    let lines: TextLines = TextLines::of(text);
    let block = |track: &TrackSource| lines.lines[track.header_index..=track.last_index].to_vec();
    let gap: Vec<String> = lines.lines[first.last_index + 1..second.header_index].to_vec();
    let mut moved: Vec<String> = lines.lines[..first.header_index].to_vec();
    moved.extend(block(second));
    moved.extend(gap);
    moved.extend(block(first));
    moved.extend(lines.lines[second.last_index + 1..].iter().cloned());
    let mut result: TextLines = TextLines::of(text);
    result.lines = moved;
    Ok(result.joined())
}

/// The number at the start of a `voice_layer 2 "Inner"` line's body.
fn layer_number_of(line: &str) -> Option<u8> {
    line.trim()
        .strip_prefix("voice_layer")?
        .split_whitespace()
        .next()?
        .parse::<u8>()
        .ok()
}

/// Names voice layer `number` of a track (`voice_layer 2 "Inner"`), or
/// clears its name when `name` is `None`. A name already given is replaced
/// in place; a new one goes at the end of the track's block.
///
/// # Errors
///
/// When the track does not exist, or the name has a double quote.
pub fn set_voice_layer_name(
    text: &str,
    source: &SongSource,
    track_name: &str,
    number: u8,
    name: Option<&str>,
) -> Result<String, String> {
    if let Some(name) = name {
        check_name("voice layer", name)?;
    }
    let track: &TrackSource = track_of(source, track_name)?;
    let mut lines: TextLines = TextLines::of(text);
    let existing: Option<usize> = track.voice_layer_indices.iter().copied().find(|index| {
        lines
            .lines
            .get(*index)
            .is_some_and(|line: &String| layer_number_of(line) == Some(number))
    });
    match (existing, name) {
        (Some(index), Some(name)) => {
            replace_body(
                &mut lines,
                index,
                &format!("voice_layer {number} \"{name}\""),
            )?;
        }
        (Some(index), None) => remove_indices(&mut lines, &[index]),
        (None, Some(name)) => {
            let indent: String = lines
                .lines
                .get(track.header_index)
                .map(|line: &String| format!("{}  ", indentation(line)))
                .unwrap_or_default();
            lines.lines.insert(
                track.last_index + 1,
                format!("{indent}voice_layer {number} \"{name}\""),
            );
        }
        (None, None) => {}
    }
    Ok(lines.joined())
}

/// Carries out a settings command on the song. The result is what the
/// studio must follow, when the song accepted a rename or a removal; the
/// session says whether it was done or why not, in a sentence.
pub fn apply(session: &mut Session, command: SettingsCommand) -> Option<StudioChange> {
    match command {
        SettingsCommand::SetTitle(title) => {
            session.edit_text(
                "Song title changed",
                "Cannot change the title",
                |text, source, _| set_title(text, source, &title),
            );
            None
        }
        SettingsCommand::CycleNumerals => {
            let next: Option<NumeralStandard> = match session.document().song().numeral_standard() {
                None => Some(NumeralStandard::KeyDependent),
                Some(NumeralStandard::KeyDependent) => Some(NumeralStandard::MajorReferential),
                Some(NumeralStandard::MajorReferential) => None,
            };
            session.edit_text(
                "Chord numeral standard changed",
                "Cannot change the chord numeral standard",
                |text, source, _| set_numeral_standard(text, source, next),
            );
            None
        }
        SettingsCommand::RenameTrack {
            track_index,
            new_name,
        } => {
            let old_name: String = track_name_at(session, track_index)?;
            if new_name == old_name {
                return None;
            }
            let done: bool = session.edit_text(
                &format!("Track {old_name} renamed to {new_name}"),
                &format!("Cannot rename track {old_name}"),
                |text, source, _| rename_track(text, source, &old_name, &new_name),
            );
            done.then_some(StudioChange::Rename { old_name, new_name })
        }
        SettingsCommand::SetType {
            track_index,
            new_type,
        } => {
            let name: String = track_name_at(session, track_index)?;
            session.edit_text(
                &format!("Track {name} is now {}", new_type.word()),
                &format!("Cannot change the type of {name}"),
                |text, source, _| set_type(text, source, &name, new_type),
            );
            None
        }
        SettingsCommand::SetClef { track_index, clef } => {
            change_clef(session, track_index, clef)?;
            None
        }
        SettingsCommand::MoveTrack { track_index, up } => {
            move_track_command(session, track_index, up)?;
            None
        }
        SettingsCommand::SetVoiceLayerName {
            track_index,
            number,
            name,
        } => {
            name_layer(session, track_index, number, name.as_deref())?;
            None
        }
        SettingsCommand::MakeLead { track_index } => {
            let name: String = track_name_at(session, track_index)?;
            session.edit_text(
                &format!("Track {name} is now the lead"),
                &format!("Cannot make {name} the lead"),
                |text, source, _| make_lead(text, source, &name),
            );
            None
        }
        SettingsCommand::AddTrack { name } => {
            session.edit_text(
                &format!("Track {name} added"),
                &format!("Cannot add track {name}"),
                |text, source, song| add_track(text, source, song, &name, TrackType::PolyVoice),
            );
            None
        }
        SettingsCommand::DeleteTrack { track_index } => {
            let track_name: String = track_name_at(session, track_index)?;
            if let Some(reason) = delete_refusal(session, track_index) {
                session.say(crate::session::MessageKind::Problem, reason);
                return None;
            }
            let done: bool = session.edit_text(
                &format!("Track {track_name} deleted"),
                &format!("Cannot delete track {track_name}"),
                |text, source, _| delete_track(text, source, &track_name),
            );
            done.then_some(StudioChange::Remove { track_name })
        }
        // The studio is the workspace's, not the song's.
        SettingsCommand::Studio(_) | SettingsCommand::CreateStudio => None,
    }
}

/// Changes a voice's clef.
fn change_clef(session: &mut Session, track_index: usize, clef: Clef) -> Option<()> {
    let name: String = track_name_at(session, track_index)?;
    session.edit_text(
        &format!("Track {name} now has the {} clef", clef_word(clef)),
        &format!("Cannot change the clef of {name}"),
        |text, source, _| set_clef(text, source, &name, clef),
    );
    Some(())
}

/// Moves a track one place up or down.
fn move_track_command(session: &mut Session, track_index: usize, up: bool) -> Option<()> {
    let name: String = track_name_at(session, track_index)?;
    session.edit_text(
        &format!("Track {name} moved {}", if up { "up" } else { "down" }),
        &format!("Cannot move {name}"),
        |text, source, _| move_track(text, source, &name, up),
    );
    Some(())
}

/// Names (or clears the name of) one voice layer of a track.
fn name_layer(
    session: &mut Session,
    track_index: usize,
    number: u8,
    name: Option<&str>,
) -> Option<()> {
    let track_name: String = track_name_at(session, track_index)?;
    let done: String = match name {
        Some(name) => format!("Layer {number} of {track_name} is named {name}"),
        None => format!("Layer {number} of {track_name} has no name now"),
    };
    session.edit_text(
        &done,
        &format!("Cannot name layer {number} of {track_name}"),
        |text, source, _| set_voice_layer_name(text, source, &track_name, number, name),
    );
    Some(())
}

/// The one reason a track cannot be deleted, before the reader is asked.
fn delete_refusal(session: &Session, track_index: usize) -> Option<String> {
    let document = session.document();
    let tracks: &[Track] = document.song().tracks();
    let track: &Track = tracks.get(track_index)?;
    if track_in_use(document.text(), document.source(), track.name()) {
        return Some(in_use_reason(track.name(), "the delete"));
    }
    match track.kind() {
        TrackKind::ClockAndTransport => {
            Some(String::from("The song needs a clock and transport track."))
        }
        TrackKind::Voice { lead: true, .. } => {
            let other_voices: bool = tracks.iter().enumerate().any(|(index, other)| {
                index != track_index && matches!(other.kind(), TrackKind::Voice { .. })
            });
            other_voices.then(|| {
                format!(
                    "{} is the lead. Make another voice the lead, then retry the delete.",
                    track.name()
                )
            })
        }
        _ => None,
    }
}

fn track_name_at(session: &Session, track_index: usize) -> Option<String> {
    session
        .document()
        .song()
        .tracks()
        .get(track_index)
        .map(|track: &Track| track.name().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song_document::read_song;
    use vocabulary::song::Song;

    const BASE: &str = include_str!("../../../songs_canonical/song_settings/settings_base.song");

    fn source() -> SongSource {
        SongSource::read(BASE)
    }

    fn reads(text: &str) -> Song {
        match read_song("t.song", text) {
            Ok(song) => song,
            Err(messages) => panic!("{}\n\n{text}", messages.join("\n")),
        }
    }

    fn refused(text: &str) -> String {
        read_song("t.song", text).unwrap_err().join(" ")
    }

    #[test]
    fn the_outline_finds_each_track_block() {
        let source: SongSource = source();
        let names: Vec<&str> = source.tracks.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["Clock", "Beat", "Keys", "Bass", "Cues"]);
        let keys: &TrackSource = source.track("Keys").unwrap();
        assert!(keys.lead_index.is_some() && keys.clef_index.is_some());
        assert!(source.track("Beat").unwrap().lead_index.is_none());
    }

    #[test]
    fn the_title_is_replaced() {
        let text: String = set_title(BASE, &source(), "New name").unwrap();
        assert_eq!(reads(&text).name(), "New name");
        assert!(set_title(BASE, &source(), "Say \"hi\"").is_err());
        assert!(set_title(BASE, &source(), "  ").is_err());
    }

    #[test]
    fn the_numeral_standard_is_added_changed_and_removed() {
        let with: String =
            set_numeral_standard(BASE, &source(), Some(NumeralStandard::KeyDependent)).unwrap();
        assert_eq!(
            reads(&with).numeral_standard(),
            Some(NumeralStandard::KeyDependent)
        );
        let changed: String = set_numeral_standard(
            &with,
            &SongSource::read(&with),
            Some(NumeralStandard::MajorReferential),
        )
        .unwrap();
        assert_eq!(
            reads(&changed).numeral_standard(),
            Some(NumeralStandard::MajorReferential)
        );
        let without: String =
            set_numeral_standard(&changed, &SongSource::read(&changed), None).unwrap();
        assert_eq!(reads(&without).numeral_standard(), None);
        assert_eq!(without, BASE);
    }

    #[test]
    fn renaming_a_track_follows_its_lines_and_the_cues_that_send_to_it() {
        let text: String = rename_track(BASE, &source(), "Keys", "Piano").unwrap();
        let song: Song = reads(&text);
        assert!(song.tracks().iter().any(|t| t.name() == "Piano"));
        assert!(!text.contains("\"Keys\""), "{text}");
        assert!(text.contains("destination track \"Piano\""));
        assert!(
            song.sections()
                .iter()
                .all(|s| s.lines().iter().any(|l| l.track_name() == "Piano"))
        );
    }

    #[test]
    fn a_name_that_is_taken_or_unwritable_is_refused() {
        assert!(rename_track(BASE, &source(), "Keys", "Bass").is_err());
        assert!(rename_track(BASE, &source(), "Keys", "A\"B").is_err());
        assert!(rename_track(BASE, &source(), "Nope", "X").is_err());
    }

    #[test]
    fn a_type_change_keeps_a_voice_a_voice_and_strips_what_a_cue_track_cannot_have() {
        let mono: String = set_type(BASE, &source(), "Keys", TrackType::MonoVoice).unwrap();
        assert!(mono.contains("type mono_voice"));
        // Keys holds chords, which a mono voice refuses; the refusal is the
        // reader's, and it names the track.
        assert!(refused(&mono).contains("\"Keys\""), "{}", refused(&mono));
        let beat_to_poly: String = set_type(BASE, &source(), "Beat", TrackType::PolyVoice).unwrap();
        reads(&beat_to_poly);
        let beat_to_clock: String =
            set_type(BASE, &source(), "Beat", TrackType::ClockAndTransport).unwrap();
        assert!(
            refused(&beat_to_clock).contains("clock"),
            "{}",
            refused(&beat_to_clock)
        );
        let bass_to_meter: String =
            set_type(BASE, &source(), "Bass", TrackType::MeterControl).unwrap();
        assert!(!bass_to_meter.contains("clef bass"));
    }

    #[test]
    fn a_clef_is_replaced_and_only_voices_have_one() {
        let text: String = set_clef(BASE, &source(), "Bass", Clef::Tenor).unwrap();
        reads(&text);
        assert!(text.contains("clef tenor"));
        assert!(set_clef(BASE, &source(), "Cues", Clef::Bass).is_err());
    }

    #[test]
    fn the_lead_moves_to_the_chosen_voice() {
        let text: String = make_lead(BASE, &source(), "Beat").unwrap();
        let song: Song = reads(&text);
        let leads: Vec<&str> = song
            .tracks()
            .iter()
            .filter(|t| matches!(t.kind(), TrackKind::Voice { lead: true, .. }))
            .map(Track::name)
            .collect();
        assert_eq!(leads, ["Beat"]);
        assert!(make_lead(BASE, &source(), "Clock").is_err());
        let again: String = make_lead(&text, &SongSource::read(&text), "Bass").unwrap();
        reads(&again);
    }

    #[test]
    fn a_track_is_added_after_the_last_and_the_first_voice_is_the_lead() {
        let song: Song = reads(BASE);
        let text: String = add_track(BASE, &source(), &song, "Pad", TrackType::PolyVoice).unwrap();
        let added: Song = reads(&text);
        assert_eq!(added.tracks().last().map(Track::name), Some("Pad"));
        assert!(add_track(BASE, &source(), &song, "Keys", TrackType::PolyVoice).is_err());
        let cue_only: String =
            add_track(BASE, &source(), &song, "Meter", TrackType::MeterControl).unwrap();
        reads(&cue_only);
    }

    #[test]
    fn a_track_is_deleted_with_its_lines() {
        let text: String = delete_track(BASE, &source(), "Bass").unwrap();
        let song: Song = reads(&text);
        assert!(song.tracks().iter().all(|t| t.name() != "Bass"));
        assert!(!text.contains("\"Bass\""), "{text}");
        let lead_gone: String = delete_track(BASE, &source(), "Keys").unwrap();
        assert!(
            refused(&lead_gone).contains("lead"),
            "{}",
            refused(&lead_gone)
        );
    }

    #[test]
    fn the_studio_follows_a_rename_and_a_removal() {
        use crate::song_settings::studio_after;
        use song_text::studio_file::parse_studio;
        let studio: StudioConfiguration = parse_studio(
            "s.studio",
            include_str!("../../../songs_canonical/song_settings/settings_base.studio"),
        )
        .unwrap();
        let renamed: StudioConfiguration = studio_after(
            &studio,
            &StudioChange::Rename {
                old_name: String::from("Keys"),
                new_name: String::from("Piano"),
            },
        )
        .unwrap();
        assert!(renamed.output_of("Piano").is_some() && renamed.output_of("Keys").is_none());
        let removed: StudioConfiguration = studio_after(
            &studio,
            &StudioChange::Remove {
                track_name: String::from("Bass"),
            },
        )
        .unwrap();
        assert!(removed.output_of("Bass").is_none());
        assert!(removed.output_of("Keys").is_some());
    }

    #[test]
    fn a_rename_through_the_session_asks_the_studio_to_follow_and_a_refusal_does_not() {
        use crate::library::Libraries;
        use crate::song_document::SongDocument;
        let libraries: Libraries = Libraries::read(
            "p.library",
            include_str!("../../../library/Progressions.library"),
            "r.library",
            include_str!("../../../library/Rhythms.library"),
        )
        .unwrap();
        let document: SongDocument = SongDocument::new("t.song", BASE.to_string()).unwrap();
        let mut session: Session = Session::new(document, libraries);
        let change: Option<StudioChange> = apply(
            &mut session,
            SettingsCommand::RenameTrack {
                track_index: 2,
                new_name: String::from("Piano"),
            },
        );
        assert_eq!(
            change,
            Some(StudioChange::Rename {
                old_name: String::from("Keys"),
                new_name: String::from("Piano")
            })
        );
        assert!(session.document().text().contains("track \"Piano\""));
        // The lead cannot be deleted while other voices remain.
        let refused: Option<StudioChange> = apply(
            &mut session,
            SettingsCommand::DeleteTrack { track_index: 2 },
        );
        assert_eq!(refused, None);
        assert!(session.document().text().contains("track \"Piano\""));
    }

    #[test]
    fn t_steps_only_to_types_the_song_still_reads_with() {
        use crate::library::Libraries;
        use crate::song_document::SongDocument;
        let libraries: Libraries = Libraries::read(
            "p.library",
            include_str!("../../../library/Progressions.library"),
            "r.library",
            include_str!("../../../library/Rhythms.library"),
        )
        .unwrap();
        let document: SongDocument = SongDocument::new("t.song", BASE.to_string()).unwrap();
        let session: Session = Session::new(document, libraries);
        let options = all_type_options(&session);
        let refusal = |track: usize, to: TrackType| -> String {
            options[track]
                .iter()
                .find(|(candidate, _)| *candidate == to)
                .and_then(|(_, allowed)| allowed.clone().err())
                .unwrap()
        };
        // Clock, Keys and Beat are all used by the song (or needed by
        // it), so none can change type, and the reason is one sentence.
        assert!(refusal(0, TrackType::MonoVoice).contains("needs Clock"));
        assert_eq!(
            refusal(1, TrackType::CueControl),
            "Remove the song's use of Beat, then retry changing its type."
        );
        assert!(!refusal(2, TrackType::MonoVoice).contains("line"));
        // A track's own type is always allowed.
        assert!(options[0][2].1.is_ok());
    }

    #[test]
    fn a_track_the_song_uses_cannot_be_deleted_and_the_reason_is_one_sentence() {
        use crate::library::Libraries;
        use crate::song_document::SongDocument;
        let libraries: Libraries = Libraries::read(
            "p.library",
            include_str!("../../../library/Progressions.library"),
            "r.library",
            include_str!("../../../library/Rhythms.library"),
        )
        .unwrap();
        let document: SongDocument = SongDocument::new("t.song", BASE.to_string()).unwrap();
        let mut session: Session = Session::new(document, libraries);
        let change: Option<StudioChange> = apply(
            &mut session,
            SettingsCommand::DeleteTrack { track_index: 3 },
        );
        assert_eq!(change, None);
        assert_eq!(
            session.message().unwrap().text,
            "Remove the song's use of Bass, then retry the delete."
        );
        let clock: Option<StudioChange> = apply(
            &mut session,
            SettingsCommand::DeleteTrack { track_index: 0 },
        );
        assert_eq!(clock, None);
        assert!(session.document().text().contains("track \"Clock\""));
    }
}
