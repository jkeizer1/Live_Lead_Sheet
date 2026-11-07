//! The studio file: `studio "Name"`, its `port` declarations, a `track`
//! block for each song track it serves and a `destination` block for each
//! cue destination (Song Notation, section 6).
//!
//! A file with any mistake does not load, not even partly (EFS-6.3). That
//! the studio covers a particular song is a separate check
//! (`StudioConfiguration::unmet_needs`), made when both are loaded.

use player::studio_configuration::{
    DestinationOutput, StudioConfiguration, TrackOutput, VelocityLevels,
};
use vocabulary::duration::Duration;
use vocabulary::midi_values::{MidiChannel, Velocity};

use crate::parse_error::ParseError;
use crate::text_line::{Word, keyword_rest, quoted_prefix, without_comment, words};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Track,
    Destination,
}

/// A track or destination block while it is being read.
struct OpenBlock {
    kind: BlockKind,
    name: String,
    line_number: usize,
    port_name: Option<String>,
    channel: Option<MidiChannel>,
    normal: Option<Velocity>,
    accent: Option<Velocity>,
    strong: Option<Velocity>,
    octave_offset_octaves: Option<i8>,
    retrigger_gap: Option<Duration>,
    settle_delay: Option<Duration>,
    errors_at_start: usize,
}

struct StudioReader<'a> {
    file_name: &'a str,
    errors: Vec<ParseError>,
    studio_name: Option<(String, usize)>,
    port_names: Vec<String>,
    tracks: Vec<TrackOutput>,
    destinations: Vec<DestinationOutput>,
    block: Option<OpenBlock>,
}

/// Reads a studio file.
///
/// # Errors
///
/// Every mistake found, in line order, each naming the file and line.
pub fn parse_studio(file_name: &str, text: &str) -> Result<StudioConfiguration, Vec<ParseError>> {
    let mut reader: StudioReader = StudioReader {
        file_name,
        errors: Vec::new(),
        studio_name: None,
        port_names: Vec::new(),
        tracks: Vec::new(),
        destinations: Vec::new(),
        block: None,
    };
    for (index, raw_line) in text.lines().enumerate() {
        let line: &str = without_comment(raw_line).trim();
        if !line.is_empty() {
            reader.read_line(index + 1, line);
        }
    }
    reader.finish()
}

