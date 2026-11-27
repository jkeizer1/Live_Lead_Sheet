//! Everything that exists only while a song is open: the editing session,
//! the song file (saved, and watched for changes made in another editor),
//! the studio, and the sending thread that plays the song through the
//! studio's ports. The window holds one when a song is open and none
//! otherwise.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use composer::session::{Command, MessageKind, Session};
use composer::settings_view::{SettingsCommand, SettingsSnapshot};
use composer::song_settings::{StudioChange, TrackType, all_type_options, apply, studio_after};
use composer::studio_settings::{
    StudioEdit, StudioNeeds, apply_studio_edit, needs_of, used_destinations,
};
use composer::timeline::EntryView;
use player::schedule::{ScheduledEvent, schedule};
use player::song_plan::{SongPlan, plan_song};
use player::studio_configuration::StudioConfiguration;
use render::screen::PlayView;
use sender::play_request::PlayRequest;
use sender::player::Player;
use song_text::studio_file::parse_studio;
use song_text::studio_writer::write_studio;
use song_text::text_line::leading_comments;
use vocabulary::song::TrackKind;

use crate::song_files::{LoadedSong, SongFile, short_name, studio_path_beside};

/// How often the song file is looked at for changes made elsewhere.
const FILE_CHECK_INTERVAL: Duration = Duration::from_secs(1);

pub struct Workspace {
    pub session: Session,
    pub song_file: SongFile,
    studio: Option<StudioConfiguration>,
    studio_path: Option<PathBuf>,
    /// Looks for changes made to the studio file elsewhere (the tuning
    /// script writes it while the program is open).
    studio_watch: Option<SongFile>,
    /// The studio file's text as last read or written; its top comment
    /// block is kept when the studio is written.
    studio_text: String,
    /// The studio was changed by a track rename or removal and is not yet
    /// written.
    studio_dirty: bool,
    player: Option<Player>,
    /// About the MIDI output, for the header.
    output_status: String,
    last_file_check: Instant,
    /// The number of the last message written to the terminal.
    last_logged_message_number: u64,
}

impl Workspace {
    /// Opens the studio's MIDI output ports (when there is a studio) and
    /// keeps the song ready to edit. A port that will not open turns
    /// playing off and says so; the song still opens.
    #[must_use]
    pub fn open(loaded: LoadedSong) -> Workspace {
        let (player, output_status): (Option<Player>, String) = match (
            &loaded.studio,
            &loaded.studio_path,
        ) {
            // A new song's studio has no ports until Song settings adds them.
            (Some(studio), Some(_)) if studio.port_names().is_empty() => (
                None,
                String::from("The studio has no ports: playing is off"),
            ),
            (Some(studio), Some(path)) => match Player::start(studio.port_names().to_vec()) {
                Ok(player) => {
                    println!(
                        "MIDI output ports for {} (connect them with the MIDI router or aconnect):",
                        short_name(path)
                    );
                    for port in player.ports() {
                        println!("  {port}");
                    }
                    let status: String =
                        format!("{} ports open ({})", player.ports().len(), short_name(path));
                    (Some(player), status)
                }
                Err(problem) => {
                    eprintln!("{problem}");
                    (
                        None,
                        String::from("MIDI output did not open: see the terminal"),
                    )
                }
            },
            _ => (None, String::from("No studio file: playing is off")),
        };
        Workspace {
            session: loaded.session,
            song_file: loaded.song_file,
            studio: loaded.studio,
            studio_watch: loaded
                .studio_path
                .clone()
                .map(|path| SongFile::watching(path, "studio file")),
            studio_path: loaded.studio_path,
            studio_text: loaded.studio_text.unwrap_or_default(),
            studio_dirty: false,
            player,
            output_status,
            last_file_check: Instant::now(),
            last_logged_message_number: 0,
        }
    }

    #[must_use]
    pub fn has_unsaved_edits(&self) -> bool {
        self.session.document().has_unsaved_edits() || self.studio_dirty
    }

