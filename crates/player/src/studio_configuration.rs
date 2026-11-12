//! The studio configuration: which ports exist and what each track of a
//! song is sent to (Song Notation, section 6; D-11). It belongs to the
//! player and not to the vocabulary because it describes devices (A-1); the
//! song says nothing about them.
//!
//! A configuration is written once and serves any song whose tracks it
//! covers. `check_covers` says which of a song's needs it does not meet.

use std::fmt;

use vocabulary::duration::Duration;
use vocabulary::midi_values::{MidiChannel, Velocity};
use vocabulary::rhythm_pattern::AccentLevel;
use vocabulary::song::{CueDestination, CueMessage, ElementContents, Song, Track, TrackKind};

/// The velocity each accent level sends. `normal` is always there: it is
/// `Velocity::MEZZO_FORTE` (64) when the configuration writes none. The
/// accent (`>`) and marcato (`>>`) levels are present only when the
/// configuration gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VelocityLevels {
    normal: Velocity,
    accent: Option<Velocity>,
    marcato: Option<Velocity>,
}

impl VelocityLevels {
    #[must_use]
    pub const fn new(
        normal: Velocity,
        accent: Option<Velocity>,
        marcato: Option<Velocity>,
    ) -> VelocityLevels {
        VelocityLevels {
            normal,
            accent,
            marcato,
        }
    }

    #[must_use]
    pub const fn normal(self) -> Velocity {
        self.normal
    }

    /// The velocity for a level, when the configuration gives one.
    #[must_use]
    pub const fn for_level(self, level: AccentLevel) -> Option<Velocity> {
        match level {
            AccentLevel::Normal => Some(self.normal),
            AccentLevel::Accent => self.accent,
            AccentLevel::Marcato => self.marcato,
        }
    }
}

/// What one track of the song is sent to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackOutput {
    pub track_name: String,
    pub port_name: String,
    /// Absent for the clock and transport track: Timing Clock, Start,
    /// Continue and Stop carry no channel.
    pub channel: Option<MidiChannel>,
    /// The track's velocities. A track that sends no notes (the clock and
    /// transport track) carries them unused.
    pub velocity: VelocityLevels,
    /// Shifts every note sent, in octaves; 0 octaves when omitted, which
    /// sends the note as written.
    pub octave_offset_octaves: i8,
    /// A note off this long before the next note on, so a gate drops;
    /// 0 ms when omitted, which changes nothing.
    pub retrigger_gap: Duration,
    /// Time for a quantizer's voltage to settle after a note is sent; 0 ms
    /// when omitted.
    pub settle_delay: Duration,
}

/// Where a named cue destination goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationOutput {
    pub destination_name: String,
    pub port_name: String,
    /// Absent for a destination that only receives Start, Stop and Continue.
    pub channel: Option<MidiChannel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioConfiguration {
    name: String,
    port_names: Vec<String>,
    tracks: Vec<TrackOutput>,
    destinations: Vec<DestinationOutput>,
}