impl StudioReader<'_> {
    fn report(&mut self, line_number: usize, message: String) {
        self.errors
            .push(ParseError::new(self.file_name, line_number, message));
    }

    fn read_line(&mut self, line_number: usize, line: &str) {
        let result: Result<(), String> = if let Some(rest) = keyword_rest(line, "studio") {
            self.start_studio(line_number, rest)
        } else if let Some(rest) = keyword_rest(line, "track") {
            self.start_block(BlockKind::Track, line_number, rest)
        } else if let Some(rest) = keyword_rest(line, "destination") {
            self.start_block(BlockKind::Destination, line_number, rest)
        } else if let Some(block) = self.block.as_mut() {
            read_settings(block, line)
        } else if let Some(rest) = keyword_rest(line, "port") {
            self.declare_port(rest)
        } else {
            Err(format!(
                "\"{line}\" is outside a block. Start one with studio \"Name\", port \"Name\", \
                 track \"Name\" or destination \"Name\""
            ))
        };
        if let Err(message) = result {
            self.report(line_number, message);
        }
    }

    fn start_studio(&mut self, line_number: usize, rest: &str) -> Result<(), String> {
        if self.studio_name.is_some() {
            return Err(String::from(
                "a second studio line. A file holds one studio",
            ));
        }
        match quoted_prefix(rest) {
            Some((name, "")) => {
                self.studio_name = Some((name.to_string(), line_number));
                Ok(())
            }
            _ => Err(String::from(
                "write the studio's name in quotes, e.g. studio \"Jim's studio\"",
            )),
        }
    }

    fn declare_port(&mut self, rest: &str) -> Result<(), String> {
        match quoted_prefix(rest) {
            Some((name, "")) => {
                self.port_names.push(name.to_string());
                Ok(())
            }
            _ => Err(String::from(
                "write the port's name in quotes, e.g. port \"Harmony\"",
            )),
        }
    }

    fn start_block(
        &mut self,
        kind: BlockKind,
        line_number: usize,
        rest: &str,
    ) -> Result<(), String> {
        self.finish_block();
        let Some((name, "")) = quoted_prefix(rest) else {
            return Err(String::from(
                "write the name in quotes, e.g. track \"Keys\" or destination \"SQ-1\"",
            ));
        };
        self.block = Some(OpenBlock {
            kind,
            name: name.to_string(),
            line_number,
            port_name: None,
            channel: None,
            normal: None,
            accent: None,
            strong: None,
            octave_offset_octaves: None,
            retrigger_gap: None,
            settle_delay: None,
            errors_at_start: self.errors.len(),
        });
        Ok(())
    }

    /// Turns the open block into an output, or reports why not.
    fn finish_block(&mut self) {
        let Some(block) = self.block.take() else {
            return;
        };
        if self.errors.len() > block.errors_at_start {
            return;
        }
        let Some(port_name) = block.port_name.clone() else {
            self.report(
                block.line_number,
                format!(
                    "\"{}\" has no port. Add a line such as port \"Harmony\"",
                    block.name
                ),
            );
            return;
        };
        match block.kind {
            BlockKind::Destination => {
                if let Err(message) = destination_takes_only_port_and_channel(&block) {
                    self.report(block.line_number, message);
                    return;
                }
                self.destinations.push(DestinationOutput {
                    destination_name: block.name,
                    port_name,
                    channel: block.channel,
                });
            }
            BlockKind::Track => match build_track_output(&block, port_name) {
                Ok(output) => self.tracks.push(output),
                Err(message) => self.report(block.line_number, message),
            },
        }
    }

    fn finish(mut self) -> Result<StudioConfiguration, Vec<ParseError>> {
        self.finish_block();
        let Some((name, name_line)) = self.studio_name.clone() else {
            self.report(
                1,
                String::from("the file has no studio. Start it with studio \"Name\""),
            );
            return Err(self.errors);
        };
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        match StudioConfiguration::new(&name, self.port_names, self.tracks, self.destinations) {
            Ok(studio) => Ok(studio),
            Err(error) => {
                self.errors.push(ParseError::new(
                    self.file_name,
                    name_line,
                    error.to_string(),
                ));
                Err(self.errors)
            }
        }
    }
}

fn destination_takes_only_port_and_channel(block: &OpenBlock) -> Result<(), String> {
    if block.normal.is_some()
        || block.accent.is_some()
        || block.strong.is_some()
        || block.octave_offset_octaves.is_some()
        || block.retrigger_gap.is_some()
        || block.settle_delay.is_some()
    {
        return Err(format!(
            "destination \"{}\" takes only a port and a channel",
            block.name
        ));
    }
    Ok(())
}

fn build_track_output(block: &OpenBlock, port_name: String) -> Result<TrackOutput, String> {
    let velocity: Option<VelocityLevels> = match (block.normal, block.accent, block.strong) {
        (Some(normal), accent, strong) => Some(VelocityLevels::new(normal, accent, strong)),
        (None, None, None) => None,
        (None, _, _) => {
            return Err(format!(
                "track \"{}\" gives accent velocities but no normal one. Add velocity normal 90 \
                 (1 to 127)",
                block.name
            ));
        }
    };
    Ok(TrackOutput {
        track_name: block.name.clone(),
        port_name,
        channel: block.channel,
        velocity,
        // Omitted: 0 octaves sends the note as written, and 0 ms changes no
        // timing; both are harmless (Song Notation, section 6).
        octave_offset_octaves: block.octave_offset_octaves.unwrap_or(0),
        retrigger_gap: block.retrigger_gap.unwrap_or(Duration::ZERO),
        settle_delay: block.settle_delay.unwrap_or(Duration::ZERO),
    })
}