    /// Carries out a change from the song settings dialog. A track renamed
    /// or removed is renamed or removed in the studio too, so the studio
    /// still names the song's tracks; the studio file is written with the
    /// song.
    pub fn apply_settings(&mut self, command: SettingsCommand) {
        let command: SettingsCommand = match command {
            SettingsCommand::Studio(edit) => {
                self.apply_studio_edit(&edit);
                return;
            }
            SettingsCommand::CreateStudio => {
                self.create_studio();
                return;
            }
            song_command => song_command,
        };
        let Some(change) = apply(&mut self.session, command) else {
            return;
        };
        let Some(studio) = &self.studio else {
            return;
        };
        match studio_after(studio, &change) {
            Ok(changed) => {
                self.studio = Some(changed);
                self.studio_dirty = true;
                let what: String = match change {
                    StudioChange::Rename { old_name, new_name } => {
                        format!("Track {old_name} renamed to {new_name}, in the studio too.")
                    }
                    StudioChange::Remove { track_name } => {
                        format!("Track {track_name} deleted, from the studio too.")
                    }
                };
                self.session.say(MessageKind::Done, what);
            }
            Err(problem) => self.session.say(
                MessageKind::Problem,
                format!("The studio could not follow the change: {problem}"),
            ),
        }
    }

    /// The song text and the studio text as they are now, unsaved edits
    /// included, for saving them under a new name.
    ///
    /// # Errors
    ///
    /// When the studio cannot be written as text.
    pub fn texts_for_save_as(&self) -> Result<(String, Option<String>), String> {
        let studio_text: Option<String> = match &self.studio {
            Some(studio) => Some(write_studio(studio, leading_comments(&self.studio_text))?),
            None => None,
        };
        Ok((self.session.document().text().to_string(), studio_text))
    }

    /// What the settings dialog is made from, now.
    #[must_use]
    pub fn settings_snapshot(&self) -> SettingsSnapshot {
        let song = self.session.document().song();
        SettingsSnapshot {
            title: song.name().to_string(),
            track_names: song
                .tracks()
                .iter()
                .map(|track| track.name().to_string())
                .collect(),
            track_types: song.tracks().iter().map(TrackType::of).collect(),
            track_clefs: song
                .tracks()
                .iter()
                .map(|track| match track.kind() {
                    TrackKind::Voice { clef, .. } => Some(*clef),
                    _ => None,
                })
                .collect(),
            type_options: all_type_options(&self.session),
            needs: self
                .studio
                .as_ref()
                .map_or_else(StudioNeeds::default, |studio| needs_of(studio, song)),
            studio: self.studio.clone(),
        }
    }

    /// A change to the studio from the dialog's studio pane. It is written
    /// with the song (Ctrl+S); a change to the ports opens the MIDI output
    /// again, since the ports are made when the output opens.
    fn apply_studio_edit(&mut self, edit: &StudioEdit) {
        let Some(studio) = &self.studio else {
            self.session.say(
                MessageKind::Problem,
                String::from("There is no studio file to change."),
            );
            return;
        };
        let used: Vec<String> = used_destinations(self.session.document().song());
        match apply_studio_edit(studio, edit, &used) {
            Ok(changed) => {
                let ports_changed: bool = changed.port_names() != studio.port_names();
                self.studio = Some(changed);
                self.studio_dirty = true;
                if ports_changed {
                    self.restart_player();
                }
                self.session.say(
                    MessageKind::Done,
                    String::from("Studio changed. Ctrl+S saves it."),
                );
            }
            Err(reason) => self.session.say(MessageKind::Problem, reason),
        }
    }

    /// Starts a studio for a song that has none: named like the song, with
    /// no ports yet. The file is made when the song is saved.
    fn create_studio(&mut self) {
        if self.studio.is_some() {
            self.session.say(
                MessageKind::Problem,
                String::from("The song already has a studio."),
            );
            return;
        }
        let name: String = self.session.document().song().name().to_string();
        match StudioConfiguration::new(&name, Vec::new(), Vec::new(), Vec::new()) {
            Ok(studio) => {
                self.studio = Some(studio);
                self.studio_path = Some(studio_path_beside(&self.song_file.path));
                self.studio_text = String::new();
                self.studio_dirty = true;
                self.restart_player();
                self.session.say(
                    MessageKind::Done,
                    String::from("Studio started. Add a port and an output for each track."),
                );
            }
            Err(problem) => self.session.say(
                MessageKind::Problem,
                format!("Cannot start a studio: {problem}"),
            ),
        }
    }

    /// Closes the MIDI output (which silences the notes) and opens it again
    /// on the studio's ports.
    fn restart_player(&mut self) {
        self.player = None;
        let Some(studio) = &self.studio else {
            return;
        };
        if studio.port_names().is_empty() {
            self.output_status = String::from("The studio has no ports: playing is off");
            return;
        }
        match Player::start(studio.port_names().to_vec()) {
            Ok(player) => {
                self.output_status =
                    format!("{} ports open (studio changed)", player.ports().len());
                self.player = Some(player);
            }
            Err(problem) => {
                eprintln!("{problem}");
                self.output_status = String::from("MIDI output did not open: see the terminal");
            }
        }
    }

