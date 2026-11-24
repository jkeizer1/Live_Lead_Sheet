//! The studio configuration's settings as edits: the studio's name, its
//! ports, each track's output (port, channel, velocities, octave offset,
//! retrigger gap, delay trigger) and the cue destinations. Each edit gives a
//! new configuration, checked by the configuration's own rules, so the
//! studio never holds a port a track names and does not declare.
//!
//! When an edit cannot be done the one reason is a sentence at the level of
//! the settings ("Remove the studio's use of the port, then retry the
//! delete."), never detail about the song's music.

use player::studio_configuration::{
    DestinationOutput, StudioConfiguration, TrackOutput, UnmetNeed, VelocityLevels,
};
use vocabulary::duration::Duration;
use vocabulary::midi_values::{MidiChannel, Velocity};

use vocabulary::song::{CueDestination, Song, TrackKind};

use crate::song_settings::TrackType;

/// Which of a track's three velocities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VelocityKind {
    /// What a note with no mark sends (mezzo forte).
    Normal,
    Accent,
    Marcato,
}

/// A change to the studio configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudioEdit {
    /// Start a studio for a song that has none (done by the app, which
    /// knows the file).
    Create,
    SetName(String),
    AddPort(String),
    RenamePort {
        port_index: usize,
        new_name: String,
    },
    DeletePort {
        port_index: usize,
    },
    /// Give a track an output, on the first port, with no channel yet.
    AddOutput {
        track_name: String,
    },
    RemoveOutput {
        track_name: String,
    },
    SetTrackPort {
        track_name: String,
        port_index: usize,
    },
    SetTrackChannel {
        track_name: String,
        channel_number: Option<u8>,
    },
    SetVelocity {
        track_name: String,
        kind: VelocityKind,
        value: Option<u8>,
    },
    SetOctaveOffset {
        track_name: String,
        octaves: i8,
    },
    SetRetriggerGap {
        track_name: String,
        milliseconds: u64,
    },
    SetDelayTrigger {
        track_name: String,
        milliseconds: u64,
    },
    AddDestination {
        name: String,
    },
    RenameDestination {
        destination_index: usize,
        new_name: String,
    },
    SetDestinationPort {
        destination_index: usize,
        port_index: usize,
    },
    SetDestinationChannel {
        destination_index: usize,
        channel_number: Option<u8>,
    },
    DeleteDestination {
        destination_index: usize,
    },
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

fn channel_of(channel_number: Option<u8>) -> Result<Option<MidiChannel>, String> {
    channel_number
        .map(|number: u8| {
            MidiChannel::new(number).map_err(|_| String::from("A channel is 1 to 16."))
        })
        .transpose()
}

fn port_name_at(studio: &StudioConfiguration, port_index: usize) -> Result<String, String> {
    studio
        .port_names()
        .get(port_index)
        .cloned()
        .ok_or_else(|| String::from("There is no such port."))
}

fn rebuilt(
    studio: &StudioConfiguration,
    name: &str,
    ports: Vec<String>,
    tracks: Vec<TrackOutput>,
    destinations: Vec<DestinationOutput>,
) -> Result<StudioConfiguration, String> {
    StudioConfiguration::new(name, ports, tracks, destinations).map_err(|_| {
        // The configuration's own check names the cause; at this level the
        // one reason is that the studio would not hold together.
        let _ = studio;
        String::from("The studio would not hold together after that change.")
    })
}

