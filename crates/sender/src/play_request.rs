//! What to play, and the commands the sending thread takes.

use player::schedule::ScheduledEvent;
use player::song_plan::TempoChange;
use vocabulary::midi_values::MidiChannel;
use vocabulary::phrase_cue::OutputPortName;
use vocabulary::tempo::Tempo;

/// What to play.
pub struct PlayRequest {
    pub events: Vec<ScheduledEvent>,
    pub tempo: Tempo,
    pub tempo_changes: Vec<TempoChange>,
    /// Where to start, in ticks from the start of the song.
    pub start_ticks: u64,
    /// Where to stop (or loop back from); `None` plays to the end.
    pub end_ticks: Option<u64>,
    pub looping: bool,
    /// The clock and transport port, for Start and Stop.
    pub clock_port: OutputPortName,
    /// Each note track's port and channel, silenced when playing stops.
    pub note_outputs: Vec<(OutputPortName, MidiChannel)>,
}

/// A command to the sending thread.
pub(crate) enum Request {
    Play(Box<PlayRequest>),
    Stop,
    Quit,
}
