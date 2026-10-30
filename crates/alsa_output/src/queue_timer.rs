//! Which timer drives our queue? D-8 expects the HR timer.
//!
//! The `alsa` crate cannot select a queue's timer, so the adapter only
//! reads what the kernel reports in `/proc/asound/seq/timer`. See
//! docs/Architecture/Musical Time and Clock.md, section 5.

/// The file the kernel lists every queue's timer in.
pub const QUEUE_TIMER_FILE: &str = "/proc/asound/seq/timer";

/// The name the kernel gives the high-resolution timer.
const HR_TIMER_NAME: &str = "HR timer";

/// What the timer file says about one queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueTimerReport {
    HrTimer,
    OtherTimer { timer_name: String },
    QueueNotListed,
}

/// Finds the line "Timer for queue N : NAME" for `queue_id` in the text of
/// the timer file. Other lines (period, skew, other queues) are skipped.
#[must_use]
pub fn report_for_queue(timer_file_text: &str, queue_id: i32) -> QueueTimerReport {
    for line in timer_file_text.lines() {
        let Some(rest): Option<&str> = line.strip_prefix("Timer for queue ") else {
            continue;
        };
        let Some((listed_queue, timer_name)): Option<(&str, &str)> = rest.split_once(" : ") else {
            continue;
        };
        if listed_queue.trim().parse::<i32>() != Ok(queue_id) {
            continue;
        }
        let timer_name: &str = timer_name.trim();
        return if timer_name == HR_TIMER_NAME {
            QueueTimerReport::HrTimer
        } else {
            QueueTimerReport::OtherTimer {
                timer_name: timer_name.to_string(),
            }
        };
    }
    QueueTimerReport::QueueNotListed
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_QUEUES: &str = "Timer for queue 0 : system timer\n  Period time : 0.001000000\n  \
        Skew : 65536 / 65536\nTimer for queue 1 : HR timer\n  Period time : 0.000000000\n  \
        Skew : 65536 / 65536\n";

    #[test]
    fn hr_timer_is_recognised_for_its_queue() {
        assert_eq!(report_for_queue(TWO_QUEUES, 1), QueueTimerReport::HrTimer);
    }

    #[test]
    fn another_timer_is_reported_with_its_name() {
        assert_eq!(
            report_for_queue(TWO_QUEUES, 0),
            QueueTimerReport::OtherTimer {
                timer_name: "system timer".to_string()
            }
        );
    }

    #[test]
    fn unlisted_queue_is_reported() {
        assert_eq!(
            report_for_queue(TWO_QUEUES, 2),
            QueueTimerReport::QueueNotListed
        );
        assert_eq!(report_for_queue("", 1), QueueTimerReport::QueueNotListed);
    }

    #[test]
    fn queue_10_is_not_mistaken_for_queue_1() {
        let text: &str = "Timer for queue 10 : system timer\n";
        assert_eq!(report_for_queue(text, 1), QueueTimerReport::QueueNotListed);
    }
}
