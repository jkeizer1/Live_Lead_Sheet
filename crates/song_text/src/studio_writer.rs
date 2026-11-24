//! Writes a studio configuration as a `.studio` file (Song Notation,
//! section 6), the other direction of `studio_file`: what this writes, that
//! reads back as the same configuration (EFS-6.2).
//!
//! The only text kept from the file being replaced is its comment block at
//! the top (`leading_comments`); the file is otherwise written fresh from
//! the configuration, so comments further down are not kept. A setting at
//! its default (mezzo forte 64, 0 octaves, 0 ms) is left out, which reads
//! back as the same configuration.

use player::studio_configuration::{
    DestinationOutput, StudioConfiguration, TrackOutput, VelocityLevels,
};
use vocabulary::midi_values::{MidiChannel, Velocity};

/// The studio as file text, below `leading_comments` (kept as given).
///
/// # Errors
///
/// When a name cannot be written so that it reads back: it is empty or has
/// a double quote or a line break in it (a name sits between double quotes,
/// with no way to escape one); or a timing is not a whole number of
/// milliseconds, the only form the reader takes. The message names the
/// track, destination or port.
pub fn write_studio(
    studio: &StudioConfiguration,
    leading_comments: &str,
) -> Result<String, String> {
    check_name("studio", studio.name())?;
    let mut sections: Vec<String> = vec![format!("studio \"{}\"", studio.name())];
    if !studio.port_names().is_empty() {
        let mut ports: Vec<String> = Vec::new();
        for port_name in studio.port_names() {
            check_name("port", port_name)?;
            ports.push(format!("port \"{port_name}\""));
        }
        sections.push(ports.join("\n"));
    }
    for track in studio.tracks() {
        sections.push(track_block(track)?);
    }
    for destination in studio.destinations() {
        sections.push(destination_block(destination)?);
    }
    let mut text: String = String::from(leading_comments);
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&sections.join("\n\n"));
    text.push('\n');
    Ok(text)
}

fn check_name(what: &str, name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains(['"', '\n', '\r']) {
        return Err(format!(
            "The {what} name {name:?} cannot be written to the studio file: a name needs at least one character and no double quote or line break."
        ));
    }
    Ok(())
}

fn track_block(track: &TrackOutput) -> Result<String, String> {
    check_name("track", &track.track_name)?;
    check_name("port", &track.port_name)?;
    let mut lines: Vec<String> = vec![format!("track \"{}\"", track.track_name)];
    lines.push(port_line(
        &track.port_name,
        track.channel.map(MidiChannel::number),
    ));
    if let Some(velocity) = velocity_line(track.velocity) {
        lines.push(velocity);
    }
    if track.octave_offset_octaves != 0 {
        lines.push(format!(
            "  octave_offset {} octaves",
            track.octave_offset_octaves
        ));
    }
    for (keyword, duration_ns) in [
        ("retrigger_gap", track.retrigger_gap.ns()),
        ("delay_trigger", track.delay_trigger.ns()),
    ] {
        if duration_ns != 0 {
            lines.push(format!(
                "  {keyword} {} ms",
                whole_milliseconds(&track.track_name, keyword, duration_ns)?
            ));
        }
    }
    Ok(lines.join("\n"))
}

fn destination_block(destination: &DestinationOutput) -> Result<String, String> {
    check_name("destination", &destination.destination_name)?;
    check_name("port", &destination.port_name)?;
    Ok(format!(
        "destination \"{}\"\n{}",
        destination.destination_name,
        port_line(
            &destination.port_name,
            destination.channel.map(MidiChannel::number)
        )
    ))
}

fn port_line(port_name: &str, channel_number: Option<u8>) -> String {
    match channel_number {
        Some(channel) => format!("  port \"{port_name}\"  channel {channel}"),
        None => format!("  port \"{port_name}\""),
    }
}

/// `  velocity mezzo_forte 90  accent 105`, or nothing when every level is
/// at its default (mezzo forte 64, no accent or marcato of its own).
fn velocity_line(levels: VelocityLevels) -> Option<String> {
    if levels.normal() == Velocity::MEZZO_FORTE
        && levels.accent().is_none()
        && levels.marcato().is_none()
    {
        return None;
    }
    let mut levels_written: Vec<String> =
        vec![format!("velocity mezzo_forte {}", levels.normal().value())];
    if let Some(accent) = levels.accent() {
        levels_written.push(format!("accent {}", accent.value()));
    }
    if let Some(marcato) = levels.marcato() {
        levels_written.push(format!("marcato {}", marcato.value()));
    }
    Some(format!("  {}", levels_written.join("  ")))
}

/// A duration as whole milliseconds, or the reason it cannot be written.
fn whole_milliseconds(track_name: &str, keyword: &str, duration_ns: u64) -> Result<u64, String> {
    const NANOSECONDS_PER_MILLISECOND: u64 = 1_000_000;
    if !duration_ns.is_multiple_of(NANOSECONDS_PER_MILLISECOND) {
        return Err(format!(
            "Track \"{track_name}\": {keyword} is {duration_ns} ns, which is not a whole number of milliseconds, so it cannot be written to the studio file."
        ));
    }
    // ns ÷ (ns/ms) = ms
    Ok(duration_ns / NANOSECONDS_PER_MILLISECOND)
}