/// The studio after one edit. `used_destinations` are the destination names
/// the song's cues send to: such a destination cannot be renamed or
/// deleted here.
///
/// # Errors
///
/// The one reason the edit cannot be done, in a sentence.
#[allow(clippy::too_many_lines)] // one arm per edit, each short
pub fn apply_studio_edit(
    studio: &StudioConfiguration,
    edit: &StudioEdit,
    used_destinations: &[String],
) -> Result<StudioConfiguration, String> {
    let mut ports: Vec<String> = studio.port_names().to_vec();
    let mut tracks: Vec<TrackOutput> = studio.tracks().to_vec();
    let mut destinations: Vec<DestinationOutput> = studio.destinations().to_vec();
    let mut name: String = studio.name().to_string();
    match edit {
        StudioEdit::Create => return Err(String::from("The studio already exists.")),
        StudioEdit::SetName(new_name) => {
            check_name("studio", new_name)?;
            name.clone_from(new_name);
        }
        StudioEdit::AddPort(new_name) => {
            check_name("port", new_name)?;
            if ports.contains(new_name) {
                return Err(format!("The studio already has a port \"{new_name}\"."));
            }
            ports.push(new_name.clone());
        }
        StudioEdit::RenamePort {
            port_index,
            new_name,
        } => {
            check_name("port", new_name)?;
            let old_name: String = port_name_at(studio, *port_index)?;
            if *new_name != old_name && ports.contains(new_name) {
                return Err(format!("The studio already has a port \"{new_name}\"."));
            }
            for port in &mut ports {
                if *port == old_name {
                    port.clone_from(new_name);
                }
            }
            for track in &mut tracks {
                if track.port_name == old_name {
                    track.port_name.clone_from(new_name);
                }
            }
            for destination in &mut destinations {
                if destination.port_name == old_name {
                    destination.port_name.clone_from(new_name);
                }
            }
        }
        StudioEdit::DeletePort { port_index } => {
            let old_name: String = port_name_at(studio, *port_index)?;
            let used: bool = tracks.iter().any(|track| track.port_name == old_name)
                || destinations.iter().any(|d| d.port_name == old_name);
            if used {
                return Err(String::from(
                    "Remove the studio's use of the port, then retry the delete.",
                ));
            }
            ports.remove(*port_index);
        }
        StudioEdit::AddOutput { track_name } => {
            let Some(first_port) = ports.first() else {
                return Err(String::from("Add a port first."));
            };
            if tracks.iter().any(|track| track.track_name == *track_name) {
                return Err(format!("{track_name} already has an output."));
            }
            tracks.push(TrackOutput {
                track_name: track_name.clone(),
                port_name: first_port.clone(),
                channel: None,
                velocity: VelocityLevels::new(Velocity::MEZZO_FORTE, None, None),
                octave_offset_octaves: 0,
                retrigger_gap: Duration::ZERO,
                delay_trigger: Duration::ZERO,
            });
        }
        StudioEdit::RemoveOutput { track_name } => {
            tracks.retain(|track| track.track_name != *track_name);
        }
        StudioEdit::SetTrackPort {
            track_name,
            port_index,
        } => {
            let port_name: String = port_name_at(studio, *port_index)?;
            output_mut(&mut tracks, track_name)?.port_name = port_name;
        }
        StudioEdit::SetTrackChannel {
            track_name,
            channel_number,
        } => {
            output_mut(&mut tracks, track_name)?.channel = channel_of(*channel_number)?;
        }
        StudioEdit::SetVelocity {
            track_name,
            kind,
            value,
        } => {
            let output: &mut TrackOutput = output_mut(&mut tracks, track_name)?;
            let velocity_of = |number: Option<u8>| {
                number
                    .map(|number: u8| {
                        Velocity::new(number).map_err(|_| String::from("A velocity is 1 to 127."))
                    })
                    .transpose()
            };
            let levels: VelocityLevels = output.velocity;
            let chosen: Option<Velocity> = velocity_of(*value)?;
            output.velocity = match kind {
                VelocityKind::Normal => {
                    let Some(normal) = chosen else {
                        return Err(String::from(
                            "The normal velocity needs a number, 1 to 127.",
                        ));
                    };
                    VelocityLevels::new(normal, levels.accent(), levels.marcato())
                }
                VelocityKind::Accent => {
                    VelocityLevels::new(levels.normal(), chosen, levels.marcato())
                }
                VelocityKind::Marcato => {
                    VelocityLevels::new(levels.normal(), levels.accent(), chosen)
                }
            };
        }
        StudioEdit::SetOctaveOffset {
            track_name,
            octaves,
        } => {
            if !(-8..=8).contains(octaves) {
                return Err(String::from("An octave offset is -8 to 8."));
            }
            output_mut(&mut tracks, track_name)?.octave_offset_octaves = *octaves;
        }
        StudioEdit::SetRetriggerGap {
            track_name,
            milliseconds,
        } => {
            output_mut(&mut tracks, track_name)?.retrigger_gap = Duration::from_ms(*milliseconds);
        }
        StudioEdit::SetDelayTrigger {
            track_name,
            milliseconds,
        } => {
            output_mut(&mut tracks, track_name)?.delay_trigger = Duration::from_ms(*milliseconds);
        }
        StudioEdit::AddDestination { name: new_name } => {
            check_name("destination", new_name)?;
            let Some(first_port) = ports.first() else {
                return Err(String::from("Add a port first."));
            };
            if destinations.iter().any(|d| d.destination_name == *new_name) {
                return Err(format!(
                    "The studio already has a destination \"{new_name}\"."
                ));
            }
            destinations.push(DestinationOutput {
                destination_name: new_name.clone(),
                port_name: first_port.clone(),
                channel: None,
            });
        }
        StudioEdit::RenameDestination {
            destination_index,
            new_name,
        } => {
            check_name("destination", new_name)?;
            let destination: &mut DestinationOutput =
                destination_mut(&mut destinations, *destination_index)?;
            if used_destinations.contains(&destination.destination_name) {
                return Err(String::from(
                    "Remove the song's use of the destination, then retry the rename.",
                ));
            }
            destination.destination_name.clone_from(new_name);
        }
        StudioEdit::SetDestinationPort {
            destination_index,
            port_index,
        } => {
            let port_name: String = port_name_at(studio, *port_index)?;
            destination_mut(&mut destinations, *destination_index)?.port_name = port_name;
        }
        StudioEdit::SetDestinationChannel {
            destination_index,
            channel_number,
        } => {
            destination_mut(&mut destinations, *destination_index)?.channel =
                channel_of(*channel_number)?;
        }
        StudioEdit::DeleteDestination { destination_index } => {
            let destination: &DestinationOutput = destinations
                .get(*destination_index)
                .ok_or_else(|| String::from("There is no such destination."))?;
            if used_destinations.contains(&destination.destination_name) {
                return Err(String::from(
                    "Remove the song's use of the destination, then retry the delete.",
                ));
            }
            destinations.remove(*destination_index);
        }
    }
    rebuilt(studio, &name, ports, tracks, destinations)
}

