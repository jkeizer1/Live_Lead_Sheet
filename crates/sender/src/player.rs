//! The sending thread (Architecture, Next, step 3): it owns the ALSA output,
//! opened once with the studio's ports so the musician connects them once,
//! and plays a schedule when asked, from a place in the song, once or in a
//! loop. The window reads where the song is from a shared counter of
//! ticks, so drawing never touches the output.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use alsa_output::sequencer_output::{OutputPort, SequencerOutput};

use crate::play_report::PlayReport;
use crate::play_request::{PlayRequest, Request};
use crate::send_loop::{NO_POSITION, Shared, serve};

/// The ports the thread opened, described for the musician, or why it
/// could not open them.
type PortsOpened = Result<Vec<String>, String>;

pub struct Player {
    requests: Sender<Request>,
    problems: Receiver<String>,
    reports: Receiver<PlayReport>,
    shared: Arc<Shared>,
    ports: Vec<String>,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    /// Opens the output ports in the sending thread.
    ///
    /// # Errors
    ///
    /// When the ALSA sequencer cannot be opened.
    pub fn start(port_names: Vec<String>) -> Result<Player, String> {
        let (requests, request_receiver): (Sender<Request>, Receiver<Request>) = mpsc::channel();
        let (problem_sender, problems): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (report_sender, reports): (Sender<PlayReport>, Receiver<PlayReport>) = mpsc::channel();
        let (ready_sender, ready): (Sender<PortsOpened>, Receiver<PortsOpened>) = mpsc::channel();
        let shared: Arc<Shared> = Arc::new(Shared {
            position_ticks: AtomicU64::new(NO_POSITION),
            playing: AtomicBool::new(false),
            looping: AtomicBool::new(false),
        });
        let thread_shared: Arc<Shared> = Arc::clone(&shared);
        let thread: JoinHandle<()> = thread::Builder::new()
            .name(String::from("sending"))
            .spawn(move || {
                let names: Vec<&str> = port_names.iter().map(String::as_str).collect();
                let output: SequencerOutput = match SequencerOutput::open("Lead Sheet", &names) {
                    Ok((output, warnings)) => {
                        for warning in &warnings {
                            // The window shows these in its message line.
                            let _ = problem_sender.send(warning.to_string());
                        }
                        output
                    }
                    Err(error) => {
                        let _ =
                            ready_sender.send(Err(format!("Cannot open the MIDI output: {error}")));
                        return;
                    }
                };
                let described: Vec<String> = output
                    .ports()
                    .iter()
                    .map(|port: &OutputPort| {
                        format!("{} {}:{}", port.name(), output.client_id(), port.port_id())
                    })
                    .collect();
                let _ = ready_sender.send(Ok(described));
                serve(
                    &output,
                    &request_receiver,
                    &problem_sender,
                    &report_sender,
                    &thread_shared,
                );
            })
            .map_err(|error| format!("Cannot start the sending thread: {error}"))?;
        let ports: Vec<String> = ready.recv().map_err(|_| {
            String::from("The sending thread stopped while opening the MIDI output.")
        })??;
        Ok(Player {
            requests,
            problems,
            reports,
            shared,
            ports,
            thread: Some(thread),
        })
    }

    /// The ports, as `Name client:port`, for the musician to connect.
    #[must_use]
    pub fn ports(&self) -> &[String] {
        &self.ports
    }

    /// Plays `request`, stopping whatever was playing.
    ///
    /// # Errors
    ///
    /// When the sending thread has stopped.
    pub fn play(&self, request: PlayRequest) -> Result<(), String> {
        self.requests
            .send(Request::Play(Box::new(request)))
            .map_err(|_| String::from("The sending thread has stopped; restart the program."))
    }

    /// Stops playing, silencing every note.
    ///
    /// # Errors
    ///
    /// When the sending thread has stopped.
    pub fn stop(&self) -> Result<(), String> {
        self.requests
            .send(Request::Stop)
            .map_err(|_| String::from("The sending thread has stopped; restart the program."))
    }

    #[must_use]
    pub fn playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn looping(&self) -> bool {
        self.shared.looping.load(Ordering::Relaxed)
    }

    /// Where the song is now, in ticks from its start.
    #[must_use]
    pub fn position_ticks(&self) -> Option<u64> {
        let ticks: u64 = self.shared.position_ticks.load(Ordering::Relaxed);
        (ticks != NO_POSITION).then_some(ticks)
    }

    /// A problem the thread met, if any is waiting.
    #[must_use]
    pub fn next_problem(&self) -> Option<String> {
        self.problems.try_recv().ok()
    }

    /// How the timing of a finished play went, if one is waiting.
    #[must_use]
    pub fn next_report(&self) -> Option<PlayReport> {
        self.reports.try_recv().ok()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        // The thread silences the notes as it ends; wait for that, so no
        // note is left sounding when the program closes.
        let _ = self.requests.send(Request::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
