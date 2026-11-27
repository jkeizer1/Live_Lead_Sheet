//! Writing a track's line back as song notation (Song Notation, section 4):
//! what each element plays, then its length when it changes, then its
//! marks and tie. The editor builds a line's elements, writes them here,
//! and reads the whole song back; the text it writes is what the musician
//! would write by hand, so `| C4:q> / / / |` stays `| C4:q> / / / |`.

use std::fmt;

use note_spelling::note_value::{BaseValue, NoteValue};
use vocabulary::interval::Interval;
use vocabulary::pitch::Pitch;
use vocabulary::rhythm_pattern::{AccentLevel, ElementMarks};
use vocabulary::song::ElementContents;

/// Bars written on each text line, so a long line stays readable.
pub const BARS_PER_TEXT_LINE: usize = 4;

/// What one element of a written line plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WrittenSound {
    Rest,
    /// Contents written before the colon: `I`, `C2`, `[C3 E3]`, `"Cue"`.
    Plays(ElementContents),
    /// Nothing before the colon: takes what it plays from the track's
    /// sample source (`:q`).
    FromSource,
}

/// How a line writes its notes (Song Notation, section 4): note names on a
/// `notes midi_note` track, intervals from the tonic on a pitched one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteSpelling {
    NoteNames,
    /// From the section's tonic, as the selected entry plays it, in the
    /// line's `tonic_octave`.
    Intervals {
        tonic: Pitch,
    },
}

/// One element of a line, as the editor builds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenElement {
    pub sound: WrittenSound,
    pub length_ticks: u64,
    pub marks: ElementMarks,
    pub tied_to_next: bool,
    /// `+8va` is 1, `-8va` −1, none 0 (Song Notation, section 4).
    pub octave_shift_octaves: i8,
}

/// One bar of a line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WrittenBar {
    pub elements: Vec<WrittenElement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineTextError {
    /// A length that is neither a plain or dotted value nor part of a
    /// triplet, such as one of five notes fitted into a quarter.
    LengthNotWritable {
        bar_number: usize,
        length_ticks: u64,
    },
    /// A tie inside a triplet, which the editor does not write yet.
    TieInsideTriplet { bar_number: usize },
    /// An octave annotation other than +8va, -8va or none.
    OctaveShiftNotWritable { bar_number: usize, octaves: i8 },
    /// A note that is no interval the notation writes from the tonic.
    NoteNotWritable { pitch: Pitch, tonic: Pitch },
}

impl fmt::Display for LineTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineTextError::LengthNotWritable {
                bar_number,
                length_ticks,
            } => write!(
                f,
                "bar {bar_number} has a length of {length_ticks} ticks that the editor cannot \
                 write yet (only plain and dotted values and triplets). Edit the text for this line"
            ),
            LineTextError::TieInsideTriplet { bar_number } => write!(
                f,
                "bar {bar_number} has a tie inside a triplet, which the editor cannot write yet. \
                 Edit the text for this line"
            ),
            LineTextError::OctaveShiftNotWritable {
                bar_number,
                octaves,
            } => write!(
                f,
                "bar {bar_number} is {octaves} octaves from the clef's; only +8va and -8va can be \
                 written"
            ),
            LineTextError::NoteNotWritable { pitch, tonic } => write!(
                f,
                "{pitch} is no interval the notation writes from the tonic {tonic}. Respell the \
                 note in the text for this line"
            ),
        }
    }
}

impl std::error::Error for LineTextError {}

/// `q`, `h.`, `sx`: a note value as the song notation writes it.
#[must_use]
pub fn value_text(value: NoteValue) -> String {
    let letter: &str = match value.base() {
        BaseValue::Whole => "w",
        BaseValue::Half => "h",
        BaseValue::Quarter => "q",
        BaseValue::Eighth => "e",
        BaseValue::Sixteenth => "sx",
        BaseValue::ThirtySecond => "tx",
    };
    format!("{letter}{}", ".".repeat(usize::from(value.dots())))
}