fn output_mut<'a>(
    tracks: &'a mut [TrackOutput],
    track_name: &str,
) -> Result<&'a mut TrackOutput, String> {
    tracks
        .iter_mut()
        .find(|track| track.track_name == track_name)
        .ok_or_else(|| format!("{track_name} has no output in the studio."))
}

fn destination_mut(
    destinations: &mut [DestinationOutput],
    destination_index: usize,
) -> Result<&mut DestinationOutput, String> {
    destinations
        .get_mut(destination_index)
        .ok_or_else(|| String::from("There is no such destination."))
}

/// What the song needs that the studio does not give, as the settings
/// dialog shows it: the tracks that cannot play yet, and lines about cue
/// destinations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StudioNeeds {
    /// Tracks with no output, or an output with no channel.
    pub tracks: Vec<String>,
    /// One line for each cue destination the song uses and the studio lacks
    /// or leaves without a channel.
    pub destination_lines: Vec<String>,
}

/// What `studio` leaves unmet for `song`.
#[must_use]
pub fn needs_of(studio: &StudioConfiguration, song: &Song) -> StudioNeeds {
    let mut needs: StudioNeeds = StudioNeeds::default();
    let add_line = |line: String, lines: &mut Vec<String>| {
        if !lines.contains(&line) {
            lines.push(line);
        }
    };
    for need in studio.unmet_needs(song) {
        match need {
            UnmetNeed::NoTrackBlock { track_name } | UnmetNeed::NoChannel { track_name } => {
                if !needs.tracks.contains(&track_name) {
                    needs.tracks.push(track_name);
                }
            }
            UnmetNeed::NoDestination {
                destination_name, ..
            } => add_line(
                format!("Cues send to \"{destination_name}\": add that destination."),
                &mut needs.destination_lines,
            ),
            UnmetNeed::NoDestinationChannel { destination_name } => add_line(
                format!("Destination \"{destination_name}\" needs a channel."),
                &mut needs.destination_lines,
            ),
            UnmetNeed::CueToUnknownVoice { track_name, .. } => add_line(
                format!("A cue sends to \"{track_name}\", which is not a voice of the song."),
                &mut needs.destination_lines,
            ),
        }
    }
    needs
}

/// The destination names the song's cues send to.
#[must_use]
pub fn used_destinations(song: &Song) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for track in song.tracks() {
        if let TrackKind::CueControl { cues } = track.kind() {
            for cue in cues {
                if let CueDestination::Named(name) = cue.destination()
                    && !names.contains(name)
                {
                    names.push(name.clone());
                }
            }
        }
    }
    names
}

