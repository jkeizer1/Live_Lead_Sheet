//! The line of a `meter_control` track: `| mp [3_4 5_4 4_4] |`, one meter
//! pattern that repeats to the end of the section (Song Notation, section 5,
//! "Meter Control").

use vocabulary::bar_meters::BarMeters;
use vocabulary::time_signature::TimeSignature;

/// Reads the bars text of a `meter_control` line.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead; the caller adds
/// the file and line.
pub fn parse_meter_pattern(text: &str) -> Result<BarMeters, String> {
    let example: &str = "write the meter pattern like this: | mp [3_4 5_4] |";
    let text: &str = text.trim();
    if text.len() < 2 || !text.starts_with('|') || !text.ends_with('|') {
        return Err(format!(
            "a meter_control line starts and ends with |. {example}"
        ));
    }
    let inside: &str = text[1..text.len() - 1].trim();
    if inside.contains('|') {
        return Err(format!(
            "a meter_control line holds one mp mark, which repeats to the end of the section by \
             itself. {example}"
        ));
    }
    let Some(after_mp) = inside.strip_prefix("mp") else {
        return Err(format!("a meter_control line starts with mp. {example}"));
    };
    let pattern_text: &str = after_mp.trim();
    let Some(listed) = pattern_text.strip_prefix('[') else {
        return Err(format!(
            "mp is followed by the meters in square brackets. {example}. The pattern repeats to \
             the end of the section, so nothing like section, / or _end is written"
        ));
    };
    let Some(listed) = listed.strip_suffix(']') else {
        return Err(format!(
            "the meters of the pattern end with ]. Nothing is written after it: the pattern \
             repeats to the end of the section by itself. {example}"
        ));
    };
    let mut meters: Vec<TimeSignature> = Vec::new();
    for entry in listed.split_whitespace() {
        meters.push(parse_entry(entry)?);
    }
    BarMeters::pattern(meters).map_err(|error| error.to_string())
}

/// `3_4` → 3/4. The underscore keeps `/` free for the notation's other uses.
fn parse_entry(entry: &str) -> Result<TimeSignature, String> {
    let wrong = || {
        format!(
            "\"{entry}\" is not a meter in a pattern. Write the upper and lower figure with an underscore, e.g. 3_4"
        )
    };
    let Some((upper, lower)) = entry.split_once('_') else {
        return Err(wrong());
    };
    let pulses_per_bar: u8 = upper.parse::<u8>().map_err(|_| wrong())?;
    let pulses_per_whole_note: u8 = lower.parse::<u8>().map_err(|_| wrong())?;
    TimeSignature::new(pulses_per_bar, pulses_per_whole_note)
        .map_err(|error| format!("\"{entry}\": {error}"))
}
