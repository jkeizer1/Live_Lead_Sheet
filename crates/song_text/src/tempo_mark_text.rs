//! The tempo mark at the start of a bar of the lead line: `tempo 130
//! beats/min`, or `tempo 110 ramp_to 130 beats/min 2_bars` (Song Notation,
//! section 5, "Tempo marks on the lead line").

use vocabulary::score_length::ScoreLength;
use vocabulary::tempo::{Tempo, TempoError};
use vocabulary::tempo_mark::TempoMark;

use crate::rhythm_bar::split_at_barlines;
use crate::song_file::parse_milli_beats;

const EXAMPLES: &str = "write tempo 130 beats/min, or tempo 110 ramp_to 130 beats/min 2_bars";

/// Takes the tempo mark off the front of each bar of `text`, a line of bars
/// such as `| tempo 90 beats/min I:w | IV:w |`. Returns the bars without the
/// marks, so the bar parser reads what is left, and the marks with their bar
/// counted from the start of the line (`first_bar_index` is the number of
/// bars before this text).
///
/// `allowed` is true for the lead track's first voice layer, the only line
/// that takes marks. `beat_length` is the beat of the section's meter, which
/// says what a beat per minute is.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead, naming the bar
/// by its number in the line; the caller adds the file and line.
pub fn take_tempo_marks(
    text: &str,
    first_bar_index: usize,
    beat_length: ScoreLength,
    allowed: bool,
) -> Result<(String, Vec<TempoMark>), String> {
    let trimmed: &str = text.trim();
    // Anything that is not a line of bars is left to the bar parser to report.
    if trimmed.len() < 2 || !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return Ok((text.to_string(), Vec::new()));
    }
    let inside: &str = &trimmed[1..trimmed.len() - 1];
    let mut bars: Vec<String> = Vec::new();
    let mut marks: Vec<TempoMark> = Vec::new();
    for (offset, bar_text) in split_at_barlines(inside).into_iter().enumerate() {
        let bar_number: usize = first_bar_index + offset + 1;
        let (word, after_word): (&str, &str) = next_word(bar_text);
        if word != "tempo" {
            bars.push(bar_text.to_string());
            continue;
        }
        if !allowed {
            return Err(format!(
                "bar {bar_number}: a tempo mark goes on the first voice layer of the lead track. \
                 Move it there"
            ));
        }
        let bar_index: u32 = u32::try_from(bar_number - 1)
            .map_err(|_| format!("bar {bar_number}: the line has too many bars"))?;
        let (mark, rest): (TempoMark, &str) =
            read_mark(after_word, bar_index, bar_number, beat_length)?;
        if next_word(rest).0 == "tempo" {
            return Err(format!(
                "bar {bar_number}: a bar holds one tempo mark. Put the second one at the start of \
                 a later bar"
            ));
        }
        marks.push(mark);
        bars.push(rest.to_string());
    }
    Ok((format!("|{}|", bars.join("|")), marks))
}

/// `130 beats/min` or `110 ramp_to 130 beats/min 2_bars`, and what follows.
fn read_mark(
    after_tempo: &str,
    bar_index: u32,
    bar_number: usize,
    beat_length: ScoreLength,
) -> Result<(TempoMark, &str), String> {
    let wrong = || format!("bar {bar_number}: the tempo mark is not complete. {EXAMPLES}");
    let (first_number, rest): (&str, &str) = next_word(after_tempo);
    let start_milli_beats: u64 = number_of(first_number, bar_number)?;
    let (word, rest): (&str, &str) = next_word(rest);
    if word == "ramp_to" {
        let (second_number, rest): (&str, &str) = next_word(rest);
        let end_milli_beats: u64 = number_of(second_number, bar_number)?;
        let (unit, rest): (&str, &str) = next_word(rest);
        let unit: TempoUnit = unit_of(unit, bar_number)?;
        let (length, rest): (&str, &str) = next_word(rest);
        let Some(bars_text) = length.strip_suffix("_bars") else {
            return Err(format!(
                "bar {bar_number}: a ramp says how many bars it takes, e.g. 2_bars. {EXAMPLES}"
            ));
        };
        let ramp_bars: u32 = bars_text.parse::<u32>().map_err(|_| wrong())?;
        let start: Tempo = tempo_of(
            first_number,
            start_milli_beats,
            unit,
            beat_length,
            bar_number,
        )?;
        let end: Tempo = tempo_of(
            second_number,
            end_milli_beats,
            unit,
            beat_length,
            bar_number,
        )?;
        let Some(mark) = TempoMark::ramp(bar_index, start, end, ramp_bars) else {
            return Err(format!(
                "bar {bar_number}: a ramp over 0 bars never moves. Write 1_bars or more"
            ));
        };
        Ok((mark, rest))
    } else {
        let unit: TempoUnit = unit_of(word, bar_number)?;
        let tempo: Tempo = tempo_of(
            first_number,
            start_milli_beats,
            unit,
            beat_length,
            bar_number,
        )?;
        Ok((TempoMark::step(bar_index, tempo), rest))
    }
}

/// The unit a tempo mark is written in, as for a section's `tempo`.
#[derive(Clone, Copy)]
enum TempoUnit {
    BeatsPerMinute,
    BeatsPerSecond,
}

fn unit_of(unit: &str, bar_number: usize) -> Result<TempoUnit, String> {
    match unit {
        "beats/min" => Ok(TempoUnit::BeatsPerMinute),
        "beats/s" => Ok(TempoUnit::BeatsPerSecond),
        _ => Err(format!(
            "bar {bar_number}: the tempo mark needs its unit, beats/min or beats/s. {EXAMPLES}"
        )),
    }
}

fn number_of(text: &str, bar_number: usize) -> Result<u64, String> {
    parse_milli_beats(text).ok_or_else(|| {
        format!(
            "bar {bar_number}: \"{text}\" is not a tempo. Write a number with up to 3 decimals, \
             e.g. tempo 130 beats/min"
        )
    })
}

/// The tempo of a written number of beats per minute or second, in the
/// section's beat. `milli_beats` is the number times 1000.
fn tempo_of(
    text: &str,
    milli_beats: u64,
    unit: TempoUnit,
    beat_length: ScoreLength,
    bar_number: usize,
) -> Result<Tempo, String> {
    let tempo: Result<Tempo, TempoError> = match unit {
        TempoUnit::BeatsPerMinute => Tempo::from_milli_beats_per_minute(milli_beats, beat_length),
        TempoUnit::BeatsPerSecond => Tempo::from_milli_beats_per_second(milli_beats, beat_length),
    };
    tempo.map_err(|error| match error {
        TempoError::Zero => {
            format!("bar {bar_number}: a tempo of {text} never plays. Use more than 0")
        }
        _ => format!("bar {bar_number}: {text}: {error}"),
    })
}

/// The first word of `text` and what follows it.
fn next_word(text: &str) -> (&str, &str) {
    let trimmed: &str = text.trim_start();
    let end: usize = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    trimmed.split_at(end)
}