/// Reads `port "X"  channel 1`, `velocity normal 90  accent 105`,
/// `octave_offset -1 octaves`, `retrigger_gap 2 ms`, `settle_delay 2 ms`,
/// in any order, on one line or several.
fn read_settings(block: &mut OpenBlock, text: &str) -> Result<(), String> {
    let all: Vec<Word> = words(text)?;
    let mut remaining: &[Word] = &all;
    while let Some((first, after)) = remaining.split_first() {
        if first.quoted {
            return Err(format!(
                "\"{}\" in quotes is not a setting. Expected port, channel, velocity, \
                 octave_offset, retrigger_gap or settle_delay",
                first.text
            ));
        }
        remaining = match first.text.as_str() {
            "port" => read_port(block, after)?,
            "channel" => read_channel(block, after)?,
            "velocity" => read_velocity(block, after)?,
            "octave_offset" => read_octave_offset(block, after)?,
            "retrigger_gap" => {
                let (gap, rest) = read_milliseconds("retrigger_gap", after)?;
                set_once(&mut block.retrigger_gap, gap, "retrigger_gap")?;
                rest
            }
            "settle_delay" => {
                let (delay, rest) = read_milliseconds("settle_delay", after)?;
                set_once(&mut block.settle_delay, delay, "settle_delay")?;
                rest
            }
            other => {
                return Err(format!(
                    "\"{other}\" is not understood in a block. Expected port, channel, \
                     velocity, octave_offset, retrigger_gap or settle_delay"
                ));
            }
        };
    }
    Ok(())
}

fn set_once<T>(slot: &mut Option<T>, value: T, what: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        return Err(format!("{what} is written twice in this block"));
    }
    Ok(())
}

fn read_port<'a>(block: &mut OpenBlock, after: &'a [Word]) -> Result<&'a [Word], String> {
    match after.split_first() {
        Some((name, rest)) if name.quoted => {
            set_once(&mut block.port_name, name.text.clone(), "port")?;
            Ok(rest)
        }
        _ => Err(String::from(
            "write the port's name in quotes, e.g. port \"Harmony\"",
        )),
    }
}

fn read_channel<'a>(block: &mut OpenBlock, after: &'a [Word]) -> Result<&'a [Word], String> {
    let Some((number, rest)) = after.split_first() else {
        return Err(String::from("channel needs a number, e.g. channel 1"));
    };
    let channel_number: u8 = number
        .text
        .parse::<u8>()
        .map_err(|_| format!("\"{}\" is not a channel. Use 1 to 16", number.text))?;
    let channel: MidiChannel = MidiChannel::new(channel_number).map_err(|e| e.to_string())?;
    set_once(&mut block.channel, channel, "channel")?;
    Ok(rest)
}

/// `velocity normal 90  accent 105  strong 120`: levels follow one another
/// until a word that is not a level.
fn read_velocity<'a>(block: &mut OpenBlock, after: &'a [Word]) -> Result<&'a [Word], String> {
    let mut remaining: &[Word] = after;
    let mut read_any: bool = false;
    while let [level, number, rest @ ..] = remaining {
        let slot: &mut Option<Velocity> = match level.text.as_str() {
            "normal" => &mut block.normal,
            "accent" => &mut block.accent,
            "strong" => &mut block.strong,
            _ => break,
        };
        let value: u8 = number.text.parse::<u8>().map_err(|_| {
            format!(
                "\"{}\" is not a velocity. Use 1 to 127, e.g. velocity normal 90",
                number.text
            )
        })?;
        let velocity: Velocity = Velocity::new(value).map_err(|e| e.to_string())?;
        set_once(slot, velocity, &format!("velocity {}", level.text))?;
        remaining = rest;
        read_any = true;
    }
    if !read_any {
        return Err(String::from(
            "velocity needs a level and a number, e.g. velocity normal 90  accent 105",
        ));
    }
    Ok(remaining)
}

fn read_octave_offset<'a>(block: &mut OpenBlock, after: &'a [Word]) -> Result<&'a [Word], String> {
    let wrong = || String::from("write the shift with its unit, e.g. octave_offset -1 octaves");
    let [number, unit, rest @ ..] = after else {
        return Err(wrong());
    };
    if unit.text != "octaves" && unit.text != "octave" {
        return Err(wrong());
    }
    let octaves: i8 = number.text.parse::<i8>().map_err(|_| wrong())?;
    set_once(&mut block.octave_offset_octaves, octaves, "octave_offset")?;
    Ok(rest)
}

