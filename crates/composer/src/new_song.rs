//! What a new song starts as: the text of its three files (the song, its
//! studio and a MIDI router configuration), kept in a folder named for the
//! song. This makes only text; the app makes the folder and writes it.

use player::studio_configuration::StudioConfiguration;
use song_text::studio_writer::write_studio;

/// The text of the three files of a new song's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSongFiles {
    /// `<name>.song`
    pub song_text: String,
    /// `<name>.studio`
    pub studio_text: String,
    /// `<name>.toml`, for the MIDI router
    pub router_text: String,
}

/// Why a name cannot name a new song, in one sentence. The name is also
/// the folder's and the files' name, and sits between double quotes in the
/// song and the router file.
///
/// # Errors
///
/// The sentence, when the name is empty, starts with a dot, or has a slash,
/// backslash, double quote or line break in it.
pub fn check_song_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err(String::from("Type a name for the song."));
    }
    if name != name.trim() || name.starts_with('.') {
        return Err(String::from(
            "A song name has no space at either end and does not start with a dot.",
        ));
    }
    if name.contains(['/', '\\', '"', '\n', '\r']) {
        return Err(String::from(
            "A song name has no slash, backslash or double quote.",
        ));
    }
    Ok(())
}

/// The files of a new song called `name`: a clock track and one lead
/// voice, one section holding one bar of rest to compose over, a studio
/// with no ports yet (Song settings adds them), and a router file with no
/// ports or routes yet.
///
/// # Errors
///
/// When the name is not allowed, or the studio cannot be written.
pub fn new_song_files(name: &str) -> Result<NewSongFiles, String> {
    check_song_name(name)?;
    let song_text: String = format!(
        "# {name}: a new song. Song settings (F6) manages its tracks and its studio.\n\
         song \"{name}\"\n\
         \n\
         track \"Clock\"\n\
         \x20 type clock_and_transport\n\
         \n\
         track \"Keys\"\n\
         \x20 type poly_voice\n\
         \x20 notes pitched\n\
         \x20 clef grand\n\
         \x20 lead\n\
         \n\
         song_composition\n\
         \x20 key C major  meter 4/4  tempo 120 beats/min\n\
         \x20 \"Section\"\n\
         song_composition_close\n\
         \n\
         section \"Section\"\n\
         \x20 key C major  meter 4/4  tempo 120 beats/min\n\
         \x20 line \"Keys\"  | r:w |\n"
    );
    let studio: StudioConfiguration = StudioConfiguration::new(
        &format!("{name} studio"),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .map_err(|problem| format!("Cannot start the studio: {problem}"))?;
    let studio_text: String = write_studio(
        &studio,
        &format!(
            "# Studio of {name}.song: where its tracks go. Add ports and an output for each track in Song settings (F6, then Tab).\n"
        ),
    )?;
    let router_text: String = format!(
        "# MIDI router configuration for {name}. Start the router here with router_here.sh.\n\
         [configuration]\n\
         name = \"{name}\"\n"
    );
    Ok(NewSongFiles {
        song_text,
        studio_text,
        router_text,
    })
}

#[cfg(test)]
mod tests {
    use super::{check_song_name, new_song_files};
    use song_text::song_file::parse_song;
    use song_text::studio_file::parse_studio;

    #[test]
    fn a_new_song_and_its_studio_read_back() {
        let files = new_song_files("Blue Bossa").unwrap();
        let song = parse_song("Blue Bossa.song", &files.song_text);
        assert!(song.is_ok(), "{song:?}");
        let studio = parse_studio("Blue Bossa.studio", &files.studio_text).unwrap();
        assert_eq!(studio.name(), "Blue Bossa studio");
        assert!(files.router_text.contains("name = \"Blue Bossa\""));
    }

    #[test]
    fn names_that_cannot_be_a_folder_or_be_quoted_are_refused() {
        for bad in ["", "  ", ".hidden", "a/b", "a\\b", "a\"b", " Lead", "Lead "] {
            assert!(check_song_name(bad).is_err(), "{bad:?}");
            assert!(new_song_files(bad).is_err(), "{bad:?}");
        }
        assert!(check_song_name("Nov 24 - Take 2").is_ok());
    }
}