/// `>`, `>>`, `'`, `_`, `^` and `~`, in the order the notation fixes.
fn marks_text(marks: ElementMarks, tied_to_next: bool) -> String {
    let mut text: String = String::new();
    text.push_str(match marks.accent() {
        AccentLevel::Normal => "",
        AccentLevel::Accent => ">",
        AccentLevel::Marcato => ">>",
        AccentLevel::Soft => "<",
        AccentLevel::Softer => "<<",
    });
    if marks.staccato() {
        text.push('\'');
    }
    if marks.legato() {
        text.push('_');
    }
    if marks.fermata() {
        text.push('^');
    }
    if tied_to_next {
        text.push('~');
    }
    text
}

/// `[1stInversion]`'s ordinal: 1st, 2nd, 3rd, 4th.
fn ordinal(number: u8) -> String {
    let suffix: &str = match number {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    };
    format!("{number}{suffix}")
}

/// A note as the line writes it: `C2`, or `P5` from the tonic.
fn note_text(pitch: Pitch, spelling: NoteSpelling) -> Result<String, LineTextError> {
    match spelling {
        NoteSpelling::NoteNames => Ok(pitch.to_string()),
        NoteSpelling::Intervals { tonic } => Interval::between(tonic, pitch)
            .map(|interval: Interval| interval.to_string())
            .map_err(|_| LineTextError::NoteNotWritable { pitch, tonic }),
    }
}

/// What an element's contents are as the song writes them before the colon.
///
/// # Errors
///
/// When a note is no interval the notation writes from the tonic.
pub fn contents_text(
    contents: &ElementContents,
    spelling: NoteSpelling,
) -> Result<String, LineTextError> {
    let notes_text = |notes: &[Pitch]| -> Result<String, LineTextError> {
        let names: Vec<String> = notes
            .iter()
            .map(|pitch: &Pitch| note_text(*pitch, spelling))
            .collect::<Result<Vec<String>, LineTextError>>()?;
        Ok(format!("[{}]", names.join(" ")))
    };
    Ok(match contents {
        ElementContents::Chord(chord) => chord.to_string(),
        ElementContents::Notes(notes) => match notes.as_slice() {
            [single] => note_text(*single, spelling)?,
            many => notes_text(many)?,
        },
        ElementContents::ChordWithNotes { chord, notes } => {
            format!("{chord}{}", notes_text(notes)?)
        }
        ElementContents::InvertedChord { chord, inversion } => {
            format!("{chord}[{}Inversion]", ordinal(inversion.number()))
        }
        ElementContents::Dynamic(mark) => mark.to_string(),
        ElementContents::Cue(name) => format!("\"{name}\""),
    })
}

/// The octave of the tonic a pitched line is measured from, for a line that
/// has none yet: the tonic at or below the lowest note, or the one above
/// when that one is a fifth or more below it, so the intervals stay small.
#[must_use]
pub fn tonic_octave_for(tonic: vocabulary::pitch::NoteName, lowest: Pitch) -> i8 {
    // 7 semitones: a perfect fifth.
    const FIFTH_SEMITONES: i16 = 7;
    let mut octave: i8 = lowest.octave().saturating_add(1);
    while octave > -1
        && Pitch::new(tonic, octave).semitones_above_c_minus_1()
            > lowest.semitones_above_c_minus_1()
    {
        octave -= 1;
    }
    if lowest.semitones_above_c_minus_1() - Pitch::new(tonic, octave).semitones_above_c_minus_1()
        >= FIFTH_SEMITONES
    {
        octave = octave.saturating_add(1);
    }
    octave
}

/// Where the next element's length need not be written: the length
/// carried from the element before, if any.
struct Carry {
    length_ticks: Option<u64>,
    /// What the last sounding element played, so `/` can repeat it.
    last_sound: Option<WrittenSound>,
    octave_shift_octaves: i8,
}