    /// The studio file changed elsewhere: take it up when nothing here is
    /// unsaved, else say which wins.
    fn check_studio_file(&mut self) {
        let Some(watch) = self.studio_watch.as_mut() else {
            return;
        };
        let name: String = short_name(&watch.path);
        let text: String = match watch.changed_text() {
            Ok(Some(text)) if text != self.studio_text => text,
            Ok(_) => return,
            Err(problem) => {
                self.session.say(MessageKind::Problem, problem);
                return;
            }
        };
        if self.studio_dirty {
            self.session.say(
                MessageKind::Problem,
                format!(
                    "{name} changed outside, but there are unsaved studio edits here. Ctrl+S keeps these and replaces the file's changes."
                ),
            );
            return;
        }
        match parse_studio(&name, &text) {
            Ok(changed) => {
                let ports_changed: bool = self
                    .studio
                    .as_ref()
                    .is_none_or(|old| old.port_names() != changed.port_names());
                self.studio = Some(changed);
                self.studio_text = text;
                if ports_changed {
                    self.restart_player();
                }
                self.session.say(
                    MessageKind::Done,
                    format!("{name} changed outside: reloaded."),
                );
            }
            Err(errors) => {
                let first: String = errors.first().map_or_else(String::new, |error| {
                    format!("line {}: {}", error.line_number(), error.message())
                });
                self.session.say(
                    MessageKind::Problem,
                    format!("{name} changed outside but does not read ({first}). The earlier studio stays."),
                );
            }
        }
    }

    fn save_studio(&mut self) -> Result<(), String> {
        let (Some(studio), Some(path)) = (&self.studio, &self.studio_path) else {
            return Ok(());
        };
        let text: String = write_studio(studio, leading_comments(&self.studio_text))?;
        // Written through the watch, so the write is not taken for a change
        // made elsewhere.
        let mut watch: SongFile = self
            .studio_watch
            .take()
            .unwrap_or_else(|| SongFile::watching(path.clone(), "studio file"));
        let written: Result<(), String> = watch.save(&text);
        self.studio_watch = Some(watch);
        written?;
        self.studio_text = text;
        self.studio_dirty = false;
        Ok(())
    }

    /// What the screen shows of the player.
    #[must_use]
    pub fn play_view(&self) -> PlayView {
        match &self.player {
            Some(player) => PlayView {
                playing: player.playing(),
                looping: player.looping(),
                position_ticks: player.position_ticks(),
                status: self.output_status.clone(),
            },
            None => PlayView {
                playing: false,
                looping: false,
                position_ticks: None,
                status: self.output_status.clone(),
            },
        }
    }

    pub fn handle(&mut self, command: Command) {
        self.session.handle(command);
    }

    pub fn save(&mut self) {
        let text: String = self.session.document().text().to_string();
        match self.song_file.save(&text) {
            Ok(()) => {
                self.session.mark_saved();
                let mut saved: String = short_name(&self.song_file.path);
                if self.studio_dirty {
                    match self.save_studio() {
                        Ok(()) => {
                            if let Some(path) = &self.studio_path {
                                saved = format!("{saved} and {}", short_name(path));
                            }
                        }
                        Err(problem) => {
                            self.session.say(MessageKind::Problem, problem);
                            return;
                        }
                    }
                }
                self.session
                    .say(MessageKind::Done, format!("Saved {saved}."));
            }
            Err(problem) => self.session.say(MessageKind::Problem, problem),
        }
    }

    /// Writes a new problem message to the terminal too, so it can be
    /// copied whole; the message line may cut it short. Once per frame
    /// catches every message, since each key or click is followed by one.
    pub fn log_new_problem(&mut self) {
        if let Some(message) = self.session.message()
            && message.number > self.last_logged_message_number
        {
            self.last_logged_message_number = message.number;
            if message.kind == MessageKind::Problem {
                println!("Problem: {}", message.text);
            }
        }
    }

