//! Problems the program can live with, shown in the warnings region
//! (A-8, EFS-1.14). Written for the musician mid-session: short, plain,
//! saying what to do.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputWarning {
    /// Our queue runs on a timer other than the HR timer.
    QueueTimerNotHr { queue_id: i32, timer_name: String },
    /// The timer file has no line for our queue.
    QueueTimerNotListed { queue_id: i32 },
    /// The timer file could not be read.
    QueueTimerFileUnreadable { file: String, reason: String },
}

impl fmt::Display for OutputWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputWarning::QueueTimerNotHr {
                queue_id,
                timer_name,
            } => write!(
                f,
                "MIDI timing may be uneven: queue {queue_id} runs on '{timer_name}', \
                 not the HR timer. Load module snd_hrtimer, then restart."
            ),
            OutputWarning::QueueTimerNotListed { queue_id } => write!(
                f,
                "Cannot confirm MIDI timing: queue {queue_id} is not in the kernel's timer list. \
                 Restart; if it stays, tell Jim."
            ),
            OutputWarning::QueueTimerFileUnreadable { file, reason } => write!(
                f,
                "Cannot confirm MIDI timing: {file} is unreadable ({reason}). \
                 Restart; if it stays, tell Jim."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_hr_warning_names_queue_timer_and_fix() {
        assert_eq!(
            OutputWarning::QueueTimerNotHr {
                queue_id: 3,
                timer_name: "system timer".to_string()
            }
            .to_string(),
            "MIDI timing may be uneven: queue 3 runs on 'system timer', \
             not the HR timer. Load module snd_hrtimer, then restart."
        );
    }
}