/// One bar's text, without the barlines: `C2:h r:q G2`.
fn bar_text(
    bar_number: usize,
    bar: &WrittenBar,
    carry: &mut Carry,
    spelling: NoteSpelling,
) -> Result<String, LineTextError> {
    let mut words: Vec<String> = Vec::new();
    let mut index: usize = 0;
    while index < bar.elements.len() {
        let element: &WrittenElement = &bar.elements[index];
        if element.octave_shift_octaves != carry.octave_shift_octaves {
            words.push(
                match element.octave_shift_octaves {
                    1 => "+8va",
                    0 => "loco",
                    -1 => "-8va",
                    octaves => {
                        return Err(LineTextError::OctaveShiftNotWritable {
                            bar_number,
                            octaves,
                        });
                    }
                }
                .to_string(),
            );
            carry.octave_shift_octaves = element.octave_shift_octaves;
            // `/` repeats what the element before did, octave included, so
            // it is not used across a change of octave.
            carry.last_sound = None;
        }
        if NoteValue::from_ticks(element.length_ticks).is_some() {
            words.push(single_text(element, carry, spelling)?);
            index += 1;
        } else {
            let (word, used): (String, usize) =
                triplet_text(bar_number, &bar.elements[index..], spelling)?;
            words.push(word);
            index += used;
            // After a group the carried length is the one before it, and
            // `/` would play the whole group again: write the next element
            // in full.
            carry.length_ticks = None;
            carry.last_sound = None;
        }
    }
    Ok(words.join(" "))
}

/// An element with a plain or dotted value.
fn single_text(
    element: &WrittenElement,
    carry: &mut Carry,
    spelling: NoteSpelling,
) -> Result<String, LineTextError> {
    let length_written: bool = carry.length_ticks != Some(element.length_ticks)
        || element.sound == WrittenSound::FromSource;
    let length: String = if length_written {
        NoteValue::from_ticks(element.length_ticks).map_or_else(String::new, value_text)
    } else {
        String::new()
    };
    carry.length_ticks = Some(element.length_ticks);
    let marks: String = marks_text(element.marks, element.tied_to_next);
    let colon_length: String = if length.is_empty() {
        String::new()
    } else {
        format!(":{length}")
    };
    Ok(match &element.sound {
        WrittenSound::Rest => format!("r{colon_length}{marks}"),
        WrittenSound::FromSource => {
            carry.last_sound = Some(WrittenSound::FromSource);
            format!(":{length}{marks}")
        }
        WrittenSound::Plays(contents) => {
            let repeated: bool = carry.last_sound.as_ref() == Some(&element.sound);
            carry.last_sound = Some(element.sound.clone());
            if repeated {
                format!("/{colon_length}{marks}")
            } else {
                format!(
                    "{}{colon_length}{marks}",
                    contents_text(contents, spelling)?
                )
            }
        }
    })
}

/// A triplet: the elements from the start of `elements` whose lengths are
/// two thirds of a written value, closed as soon as they fill a plain
/// value together. Returns the text and how many elements it used.
fn triplet_text(
    bar_number: usize,
    elements: &[WrittenElement],
    spelling: NoteSpelling,
) -> Result<(String, usize), LineTextError> {
    let mut played_ticks: u64 = 0;
    let mut parts: Vec<String> = Vec::new();
    let mut uncoupled: bool = false;
    for (index, element) in elements.iter().enumerate() {
        // A triplet note plays 2/3 of its written value: written = played × 3/2.
        let written: Option<NoteValue> = if (element.length_ticks * 3).is_multiple_of(2) {
            NoteValue::from_ticks(element.length_ticks * 3 / 2)
        } else {
            None
        };
        let Some(written) = written else {
            return Err(LineTextError::LengthNotWritable {
                bar_number,
                length_ticks: element.length_ticks,
            });
        };
        if element.tied_to_next {
            return Err(LineTextError::TieInsideTriplet { bar_number });
        }
        let length: String = value_text(written);
        let marks: String = marks_text(element.marks, false);
        parts.push(match &element.sound {
            WrittenSound::Rest => format!("r:{length}{marks}"),
            WrittenSound::FromSource => {
                uncoupled = true;
                format!("{length}{marks}")
            }
            WrittenSound::Plays(contents) => {
                format!("{}:{length}{marks}", contents_text(contents, spelling)?)
            }
        });
        played_ticks += element.length_ticks;
        if NoteValue::from_ticks(played_ticks).is_some() {
            let opening: &str = if uncoupled { ":trp(" } else { "trp(" };
            return Ok((format!("{opening}{})", parts.join(" ")), index + 1));
        }
    }
    Err(LineTextError::LengthNotWritable {
        bar_number,
        length_ticks: played_ticks,
    })
}