#[cfg(test)]
mod tests {
    use super::write_studio;
    use crate::studio_file::parse_studio;
    use crate::text_line::leading_comments;
    use player::studio_configuration::{StudioConfiguration, TrackOutput, VelocityLevels};
    use vocabulary::duration::Duration;
    use vocabulary::midi_values::{MidiChannel, Velocity};

    const ALL_SETTINGS: &str =
        include_str!("../../../songs_canonical/studio_writer/all_settings.studio");
    const DEFAULTS_LOUD: &str =
        include_str!("../../../songs_canonical/studio_writer/defaults_written_out.studio");
    const DEFAULTS_LOUD_WRITTEN: &str =
        include_str!("../../../songs_canonical/studio_writer/defaults_written_out_written.studio");
    const COMMENTS: &str =
        include_str!("../../../songs_canonical/studio_writer/comments_kept_at_top.studio");
    const COMMENTS_WRITTEN: &str =
        include_str!("../../../songs_canonical/studio_writer/comments_kept_at_top_written.studio");
    const DEMO: &str = include_str!(
        "../../../songs_canonical/count_in_and_verse/song_test_count_in_and_verse.studio"
    );

    fn read(name: &str, text: &str) -> StudioConfiguration {
        parse_studio(name, text).unwrap()
    }

    /// A one-track studio, for the refusals.
    fn studio_with(track: TrackOutput, studio_name: &str) -> StudioConfiguration {
        StudioConfiguration::new(
            studio_name,
            vec![String::from("P")],
            vec![track],
            Vec::new(),
        )
        .unwrap()
    }

    fn track(name: &str) -> TrackOutput {
        TrackOutput {
            track_name: name.to_string(),
            port_name: String::from("P"),
            channel: Some(MidiChannel::new(1).unwrap()),
            velocity: VelocityLevels::new(Velocity::MEZZO_FORTE, None, None),
            octave_offset_octaves: 0,
            retrigger_gap: Duration::ZERO,
            delay_trigger: Duration::ZERO,
        }
    }

    #[test]
    fn a_file_in_the_writers_form_is_written_back_byte_for_byte() {
        let studio: StudioConfiguration = read("all_settings.studio", ALL_SETTINGS);
        let written: String = write_studio(&studio, leading_comments(ALL_SETTINGS)).unwrap();
        assert_eq!(written, ALL_SETTINGS);
    }

    #[test]
    fn what_is_written_reads_back_as_the_same_studio() {
        for (name, text) in [
            ("all_settings.studio", ALL_SETTINGS),
            ("defaults_written_out.studio", DEFAULTS_LOUD),
            ("comments_kept_at_top.studio", COMMENTS),
            ("song_test_count_in_and_verse.studio", DEMO),
        ] {
            let studio: StudioConfiguration = read(name, text);
            let written: String = write_studio(&studio, leading_comments(text)).unwrap();
            assert_eq!(read("written.studio", &written), studio, "{name}");
        }
    }

    #[test]
    fn settings_at_their_defaults_are_left_out() {
        let studio: StudioConfiguration = read("defaults_written_out.studio", DEFAULTS_LOUD);
        let written: String = write_studio(&studio, leading_comments(DEFAULTS_LOUD)).unwrap();
        assert_eq!(written, DEFAULTS_LOUD_WRITTEN);
    }

    #[test]
    fn the_top_comment_block_is_kept_and_later_comments_are_not() {
        assert_eq!(leading_comments(COMMENTS).lines().count(), 4);
        let studio: StudioConfiguration = read("comments_kept_at_top.studio", COMMENTS);
        let written: String = write_studio(&studio, leading_comments(COMMENTS)).unwrap();
        assert_eq!(written, COMMENTS_WRITTEN);
    }

    #[test]
    fn a_studio_with_no_comment_block_starts_with_its_name() {
        let studio: StudioConfiguration = read("all_settings.studio", ALL_SETTINGS);
        let written: String = write_studio(&studio, "").unwrap();
        assert!(written.starts_with("studio \"Jim's studio\"\n"));
    }

    #[test]
    fn a_name_with_a_quote_is_refused_and_names_what_it_was() {
        let studio: StudioConfiguration = studio_with(track("The \"bass\""), "S");
        let message: String = write_studio(&studio, "").unwrap_err();
        assert!(message.contains("track name"), "{message}");
        assert!(message.contains("double quote"), "{message}");
    }

    #[test]
    fn an_empty_studio_name_is_refused() {
        let studio: StudioConfiguration = studio_with(track("Bass"), "");
        assert!(
            write_studio(&studio, "")
                .unwrap_err()
                .contains("studio name")
        );
    }

    #[test]
    fn a_timing_that_is_not_whole_milliseconds_is_refused() {
        let mut bass: TrackOutput = track("Bass");
        bass.delay_trigger = Duration::from_ns(1_500_000);
        let studio: StudioConfiguration = studio_with(bass, "S");
        let message: String = write_studio(&studio, "").unwrap_err();
        assert!(message.contains("Bass"), "{message}");
        assert!(message.contains("delay_trigger"), "{message}");
    }

    #[test]
    fn a_studio_with_no_ports_or_tracks_reads_back() {
        // What the settings dialog starts for a song that has no studio.
        let studio: StudioConfiguration =
            StudioConfiguration::new("New", Vec::new(), Vec::new(), Vec::new()).unwrap();
        let written: String = write_studio(&studio, "").unwrap();
        assert_eq!(written, "studio \"New\"\n");
        assert_eq!(read("new.studio", &written), studio);
    }
}
