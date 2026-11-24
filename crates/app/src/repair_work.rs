//! A song being repaired in the window: the repair state from the composer,
//! and the files it came from, so saving can write them back.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use composer::repair::Repair;

use crate::song_files::{RepairSource, short_name};

pub struct RepairWork {
    pub repair: Repair,
    pub song_path: PathBuf,
    pub studio_path: Option<PathBuf>,
}

impl RepairWork {
    #[must_use]
    pub fn new(source: RepairSource) -> RepairWork {
        RepairWork {
            repair: Repair::new(source.files, &today_text()),
            song_path: source.song_path,
            studio_path: source.studio_path,
        }
    }

    /// Writes each file whose text changed, as it now stands (junk tossed
    /// and noted, edits made), even when problems remain: the musician's
    /// work is kept either way.
    ///
    /// # Errors
    ///
    /// A message naming the file that could not be written.
    pub fn save(&mut self) -> Result<(), String> {
        let paths: [Option<&Path>; 2] =
            [Some(self.song_path.as_path()), self.studio_path.as_deref()];
        for (index, path) in paths.into_iter().enumerate() {
            let Some(path) = path else {
                continue;
            };
            if !self.repair.is_file_changed(index) {
                continue;
            }
            let Some(file) = self.repair.files().get(index) else {
                continue;
            };
            std::fs::write(path, &file.text).map_err(|error| {
                format!(
                    "Cannot save {}: {error}. Nothing else was changed.",
                    short_name(path)
                )
            })?;
        }
        self.repair.mark_saved();
        Ok(())
    }
}

/// Today's date as the musician writes it ("24 Nov 2026"), in UTC: the
/// standard library has no calendar, and a date in a note about a tossed
/// line need not follow the time zone.
fn today_text() -> String {
    const SECONDS_PER_DAY: u64 = 86_400;
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        // s ÷ (s/day) = days since 1 Jan 1970. A date is under 2^63 days.
        Ok(since_epoch) => {
            date_text(i64::try_from(since_epoch.as_secs() / SECONDS_PER_DAY).unwrap_or(0))
        }
        // A clock set before 1970: the note only needs some words there, and
        // this says plainly why there is no date.
        Err(_) => String::from("(clock not set)"),
    }
}

/// The civil date of a day counted from 1 Jan 1970 (Howard Hinnant's
/// `civil_from_days`, which counts years from 1 March so the leap day is
/// last in the year).
fn date_text(days_since_epoch: i64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    const DAYS_FROM_MARCH_YEAR_0_TO_EPOCH: i64 = 719_468;
    const DAYS_PER_400_YEARS: i64 = 146_097;
    let shifted_days: i64 = days_since_epoch + DAYS_FROM_MARCH_YEAR_0_TO_EPOCH;
    let era: i64 = shifted_days.div_euclid(DAYS_PER_400_YEARS);
    let day_of_era: i64 = shifted_days - era * DAYS_PER_400_YEARS;
    let year_of_era: i64 =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year: i64 = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march: i64 = (5 * day_of_year + 2) / 153;
    let day: i64 = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month: i64 = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year: i64 = year_of_era + era * 400 + i64::from(month <= 2);
    let month_name: &str = usize::try_from(month - 1)
        .ok()
        .and_then(|index: usize| MONTHS.get(index))
        .copied()
        .unwrap_or("???");
    format!("{day} {month_name} {year}")
}

#[cfg(test)]
mod tests {
    use super::date_text;

    #[test]
    fn days_since_1970_become_a_date() {
        assert_eq!(date_text(0), "1 Jan 1970");
        assert_eq!(date_text(19_723), "1 Jan 2024");
        assert_eq!(date_text(19_782), "29 Feb 2024");
        assert_eq!(date_text(20_781), "24 Nov 2026");
    }
}