/// A track output setting, as a row of the studio pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputField {
    Port,
    Channel,
    Velocity(VelocityKind),
    OctaveOffset,
    RetriggerGap,
    DelayTrigger,
}

/// What a row of the studio pane is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioRow {
    /// A section's title, such as "Ports" or "Track destination: Keys";
    /// it cannot be chosen.
    Heading,
    /// A line of explanation that cannot be chosen.
    Note,
    /// A line saying what the song needs and the studio lacks; it cannot be
    /// chosen.
    Warning,
    Name,
    Port(usize),
    AddPort,
    /// A setting of the chosen song track's output.
    Field(OutputField),
    AddOutput,
    DestinationName(usize),
    DestinationPort(usize),
    DestinationChannel(usize),
    AddDestination,
    /// There is no studio: start one.
    Create,
}

impl StudioRow {
    #[must_use]
    pub const fn selectable(self) -> bool {
        !matches!(
            self,
            StudioRow::Heading | StudioRow::Note | StudioRow::Warning
        )
    }
}

/// A row with what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioRowView {
    pub row: StudioRow,
    pub label: String,
    pub value: String,
}

fn row(row: StudioRow, label: &str, value: &str) -> StudioRowView {
    StudioRowView {
        row,
        label: label.to_string(),
        value: value.to_string(),
    }
}

fn channel_text(channel: Option<MidiChannel>) -> String {
    channel.map_or_else(
        || String::from("none"),
        |channel: MidiChannel| channel.number().to_string(),
    )
}

/// The rows of the studio pane: the studio's name, its ports, the settings
/// of the chosen song track's output, and the destinations.
#[must_use]
pub fn studio_rows(
    studio: Option<&StudioConfiguration>,
    track: Option<(&str, TrackType)>,
    needs: &StudioNeeds,
) -> Vec<StudioRowView> {
    let Some(studio) = studio else {
        return vec![
            row(StudioRow::Note, "No studio file for this song.", ""),
            row(StudioRow::Create, "Create a studio", "Enter"),
        ];
    };
    let mut rows: Vec<StudioRowView> = vec![row(StudioRow::Name, "Studio", studio.name())];
    rows.push(row(StudioRow::Heading, "Ports", ""));
    for (index, port) in studio.port_names().iter().enumerate() {
        rows.push(row(StudioRow::Port(index), port, ""));
    }
    rows.push(row(StudioRow::AddPort, "Add a port...", ""));
    if let Some((track_name, track_type)) = track {
        rows.push(row(
            StudioRow::Heading,
            &format!("Track destination: {track_name}"),
            "",
        ));
        rows.extend(output_rows(studio, track_name, track_type));
        if needs.tracks.iter().any(|name| name == track_name)
            && studio.output_of(track_name).is_some()
        {
            rows.insert(
                rows.len(),
                row(StudioRow::Warning, "Needs a channel to play.", ""),
            );
        }
    }
    rows.push(row(StudioRow::Heading, "Cue destinations", ""));
    for line in &needs.destination_lines {
        rows.push(row(StudioRow::Warning, line, ""));
    }
    for (index, destination) in studio.destinations().iter().enumerate() {
        rows.push(row(
            StudioRow::DestinationName(index),
            "Cue destination",
            &destination.destination_name,
        ));
        rows.push(row(
            StudioRow::DestinationPort(index),
            "   port",
            &destination.port_name,
        ));
        rows.push(row(
            StudioRow::DestinationChannel(index),
            "   channel",
            &channel_text(destination.channel),
        ));
    }
    rows.push(row(StudioRow::AddDestination, "Add a destination...", ""));
    rows
}

