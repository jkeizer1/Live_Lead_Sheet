//! A read-only handle on the queue's real time, for a thread other than the
//! sender's (the screen reads it once per frame, D-8).
//!
//! The ALSA client handle can move between threads but not be shared, so the
//! screen opens its own client and reads the sender's queue by number.

use std::time::Duration as StdDuration;

use alsa::Direction;
use alsa::seq::{QueueStatus, Seq};
use vocabulary::instant::Instant;

use crate::error::AlsaOutputError;

pub struct QueueClock {
    seq: Seq,
    queue_id: i32,
}

impl QueueClock {
    /// Opens a client that reads queue `queue_id` (see
    /// [`crate::sequencer_output::SequencerOutput::queue_id`]).
    ///
    /// # Errors
    ///
    /// When the sequencer cannot be opened.
    pub fn open(queue_id: i32) -> Result<QueueClock, AlsaOutputError> {
        let seq: Seq = Seq::open(None, Some(Direction::Playback), true).map_err(|e| {
            AlsaOutputError::alsa(format!("opening the ALSA sequencer to read queue {queue_id}"), e)
        })?;
        Ok(QueueClock { seq, queue_id })
    }

    /// The queue's real time, in nanoseconds since the queue was created.
    ///
    /// # Errors
    ///
    /// When the queue's status cannot be read.
    pub fn now(&self) -> Result<Instant, AlsaOutputError> {
        read_queue_instant(&self.seq, self.queue_id)
    }
}

/// Reads queue `queue_id`'s real time through `seq`.
pub(crate) fn read_queue_instant(seq: &Seq, queue_id: i32) -> Result<Instant, AlsaOutputError> {
    let status: QueueStatus = seq
        .get_queue_status(queue_id)
        .map_err(|e| AlsaOutputError::alsa(format!("reading the real time of queue {queue_id}"), e))?;
    let real_time: StdDuration = status.get_real_time();
    // as_nanos is u128 ns; it must fit the u64 ns of an Instant.
    match u64::try_from(real_time.as_nanos()) {
        Ok(instant_ns) => Ok(Instant::from_ns(instant_ns)),
        Err(_) => Err(AlsaOutputError::ClockOutOfRange {
            seconds: real_time.as_secs(),
            nanoseconds: real_time.subsec_nanos(),
        }),
    }
}