    /// Between frames: the sending thread's problems, and the song file.
    pub fn housekeeping(&mut self) {
        if let Some(player) = &self.player
            && let Some(problem) = player.next_problem()
        {
            self.session
                .say(MessageKind::Problem, format!("MIDI output: {problem}"));
        }
        if let Some(player) = &self.player
            && let Some(report) = player.next_report()
        {
            // Every play is reported in the terminal; a late event is also
            // a problem on the screen, so it is not missed.
            println!("{}", report.summary());
            if report.late_events > 0 {
                self.session.say(MessageKind::Problem, report.summary());
            }
        }
        if self.last_file_check.elapsed() < FILE_CHECK_INTERVAL {
            return;
        }
        self.last_file_check = Instant::now();
        self.check_studio_file();
        match self.song_file.changed_text() {
            Ok(Some(text)) if text != self.session.document().text() => {
                if self.session.document().has_unsaved_edits() {
                    self.session.say(
                        MessageKind::Problem,
                        String::from("The song file changed outside, but there are unsaved edits here. Ctrl+S keeps these; Ctrl+Z back to the saved version, then the file's changes load."),
                    );
                } else {
                    self.session.reload(text);
                }
            }
            Ok(_) => {}
            Err(problem) => self.session.say(MessageKind::Problem, problem),
        }
    }

    pub fn play_or_stop(&mut self, from_start: bool, looping: bool) {
        let Some(player) = &self.player else {
            self.session.say(
                MessageKind::Problem,
                String::from(
                    "Playing is off: the song needs a studio file of the same name beside it.",
                ),
            );
            return;
        };
        if player.playing() {
            if let Err(problem) = player.stop() {
                self.session.say(MessageKind::Problem, problem);
            }
            return;
        }
        let Some(studio) = &self.studio else {
            return;
        };
        let timeline = self.session.timeline();
        let Some(entry) = timeline.entries.get(self.session.selection.entry) else {
            return;
        };
        if entry.muted && !from_start {
            self.session.say(
                MessageKind::Problem,
                String::from(
                    "This entry is muted (repeat 0); press M to restore it, or pick another.",
                ),
            );
            return;
        }
        let (start_ticks, end_ticks): (u64, Option<u64>) = if from_start {
            (0, None)
        } else if looping {
            (
                entry.start_ticks,
                Some(entry.start_ticks + entry.length_ticks()),
            )
        } else {
            (entry.start_ticks, None)
        };
        let what: String =
            describe_play(entry, from_start, looping, self.session.preview().is_some());
        // What this window asked the sender to play, in the terminal, so a
        // play that sounds wrong can be told from one asked for wrongly.
        println!(
            "Play asked: {} from {} at tick {start_ticks}{}{}.",
            if self.session.preview().is_some() {
                "the PREVIEW"
            } else {
                "the song"
            },
            if from_start {
                String::from("the start")
            } else {
                format!("\"{}\"", entry.section_name)
            },
            end_ticks.map_or_else(String::new, |end| format!(" to tick {end}")),
            if looping { ", looping" } else { "" },
        );
        let song = self.session.playing_song();
        let request: Result<PlayRequest, String> = plan_song(song, studio)
            .map_err(|error| format!("Cannot play: {error}"))
            .and_then(|plan: SongPlan| {
                let events: Vec<ScheduledEvent> = schedule(
                    &plan.clock_port,
                    &plan.note_tracks,
                    &plan.control_tracks,
                    plan.stretch_length,
                )
                .map_err(|error| format!("Cannot play: {error}"))?;
                Ok(PlayRequest {
                    events,
                    tempo: plan.tempo,
                    tempo_changes: plan.tempo_changes.clone(),
                    start_ticks,
                    end_ticks,
                    looping,
                    clock_port: plan.clock_port.clone(),
                    note_outputs: plan
                        .note_tracks
                        .iter()
                        .map(|track| (track.port().clone(), track.channel()))
                        .collect(),
                })
            });
        match request.and_then(|request| player.play(request)) {
            Ok(()) => self.session.say(MessageKind::Done, what),
            Err(problem) => self.session.say(MessageKind::Problem, problem),
        }
    }

    /// The song file's path, for the window's title.
    #[must_use]
    pub fn song_path(&self) -> &PathBuf {
        &self.song_file.path
    }
}

/// What the message line says when playing starts.
fn describe_play(entry: &EntryView, from_start: bool, looping: bool, preview: bool) -> String {
    let what: &str = if preview { "the preview" } else { "the song" };
    if from_start {
        format!("Playing {what} from the start. Space stops.")
    } else if looping {
        format!("Looping \"{}\" in {what}. Space stops.", entry.section_name)
    } else {
        format!(
            "Playing {what} from \"{}\". Space stops.",
            entry.section_name
        )
    }
}