/// The text lines of a line: `header` (`line "Keys"`, with its voice
/// layer or `sample_source`) and the bars, four to a text line, the later
/// text lines indented under the first bar.
///
/// # Errors
///
/// When a length, tie or octave annotation cannot be written.
pub fn line_text(
    indent: &str,
    header: &str,
    bars: &[WrittenBar],
    spelling: NoteSpelling,
) -> Result<Vec<String>, LineTextError> {
    let mut carry: Carry = Carry {
        length_ticks: None,
        last_sound: None,
        octave_shift_octaves: 0,
    };
    let mut bar_texts: Vec<String> = Vec::with_capacity(bars.len());
    for (index, bar) in bars.iter().enumerate() {
        bar_texts.push(bar_text(index + 1, bar, &mut carry, spelling)?);
    }
    Ok(join_bars(indent, header, &bar_texts))
}

/// The text lines of a sample source: what each chord or note is, with no
/// lengths, the chords of a bar sharing it evenly (`| I | IV IV V |`).
#[must_use]
pub fn source_text(indent: &str, header: &str, bars: &[Vec<String>]) -> Vec<String> {
    let bar_texts: Vec<String> = bars.iter().map(|bar: &Vec<String>| bar.join(" ")).collect();
    join_bars(indent, header, &bar_texts)
}

fn join_bars(indent: &str, header: &str, bar_texts: &[String]) -> Vec<String> {
    let mut text_lines: Vec<String> = Vec::new();
    let continuation_indent: String = format!("{indent}{}", " ".repeat(header.chars().count() + 2));
    for (chunk_index, chunk) in bar_texts.chunks(BARS_PER_TEXT_LINE).enumerate() {
        let bars: String = chunk.join(" | ");
        if chunk_index == 0 {
            text_lines.push(format!("{indent}{header}  | {bars} |"));
        } else {
            text_lines.push(format!("{continuation_indent}| {bars} |"));
        }
    }
    if text_lines.is_empty() {
        text_lines.push(format!("{indent}{header}"));
    }
    text_lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use song_text::contents_text::{parse_chord, parse_pitch};

    const QUARTER_TICKS: u64 = 5040;

    fn plays(contents: &str, length_ticks: u64) -> WrittenElement {
        let contents: ElementContents = match parse_pitch(contents) {
            Ok(pitch) => ElementContents::Notes(vec![pitch]),
            Err(_) => ElementContents::Chord(parse_chord(contents).unwrap()),
        };
        WrittenElement {
            sound: WrittenSound::Plays(contents),
            length_ticks,
            marks: ElementMarks::NONE,
            tied_to_next: false,
            octave_shift_octaves: 0,
        }
    }

    #[test]
    fn lengths_carry_and_repeats_use_the_slash() {
        let mut accented: WrittenElement = plays("C4", QUARTER_TICKS);
        accented.marks = ElementMarks::new(AccentLevel::Accent, false, false, false);
        let bar: WrittenBar = WrittenBar {
            elements: vec![
                accented,
                plays("C4", QUARTER_TICKS),
                plays("C4", QUARTER_TICKS),
                plays("C4", QUARTER_TICKS),
            ],
        };
        let lines: Vec<String> =
            line_text("  ", "line \"Beat\"", &[bar], NoteSpelling::NoteNames).unwrap();
        assert_eq!(lines, vec!["  line \"Beat\"  | C4:q> / / / |"]);
    }

    #[test]
    fn triplets_are_grouped() {
        let third_ticks: u64 = QUARTER_TICKS * 2 / 3;
        let bar: WrittenBar = WrittenBar {
            elements: vec![
                plays("I", third_ticks),
                plays("I", third_ticks),
                plays("V", third_ticks),
                plays("V", QUARTER_TICKS * 2),
            ],
        };
        // Three triplet quarters fill a half note; a plain half follows.
        let lines: Vec<String> =
            line_text("", "line \"Keys\"", &[bar], NoteSpelling::NoteNames).unwrap();
        assert_eq!(lines, vec!["line \"Keys\"  | trp(I:q I:q V:q) V:h |"]);
    }
}