impl StudioConfiguration {
    /// # Errors
    ///
    /// When a port name repeats; a track or destination names a port the
    /// configuration does not declare; or a track or destination is
    /// written twice.
    pub fn new(
        name: &str,
        port_names: Vec<String>,
        tracks: Vec<TrackOutput>,
        destinations: Vec<DestinationOutput>,
    ) -> Result<StudioConfiguration, StudioError> {
        for (index, port_name) in port_names.iter().enumerate() {
            if port_names[..index].contains(port_name) {
                return Err(StudioError::DuplicatePort {
                    port_name: port_name.clone(),
                });
            }
        }
        for (index, track) in tracks.iter().enumerate() {
            if !port_names.contains(&track.port_name) {
                return Err(StudioError::UndeclaredPort {
                    user: format!("track \"{}\"", track.track_name),
                    port_name: track.port_name.clone(),
                });
            }
            if tracks[..index]
                .iter()
                .any(|earlier| earlier.track_name == track.track_name)
            {
                return Err(StudioError::DuplicateName {
                    what: "track",
                    name: track.track_name.clone(),
                });
            }
        }
        for (index, destination) in destinations.iter().enumerate() {
            if !port_names.contains(&destination.port_name) {
                return Err(StudioError::UndeclaredPort {
                    user: format!("destination \"{}\"", destination.destination_name),
                    port_name: destination.port_name.clone(),
                });
            }
            if destinations[..index]
                .iter()
                .any(|earlier| earlier.destination_name == destination.destination_name)
            {
                return Err(StudioError::DuplicateName {
                    what: "destination",
                    name: destination.destination_name.clone(),
                });
            }
        }
        Ok(StudioConfiguration {
            name: name.to_string(),
            port_names,
            tracks,
            destinations,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn port_names(&self) -> &[String] {
        &self.port_names
    }

    #[must_use]
    pub fn tracks(&self) -> &[TrackOutput] {
        &self.tracks
    }

    #[must_use]
    pub fn destinations(&self) -> &[DestinationOutput] {
        &self.destinations
    }

    #[must_use]
    pub fn output_of(&self, track_name: &str) -> Option<&TrackOutput> {
        self.tracks
            .iter()
            .find(|output| output.track_name == track_name)
    }

    #[must_use]
    pub fn destination(&self, destination_name: &str) -> Option<&DestinationOutput> {
        self.destinations
            .iter()
            .find(|destination| destination.destination_name == destination_name)
    }

    /// Everything the song needs that this configuration does not give, so
    /// the musician can fix it in one pass. Empty when the song can play.
    ///
    /// A track block for a track the song lacks is ignored, so one
    /// configuration serves many songs.
    #[must_use]
    pub fn unmet_needs(&self, song: &Song) -> Vec<UnmetNeed> {
        let mut unmet: Vec<UnmetNeed> = Vec::new();
        for track in song.tracks() {
            self.check_track(song, track, &mut unmet);
        }
        unmet
    }

    fn check_track(&self, song: &Song, track: &Track, unmet: &mut Vec<UnmetNeed>) {
        let track_name: &str = track.name();
        if let TrackKind::CueControl { cues } = track.kind() {
            // A cue track has no output of its own: its cues go to
            // destinations, which are checked here.
            for cue in cues {
                self.check_cue(song, track_name, cue.destination(), cue.message(), unmet);
            }
            return;
        }
        let Some(output) = self.output_of(track_name) else {
            unmet.push(UnmetNeed::NoTrackBlock {
                track_name: track_name.to_string(),
            });
            return;
        };
        let needs_channel: bool = !matches!(track.kind(), TrackKind::ClockAndTransport);
        if needs_channel && output.channel.is_none() {
            unmet.push(UnmetNeed::NoChannel {
                track_name: track_name.to_string(),
            });
        }
        match track.kind() {
            TrackKind::Voice { .. } => {
                let used: Vec<AccentLevel> = accent_levels_used(song, track_name);
                Self::check_velocity(track_name, output, &used, unmet);
            }
            TrackKind::ClockAndTransport | TrackKind::CueControl { .. } => {}
        }
    }

    fn check_velocity(
        track_name: &str,
        output: &TrackOutput,
        used: &[AccentLevel],
        unmet: &mut Vec<UnmetNeed>,
    ) {
        for level in used {
            if output.velocity.for_level(*level).is_none() {
                unmet.push(UnmetNeed::NoVelocity {
                    track_name: track_name.to_string(),
                    level: *level,
                });
            }
        }
    }

    fn check_cue(
        &self,
        song: &Song,
        cue_track_name: &str,
        destination: &CueDestination,
        message: CueMessage,
        unmet: &mut Vec<UnmetNeed>,
    ) {
        let needs_channel: bool = !matches!(
            message,
            CueMessage::Start | CueMessage::Stop | CueMessage::Continue
        );
        match destination {
            CueDestination::Named(name) => match self.destination(name) {
                None => unmet.push(UnmetNeed::NoDestination {
                    destination_name: name.clone(),
                    cue_track_name: cue_track_name.to_string(),
                }),
                Some(found) if needs_channel && found.channel.is_none() => {
                    unmet.push(UnmetNeed::NoDestinationChannel {
                        destination_name: name.clone(),
                    });
                }
                Some(_) => {}
            },
            CueDestination::VoiceTrack(name) => {
                let is_voice: bool = song.tracks().iter().any(|track| {
                    track.name() == name && matches!(track.kind(), TrackKind::Voice { .. })
                });
                if !is_voice {
                    unmet.push(UnmetNeed::CueToUnknownVoice {
                        track_name: name.clone(),
                        cue_track_name: cue_track_name.to_string(),
                    });
                }
            }
        }
    }
}

/// The accent levels a song's lines for `track_name` use above normal,
/// from the marks of their sounding elements.
fn accent_levels_used(song: &Song, track_name: &str) -> Vec<AccentLevel> {
    let mut used: Vec<AccentLevel> = vec![AccentLevel::Normal];
    for section in song.sections() {
        for line in section.lines() {
            if line.track_name() != track_name {
                continue;
            }
            for line_element in line.elements() {
                let level: AccentLevel = line_element.element().marks().accent();
                let sounds: bool = line_element.contents().is_some()
                    && !matches!(line_element.contents(), Some(ElementContents::Cue(_)));
                if sounds && !used.contains(&level) {
                    used.push(level);
                }
            }
        }
    }
    used
}

/// What a song needs that a studio configuration does not give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnmetNeed {
    NoTrackBlock {
        track_name: String,
    },
    NoChannel {
        track_name: String,
    },
    NoVelocity {
        track_name: String,
        level: AccentLevel,
    },
    NoDestination {
        destination_name: String,
        cue_track_name: String,
    },
    NoDestinationChannel {
        destination_name: String,
    },
    CueToUnknownVoice {
        track_name: String,
        cue_track_name: String,
    },
}

impl fmt::Display for UnmetNeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnmetNeed::NoTrackBlock { track_name } => write!(
                f,
                "the studio has no entry for the song's track \"{track_name}\". Add a block: \
                 track \"{track_name}\" with a port and a channel"
            ),
            UnmetNeed::NoChannel { track_name } => write!(
                f,
                "the studio gives track \"{track_name}\" no channel. Add channel 1 (or 1 to 16) \
                 after its port"
            ),
            UnmetNeed::NoVelocity { track_name, level } => {
                let word: &str = match level {
                    AccentLevel::Normal => "normal",
                    AccentLevel::Accent => "accent",
                    AccentLevel::Marcato => "marcato",
                };
                write!(
                    f,
                    "the song plays {word} notes on \"{track_name}\", and the studio gives no \
                     velocity for them. Add velocity {word} 100 (1 to 127) to the track"
                )
            }
            UnmetNeed::NoDestination {
                destination_name,
                cue_track_name,
            } => write!(
                f,
                "cue track \"{cue_track_name}\" sends to \"{destination_name}\", which the studio \
                 does not list. Add destination \"{destination_name}\" with a port"
            ),
            UnmetNeed::NoDestinationChannel { destination_name } => write!(
                f,
                "destination \"{destination_name}\" receives notes, controllers or programs, so it \
                 needs a channel. Add channel 1 (or 1 to 16)"
            ),
            UnmetNeed::CueToUnknownVoice {
                track_name,
                cue_track_name,
            } => write!(
                f,
                "cue track \"{cue_track_name}\" sends to the voice track \"{track_name}\", which \
                 the song does not have. Fix the name"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudioError {
    DuplicatePort { port_name: String },
    UndeclaredPort { user: String, port_name: String },
    DuplicateName { what: &'static str, name: String },
}

impl fmt::Display for StudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StudioError::DuplicatePort { port_name } => {
                write!(f, "the port \"{port_name}\" is listed twice. Keep one")
            }
            StudioError::UndeclaredPort { user, port_name } => write!(
                f,
                "{user} uses the port \"{port_name}\", which the studio does not declare. Add \
                 port \"{port_name}\" at the top, or fix the name"
            ),
            StudioError::DuplicateName { what, name } => {
                write!(f, "the {what} \"{name}\" is written twice. Keep one")
            }
        }
    }
}

impl std::error::Error for StudioError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn velocity(value: u8) -> Velocity {
        Velocity::new(value).unwrap()
    }

