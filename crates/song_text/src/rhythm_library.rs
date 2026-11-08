//! The rhythm library file: `rhythm "Name"` blocks, each with a `group`, a
//! `meter` and its bars (Song Notation, sections 2, 3 and 7).
//!
//! A file with any mistake does not load, not even partly (EFS-6.3): the
//! result is either all the entries or all the mistakes found.

use vocabulary::rhythm_pattern::{ElementMarks, RhythmElement, RhythmPattern, RhythmPatternError};
use vocabulary::score_length::ScoreLength;
use vocabulary::time_signature::TimeSignature;

use crate::length_words::describe_place;
use crate::parse_error::ParseError;
use crate::rhythm_bar::{BarElement, BarNotation, LengthCarry, parse_bars};
use crate::text_line::{keyword_rest, parse_meter, quoted, without_comment};

/// A rhythm pattern with the group it is listed under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RhythmLibraryEntry {
    group: String,
    pattern: RhythmPattern,
}

impl RhythmLibraryEntry {
    /// The genre or family, e.g. "Swing". Rhythms are listed first by
    /// meter, then by this.
    #[must_use]
    pub fn group(&self) -> &str {
        &self.group
    }

    #[must_use]
    pub fn pattern(&self) -> &RhythmPattern {
        &self.pattern
    }
}

/// A rhythm block while it is being read.
struct OpenBlock {
    name: String,
    name_line: usize,
    group: Option<String>,
    time_signature: Option<TimeSignature>,
    /// A meter line was written, right or wrong.
    meter_written: bool,
    bars: Vec<Vec<BarElement>>,
    /// How many mistakes were already reported when the block began; more
    /// by its end means it has been reported on and is not checked again.
    errors_at_start: usize,
}

/// Reads a rhythm library.
///
/// # Errors
///
/// Every mistake found, in line order, each naming the file and line.
pub fn parse_rhythm_library(
    file_name: &str,
    text: &str,
) -> Result<Vec<RhythmLibraryEntry>, Vec<ParseError>> {
    let mut entries: Vec<RhythmLibraryEntry> = Vec::new();
    let mut errors: Vec<ParseError> = Vec::new();
    let mut open: Option<OpenBlock> = None;

    for (index, raw_line) in text.lines().enumerate() {
        let line_number: usize = index + 1;
        let line: &str = without_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = keyword_rest(line, "rhythm") {
            finish_block(file_name, open.take(), &mut entries, &mut errors);
            match quoted(rest) {
                Some(name) => {
                    if entries.iter().any(|entry| entry.pattern().name() == name) {
                        report(
                            &mut errors,
                            file_name,
                            line_number,
                            format!(
                                "rhythm \"{name}\" is already in this file. Give each rhythm its \
                             own name"
                            ),
                        );
                    }
                    open = Some(OpenBlock {
                        name: name.to_string(),
                        name_line: line_number,
                        group: None,
                        time_signature: None,
                        meter_written: false,
                        bars: Vec::new(),
                        errors_at_start: errors.len(),
                    });
                }
                None => report(
                    &mut errors,
                    file_name,
                    line_number,
                    String::from("write the rhythm's name in quotes, e.g. rhythm \"Charleston\""),
                ),
            }
            continue;
        }

        let Some(block) = open.as_mut() else {
            report(
                &mut errors,
                file_name,
                line_number,
                format!("\"{line}\" is outside a rhythm. Start one with rhythm \"Name\""),
            );
            continue;
        };

        if let Err(message) = read_block_line(block, line) {
            report(&mut errors, file_name, line_number, message);
        }
    }
    finish_block(file_name, open.take(), &mut entries, &mut errors);

    if errors.is_empty() {
        Ok(entries)
    } else {
        Err(errors)
    }
}

/// Reads one line inside a rhythm block: its group, its meter, or bars.
fn read_block_line(block: &mut OpenBlock, line: &str) -> Result<(), String> {
    if let Some(rest) = keyword_rest(line, "group") {
        let group: &str = quoted(rest)
            .ok_or_else(|| String::from("write the group in quotes, e.g. group \"Swing\""))?;
        block.group = Some(group.to_string());
    } else if let Some(rest) = keyword_rest(line, "meter") {
        block.meter_written = true;
        block.time_signature = Some(parse_meter(rest)?);
    } else if line.starts_with('|') {
        let Some(time_signature) = block.time_signature else {
            if block.meter_written {
                return Ok(()); // the meter line was already reported
            }
            return Err(String::from(
                "the bars come before the meter. Write the meter first, e.g. meter 4/4",
            ));
        };
        let bars: Vec<Vec<BarElement>> = parse_bars(
            line,
            block.bars.len(),
            time_signature.bar_length().ticks(),
            BarNotation::Rhythm,
            // A rhythm writes every length; nothing is carried.
            &mut LengthCarry::start_of_line(),
        )?;
        block.bars.extend(bars);
    } else {
        return Err(format!(
            "\"{line}\" is not understood in a rhythm. Expected group \"...\", \
             meter n/d, or bars between |"
        ));
    }
    Ok(())
}

/// Records a mistake found on one line.
fn report(errors: &mut Vec<ParseError>, file_name: &str, line_number: usize, message: String) {
    errors.push(ParseError::new(file_name, line_number, message));
}