fn output_rows(
    studio: &StudioConfiguration,
    track_name: &str,
    track_type: TrackType,
) -> Vec<StudioRowView> {
    match track_type {
        TrackType::CueControl => {
            return vec![row(
                StudioRow::Note,
                "A cue track sends to the cue destinations below.",
                "",
            )];
        }
        TrackType::MeterControl => {
            return vec![row(StudioRow::Note, "A meter track sends no MIDI.", "")];
        }
        _ => {}
    }
    let Some(output) = studio.output_of(track_name) else {
        return vec![row(StudioRow::AddOutput, "No output. Add one", "Enter")];
    };
    let mut rows: Vec<StudioRowView> = vec![row(
        StudioRow::Field(OutputField::Port),
        "port",
        &output.port_name,
    )];
    if track_type == TrackType::ClockAndTransport {
        rows.push(row(
            StudioRow::Field(OutputField::DelayTrigger),
            "delay trigger",
            &format!("{} ms", output.delay_trigger.ns() / 1_000_000),
        ));
        return rows;
    }
    rows.push(row(
        StudioRow::Field(OutputField::Channel),
        "channel",
        &channel_text(output.channel),
    ));
    let levels: VelocityLevels = output.velocity;
    let velocity_text = |velocity: Option<Velocity>| {
        velocity.map_or_else(
            || String::from("default"),
            |v: Velocity| v.value().to_string(),
        )
    };
    rows.push(row(
        StudioRow::Field(OutputField::Velocity(VelocityKind::Normal)),
        "velocity",
        &levels.normal().value().to_string(),
    ));
    rows.push(row(
        StudioRow::Field(OutputField::Velocity(VelocityKind::Accent)),
        "accent velocity",
        &velocity_text(levels.accent()),
    ));
    rows.push(row(
        StudioRow::Field(OutputField::Velocity(VelocityKind::Marcato)),
        "marcato velocity",
        &velocity_text(levels.marcato()),
    ));
    rows.push(row(
        StudioRow::Field(OutputField::OctaveOffset),
        "octave offset",
        &output.octave_offset_octaves.to_string(),
    ));
    // ns ÷ (ns/ms) = ms
    rows.push(row(
        StudioRow::Field(OutputField::RetriggerGap),
        "retrigger gap",
        &format!("{} ms", output.retrigger_gap.ns() / 1_000_000),
    ));
    rows.push(row(
        StudioRow::Field(OutputField::DelayTrigger),
        "delay trigger",
        &format!("{} ms", output.delay_trigger.ns() / 1_000_000),
    ));
    rows
}

/// What to put in the text field when a row is edited by typing: the
/// value, or nothing for a value that is not set.
#[must_use]
pub fn typing_start(view: &StudioRowView) -> String {
    match view.value.as_str() {
        "none" | "default" => String::new(),
        value => value.strip_suffix(" ms").unwrap_or(value).to_string(),
    }
}