fn read_milliseconds<'a>(
    keyword: &str,
    after: &'a [Word],
) -> Result<(Duration, &'a [Word]), String> {
    let wrong = || format!("write {keyword} with its unit, e.g. {keyword} 2 ms");
    let [number, unit, rest @ ..] = after else {
        return Err(wrong());
    };
    if unit.text != "ms" {
        return Err(wrong());
    }
    let milliseconds: u64 = number.text.parse::<u64>().map_err(|_| wrong())?;
    Ok((Duration::from_ms(milliseconds), rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::rhythm_pattern::AccentLevel;

    const DEMO: &str = include_str!("../../../songs/song_test_count_in_and_verse.studio");

    fn errors_of(text: &str) -> Vec<String> {
        parse_studio("test.studio", text)
            .unwrap_err()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn the_demo_studio_loads() {
        let studio: StudioConfiguration =
            parse_studio("song_test_count_in_and_verse.studio", DEMO).unwrap();
        assert_eq!(studio.name(), "Song test studio");
        assert_eq!(studio.port_names().len(), 5);
        let bass: &TrackOutput = studio.output_of("Bass").unwrap();
        assert_eq!(bass.channel.unwrap().number(), 2);
        assert_eq!(bass.octave_offset_octaves, -1);
        assert_eq!(bass.retrigger_gap, Duration::from_ms(2));
        assert_eq!(
            bass.velocity
                .unwrap()
                .for_level(AccentLevel::Accent)
                .unwrap()
                .value(),
            115
        );
        assert!(studio.output_of("Clock").unwrap().channel.is_none());
        assert_eq!(
            studio
                .destination("SQ-1")
                .unwrap()
                .channel
                .unwrap()
                .number(),
            1
        );
    }

    #[test]
    fn a_block_without_a_port_is_reported_at_its_name() {
        let errors: Vec<String> = errors_of("studio \"S\"\ntrack \"Keys\"\n  channel 1\n");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("test.studio line 2:"));
        assert!(errors[0].contains("has no port"));
    }

    #[test]
    fn units_and_ranges_are_checked() {
        let errors: Vec<String> = errors_of(
            "studio \"S\"\nport \"P\"\ntrack \"A\"\n  port \"P\"  channel 17\n\
             track \"B\"\n  port \"P\"  retrigger_gap 2\n\
             track \"C\"\n  port \"P\"  velocity normal 0\n",
        );
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors[0].contains("channel 17"));
        assert!(errors[1].contains("retrigger_gap 2 ms"));
        assert!(errors[2].contains("velocity 0"));
    }

    #[test]
    fn accent_velocities_need_a_normal_one() {
        let errors: Vec<String> = errors_of(
            "studio \"S\"\nport \"P\"\ntrack \"A\"\n  port \"P\"  channel 1\n  velocity accent 100\n",
        );
        assert!(errors[0].contains("no normal one"), "{}", errors[0]);
    }

    #[test]
    fn an_undeclared_port_is_reported() {
        let errors: Vec<String> =
            errors_of("studio \"S\"\nport \"P\"\ntrack \"A\"\n  port \"Q\"  channel 1\n");
        assert!(errors[0].contains("does not declare"), "{}", errors[0]);
    }

    #[test]
    fn the_demo_studio_covers_the_demo_song() {
        let song = crate::song_file::parse_song(
            "song_test_count_in_and_verse.song",
            include_str!("../../../songs/song_test_count_in_and_verse.song"),
        )
        .unwrap();
        let studio: StudioConfiguration =
            parse_studio("song_test_count_in_and_verse.studio", DEMO).unwrap();
        assert_eq!(studio.unmet_needs(&song), vec![]);
    }

    #[test]
    fn a_song_track_without_a_studio_entry_is_named() {
        let song = crate::song_file::parse_song(
            "song_test_count_in_and_verse.song",
            include_str!("../../../songs/song_test_count_in_and_verse.song"),
        )
        .unwrap();
        let without_bass: String = DEMO.replace("track \"Bass\"", "track \"Not the bass\"");
        let studio: StudioConfiguration =
            parse_studio("song_test_count_in_and_verse.studio", &without_bass).unwrap();
        let unmet: Vec<String> = studio
            .unmet_needs(&song)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(unmet.len(), 1);
        assert!(unmet[0].contains("no entry for the song's track \"Bass\""));
    }

    #[test]
    fn a_mark_the_song_uses_needs_its_velocity() {
        let song = crate::song_file::parse_song(
            "song_test_count_in_and_verse.song",
            &include_str!("../../../songs/song_test_count_in_and_verse.song")
                .replace("| q:C2 q q q |", "| q>>:C2 q q q |"),
        )
        .unwrap();
        let studio: StudioConfiguration =
            parse_studio("song_test_count_in_and_verse.studio", DEMO).unwrap();
        let unmet: Vec<String> = studio
            .unmet_needs(&song)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(unmet.len(), 1, "{unmet:?}");
        assert!(unmet[0].contains("strong notes on \"Bass\""));
    }
}
