//! Playing a score: the sending loop and the thread it runs on.
//!
//! The `player` crate turns a song and a studio into a list of timed
//! events; this crate sends that list to a MIDI output at the right
//! instants, from anywhere in the song, once or looping, and stops on
//! command. It knows nothing of windows or keys: whatever controls it (the
//! text player, the window) sends it commands and reads where the song is.
//! The output is behind the `EventOutput` interface, so the loop can be
//! tested against a recording output with a simulated clock.

pub mod alsa_event_output;
pub mod event_output;
pub mod play_report;
pub mod play_request;
pub mod player;
mod send_loop;
mod sounding_notes;

#[cfg(test)]
mod commanded_play_tests;
#[cfg(test)]
mod recording_output;