/// The edit that typing `text` into a setting of `track_name`'s output
/// asks for.
///
/// # Errors
///
/// The one reason the text is not a value for that setting.
pub fn edit_for_field(
    track_name: &str,
    field: OutputField,
    text: &str,
) -> Result<StudioEdit, String> {
    let text: &str = text.trim();
    let track_name: String = track_name.to_string();
    let optional_number = |what: &str, largest: &str| -> Result<Option<u8>, String> {
        if text.is_empty() {
            return Ok(None);
        }
        text.parse::<u8>()
            .map(Some)
            .map_err(|_| format!("A {what} is {largest}."))
    };
    match field {
        OutputField::Port => Err(String::from("A port is chosen, not typed.")),
        OutputField::Channel => Ok(StudioEdit::SetTrackChannel {
            track_name,
            channel_number: optional_number("channel", "1 to 16")?,
        }),
        OutputField::Velocity(kind) => Ok(StudioEdit::SetVelocity {
            track_name,
            kind,
            value: optional_number("velocity", "1 to 127")?,
        }),
        OutputField::OctaveOffset => text
            .parse::<i8>()
            .map(|octaves: i8| StudioEdit::SetOctaveOffset {
                track_name,
                octaves,
            })
            .map_err(|_| String::from("An octave offset is -8 to 8.")),
        OutputField::RetriggerGap => text
            .parse::<u64>()
            .map(|milliseconds: u64| StudioEdit::SetRetriggerGap {
                track_name,
                milliseconds,
            })
            .map_err(|_| String::from("A time is a whole number of milliseconds.")),
        OutputField::DelayTrigger => text
            .parse::<u64>()
            .map(|milliseconds: u64| StudioEdit::SetDelayTrigger {
                track_name,
                milliseconds,
            })
            .map_err(|_| String::from("A time is a whole number of milliseconds.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use song_text::studio_file::parse_studio;

    const BASE: &str = include_str!("../../../songs_canonical/song_settings/studio_base.studio");

    fn studio() -> StudioConfiguration {
        parse_studio("studio_base.studio", BASE).unwrap()
    }

    fn done(edit: &StudioEdit) -> StudioConfiguration {
        apply_studio_edit(&studio(), edit, &[]).unwrap()
    }

    fn refused(edit: &StudioEdit, used: &[String]) -> String {
        apply_studio_edit(&studio(), edit, used).unwrap_err()
    }

    #[test]
    fn ports_are_added_renamed_and_deleted_only_when_unused() {
        let added: StudioConfiguration = done(&StudioEdit::AddPort(String::from("Extra")));
        assert_eq!(added.port_names().len(), 4);
        let renamed: StudioConfiguration = done(&StudioEdit::RenamePort {
            port_index: 1,
            new_name: String::from("Keys out"),
        });
        assert_eq!(renamed.output_of("Keys").unwrap().port_name, "Keys out");
        assert_eq!(renamed.output_of("Bass").unwrap().port_name, "Keys out");
        let in_use: String = refused(&StudioEdit::DeletePort { port_index: 1 }, &[]);
        assert_eq!(
            in_use,
            "Remove the studio's use of the port, then retry the delete."
        );
        let unused: StudioConfiguration =
            apply_studio_edit(&added, &StudioEdit::DeletePort { port_index: 3 }, &[]).unwrap();
        assert_eq!(unused.port_names().len(), 3);
        assert!(refused(&StudioEdit::AddPort(String::from("Clock")), &[]).contains("already has"));
    }

    #[test]
    fn a_tracks_output_settings_are_each_set() {
        let keys = |edit: &StudioEdit| done(edit).output_of("Keys").unwrap().clone();
        assert_eq!(
            keys(&StudioEdit::SetTrackChannel {
                track_name: String::from("Keys"),
                channel_number: Some(9)
            })
            .channel
            .unwrap()
            .number(),
            9
        );
        let accent_gone = keys(&StudioEdit::SetVelocity {
            track_name: String::from("Keys"),
            kind: VelocityKind::Accent,
            value: None,
        });
        assert_eq!(accent_gone.velocity.accent(), None);
        assert_eq!(accent_gone.velocity.marcato().unwrap().value(), 120);
        assert_eq!(
            keys(&StudioEdit::SetOctaveOffset {
                track_name: String::from("Keys"),
                octaves: -2
            })
            .octave_offset_octaves,
            -2
        );
        assert_eq!(
            keys(&StudioEdit::SetRetriggerGap {
                track_name: String::from("Keys"),
                milliseconds: 7
            })
            .retrigger_gap,
            Duration::from_ms(7)
        );
        assert_eq!(
            keys(&StudioEdit::SetDelayTrigger {
                track_name: String::from("Keys"),
                milliseconds: 0
            })
            .delay_trigger,
            Duration::ZERO
        );
        let moved: StudioConfiguration = done(&StudioEdit::SetTrackPort {
            track_name: String::from("Keys"),
            port_index: 2,
        });
        assert_eq!(moved.output_of("Keys").unwrap().port_name, "To SQ-1");
    }

    #[test]
    fn values_out_of_range_are_refused_with_one_sentence() {
        let bad_channel: String = refused(
            &StudioEdit::SetTrackChannel {
                track_name: String::from("Keys"),
                channel_number: Some(17),
            },
            &[],
        );
        assert_eq!(bad_channel, "A channel is 1 to 16.");
        assert_eq!(
            refused(
                &StudioEdit::SetVelocity {
                    track_name: String::from("Keys"),
                    kind: VelocityKind::Normal,
                    value: Some(0)
                },
                &[]
            ),
            "A velocity is 1 to 127."
        );
        assert_eq!(
            refused(
                &StudioEdit::SetOctaveOffset {
                    track_name: String::from("Keys"),
                    octaves: 9
                },
                &[]
            ),
            "An octave offset is -8 to 8."
        );
    }

    #[test]
    fn an_output_is_added_on_the_first_port_with_no_channel_and_removed() {
        let without: StudioConfiguration = done(&StudioEdit::RemoveOutput {
            track_name: String::from("Bass"),
        });
        assert!(without.output_of("Bass").is_none());
        let with: StudioConfiguration = apply_studio_edit(
            &without,
            &StudioEdit::AddOutput {
                track_name: String::from("Bass"),
            },
            &[],
        )
        .unwrap();
        let output: &TrackOutput = with.output_of("Bass").unwrap();
        assert_eq!(output.port_name, "Clock");
        assert_eq!(output.channel, None);
    }

    #[test]
    fn destinations_follow_the_same_rules_and_a_used_one_is_kept() {
        let used: Vec<String> = vec![String::from("SQ-1")];
        assert_eq!(
            refused(
                &StudioEdit::DeleteDestination {
                    destination_index: 0
                },
                &used
            ),
            "Remove the song's use of the destination, then retry the delete."
        );
        assert_eq!(
            refused(
                &StudioEdit::RenameDestination {
                    destination_index: 0,
                    new_name: String::from("X")
                },
                &used
            ),
            "Remove the song's use of the destination, then retry the rename."
        );
        let gone: StudioConfiguration = apply_studio_edit(
            &studio(),
            &StudioEdit::DeleteDestination {
                destination_index: 1,
            },
            &used,
        )
        .unwrap();
        assert_eq!(gone.destinations().len(), 1);
        let added: StudioConfiguration = done(&StudioEdit::AddDestination {
            name: String::from("FH-2"),
        });
        assert_eq!(added.destination("FH-2").unwrap().port_name, "Clock");
        let channel: StudioConfiguration = done(&StudioEdit::SetDestinationChannel {
            destination_index: 1,
            channel_number: Some(3),
        });
        assert_eq!(channel.destinations()[1].channel.unwrap().number(), 3);
    }

    #[test]
    fn names_that_cannot_be_written_are_refused() {
        assert!(refused(&StudioEdit::SetName(String::from("A\"B")), &[]).contains("double quote"));
        assert!(refused(&StudioEdit::AddPort(String::new()), &[]).contains("at least one"));
    }

    #[test]
    fn the_rows_follow_the_chosen_track() {
        let studio: StudioConfiguration = studio();
        let keys: Vec<StudioRowView> = studio_rows(
            Some(&studio),
            Some(("Keys", TrackType::PolyVoice)),
            &StudioNeeds::default(),
        );
        let fields: usize = keys
            .iter()
            .filter(|r| matches!(r.row, StudioRow::Field(_)))
            .count();
        assert_eq!(fields, 8);
        let clock: Vec<StudioRowView> = studio_rows(
            Some(&studio),
            Some(("Clock", TrackType::ClockAndTransport)),
            &StudioNeeds::default(),
        );
        assert_eq!(
            clock
                .iter()
                .filter(|r| matches!(r.row, StudioRow::Field(_)))
                .count(),
            2
        );
        let cue: Vec<StudioRowView> = studio_rows(
            Some(&studio),
            Some(("Cues", TrackType::CueControl)),
            &StudioNeeds::default(),
        );
        assert!(cue.iter().any(|r| r.label.contains("cue track sends")));
        let none: Vec<StudioRowView> = studio_rows(None, None, &StudioNeeds::default());
        assert_eq!(none[1].row, StudioRow::Create);
    }

    #[test]
    fn typed_text_becomes_the_edit_or_one_sentence() {
        assert_eq!(
            edit_for_field("Keys", OutputField::Channel, "5").unwrap(),
            StudioEdit::SetTrackChannel {
                track_name: String::from("Keys"),
                channel_number: Some(5)
            }
        );
        assert_eq!(
            edit_for_field("Keys", OutputField::Channel, "").unwrap(),
            StudioEdit::SetTrackChannel {
                track_name: String::from("Keys"),
                channel_number: None
            }
        );
        assert!(edit_for_field("Keys", OutputField::RetriggerGap, "abc").is_err());
        assert_eq!(
            edit_for_field("Keys", OutputField::OctaveOffset, "-1").unwrap(),
            StudioEdit::SetOctaveOffset {
                track_name: String::from("Keys"),
                octaves: -1
            }
        );
    }

    #[test]
    fn what_the_song_needs_and_the_studio_lacks_is_listed() {
        let song = song_text::song_file::parse_song(
            "settings_base.song",
            include_str!("../../../songs_canonical/song_settings/settings_base.song"),
        )
        .unwrap();
        let needs: StudioNeeds = needs_of(&studio(), &song);
        assert_eq!(needs.tracks, vec![String::from("Beat")], "no output yet");
        assert_eq!(needs.destination_lines, Vec::<String>::new());
        let without: StudioConfiguration = done(&StudioEdit::DeleteDestination {
            destination_index: 0,
        });
        let needs: StudioNeeds = needs_of(&without, &song);
        assert_eq!(
            needs.destination_lines,
            vec![String::from("Cues send to \"SQ-1\": add that destination.")]
        );
        let rows: Vec<StudioRowView> = studio_rows(Some(&without), None, &needs);
        assert!(rows.iter().any(|r| r.row == StudioRow::Warning));
    }
}