/// Checks a finished block and turns it into an entry, or reports why not.
fn finish_block(
    file_name: &str,
    block: Option<OpenBlock>,
    entries: &mut Vec<RhythmLibraryEntry>,
    errors: &mut Vec<ParseError>,
) {
    let Some(block) = block else {
        return;
    };
    if errors.len() > block.errors_at_start {
        return;
    }
    let mut fail = |message: String| {
        errors.push(ParseError::new(file_name, block.name_line, message));
    };
    let Some(group) = block.group.clone() else {
        fail(format!(
            "rhythm \"{}\" has no group. Add a line such as group \"Swing\"",
            block.name
        ));
        return;
    };
    let Some(time_signature) = block.time_signature else {
        fail(format!(
            "rhythm \"{}\" has no meter. Add a line such as meter 4/4",
            block.name
        ));
        return;
    };
    let Ok(bar_count) = u8::try_from(block.bars.len()) else {
        fail(format!(
            "rhythm \"{}\" has too many bars. Use 1 to 8",
            block.name
        ));
        return;
    };

    let bar_length_ticks: u64 = time_signature.bar_length().ticks();
    let mut elements: Vec<RhythmElement> = Vec::new();
    for (bar_index, bar) in block.bars.iter().enumerate() {
        // bar index × ticks/bar → ticks. Bars are at most 8, so no overflow.
        let bar_start_ticks: u64 = bar_index as u64 * bar_length_ticks;
        for placed in bar {
            match RhythmElement::new(
                ScoreLength::from_ticks(bar_start_ticks + placed.start_ticks),
                ScoreLength::from_ticks(placed.length_ticks),
                placed.kind,
                ElementMarks::new(
                    placed.marks.accent(),
                    placed.marks.staccato(),
                    placed.marks.legato(),
                    placed.marks.fermata(),
                ),
                placed.tied_to_next,
            ) {
                Ok(element) => elements.push(element),
                Err(error) => {
                    fail(error.to_string());
                    return;
                }
            }
        }
    }
    match RhythmPattern::new(&block.name, time_signature, bar_count, elements) {
        Ok(pattern) => entries.push(RhythmLibraryEntry { group, pattern }),
        Err(RhythmPatternError::TieWithoutNote { start_ticks, .. }) => fail(format!(
            "rhythm \"{}\": the tie at {} has no note after it to join. \
             Remove the ~ or put a note after it",
            block.name,
            describe_place(start_ticks, time_signature)
        )),
        Err(error) => fail(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vocabulary::rhythm_pattern::ElementKind;

    const GOOD: &str = r#"
# Starter rhythms
rhythm "Charleston"
  group "Swing"
  meter 4/4
  | q. e hr |

rhythm "Anticipation"
  group "Swing"
  meter 4/4
  | hr qr er e~ | q hr. |

rhythm "Backbeat"
  group "Rock"
  meter 4/4
  | q q> q q> |
"#;

    #[test]
    fn a_good_file_loads_every_entry() {
        let entries: Vec<RhythmLibraryEntry> = parse_rhythm_library("good.library", GOOD).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].group(), "Swing");
        assert_eq!(entries[1].pattern().bar_count(), 2);
        // The eighth before the barline is tied into the quarter after it.
        let tied: &RhythmElement = &entries[1].pattern().elements()[3];
        assert!(tied.tied_to_next());
        assert_eq!(tied.start().ticks(), 17_640);
        assert_eq!(
            entries[1].pattern().elements()[4].kind(),
            ElementKind::Sounding
        );
    }

    #[test]
    fn mistakes_are_all_reported_and_nothing_loads() {
        let text: &str = "rhythm \"Short\"\n  group \"Swing\"\n  meter 4/4\n  | q q q e |\n\
                          rhythm \"No group\"\n  meter 4/4\n  | w |\n";
        let errors: Vec<ParseError> = parse_rhythm_library("bad.library", text).unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors[0].to_string(),
            "bad.library line 4: bar 1 is short by an eighth note. Add e or er."
        );
        assert_eq!(errors[1].line_number(), 5);
        assert!(errors[1].message().contains("has no group"));
    }

    #[test]
    fn a_tie_with_nothing_to_join_is_reported_at_the_rhythm() {
        let text: &str = "rhythm \"Dangling\"\n  group \"Swing\"\n  meter 4/4\n  | hr qr er e~ |\n";
        let errors: Vec<ParseError> = parse_rhythm_library("tie.library", text).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message()
                .contains("the tie at bar 1 beat 4 has no note")
        );
    }

    #[test]
    fn a_hash_inside_a_name_is_not_a_comment() {
        let text: &str =
            "rhythm \"F# shuffle\"  # a comment\n  group \"Swing\"\n  meter 4/4\n  | w |\n";
        let entries: Vec<RhythmLibraryEntry> = parse_rhythm_library("hash.library", text).unwrap();
        assert_eq!(entries[0].pattern().name(), "F# shuffle");
    }

    #[test]
    fn lines_outside_a_rhythm_and_bad_meters_are_reported() {
        let text: &str = "| q q q q |\nrhythm \"X\"\n  group \"G\"\n  meter 4/3\n  | w |\n";
        let errors: Vec<ParseError> = parse_rhythm_library("odd.library", text).unwrap_err();
        assert!(errors[0].message().contains("outside a rhythm"));
        assert!(errors[1].message().contains("not a time signature"));
    }
}