    fn output(track_name: &str, port_name: &str, channel: Option<u8>) -> TrackOutput {
        TrackOutput {
            track_name: track_name.to_string(),
            port_name: port_name.to_string(),
            channel: channel.map(|number| MidiChannel::new(number).unwrap()),
            velocity: VelocityLevels::new(velocity(90), Some(velocity(105)), None),
            octave_offset_octaves: 0,
            retrigger_gap: Duration::ZERO,
            settle_delay: Duration::ZERO,
        }
    }

    #[test]
    fn a_track_naming_an_undeclared_port_is_refused() {
        let result = StudioConfiguration::new(
            "S",
            vec!["Harmony".to_string()],
            vec![output("Keys", "Nowhere", Some(1))],
            vec![],
        );
        assert!(matches!(result, Err(StudioError::UndeclaredPort { .. })));
    }

    #[test]
    fn a_repeated_port_or_track_is_refused() {
        assert!(matches!(
            StudioConfiguration::new("S", vec!["A".into(), "A".into()], vec![], vec![]),
            Err(StudioError::DuplicatePort { .. })
        ));
        assert!(matches!(
            StudioConfiguration::new(
                "S",
                vec!["A".into()],
                vec![output("Keys", "A", Some(1)), output("Keys", "A", Some(2))],
                vec![]
            ),
            Err(StudioError::DuplicateName { .. })
        ));
    }

    #[test]
    fn velocity_levels_answer_by_accent_level() {
        let levels: VelocityLevels = VelocityLevels::new(velocity(90), Some(velocity(105)), None);
        assert_eq!(levels.for_level(AccentLevel::Normal), Some(velocity(90)));
        assert_eq!(levels.for_level(AccentLevel::Accent), Some(velocity(105)));
        assert_eq!(levels.for_level(AccentLevel::Marcato), None);
    }
}
