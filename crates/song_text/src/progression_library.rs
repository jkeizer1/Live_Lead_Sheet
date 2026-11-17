//! The progression library file: `progression "Name"` blocks, each with its
//! two `group` names, a `mode`, a `meter` and its bars of Roman numerals
//! (Song Notation, section 7).
//!
//! A library of numerals whose entries include a minor one states its
//! `chord_numeral_standard` once, before the first progression, because
//! `III`, `VI` and `VII` are read differently by the two practices. A letter
//! chord (`Am`) is not built yet and is refused.
//!
//! A file with any mistake does not load, not even partly (EFS-6.3): the
//! result is either the library or all the mistakes found.

use vocabulary::key::Mode;
use vocabulary::numeral_standard::NumeralStandard;
use vocabulary::progression::Progression;
use vocabulary::roman_chord::RomanChord;
use vocabulary::time_signature::TimeSignature;

use crate::contents_text::parse_chord;
use crate::parse_error::ParseError;
use crate::text_line::{keyword_rest, parse_meter, quoted, quoted_prefix, without_comment};

/// The progressions of a file, and the practice its numerals are written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressionLibrary {
    numeral_standard: Option<NumeralStandard>,
    progressions: Vec<Progression>,
}

impl ProgressionLibrary {
    /// What the file declared with `chord_numeral_standard`, if anything.
    #[must_use]
    pub fn numeral_standard(&self) -> Option<NumeralStandard> {
        self.numeral_standard
    }

    #[must_use]
    pub fn progressions(&self) -> &[Progression] {
        &self.progressions
    }
}

/// A progression block while it is being read.
struct OpenBlock {
    name: String,
    name_line: usize,
    groups: Option<(String, String)>,
    mode: Option<Mode>,
    time_signature: Option<TimeSignature>,
    bars: Vec<Vec<RomanChord>>,
    /// How many mistakes were already reported when the block began; more
    /// by its end means it has been reported on and is not checked again.
    errors_at_start: usize,
}

/// Reads a progression library.
///
/// # Errors
///
/// Every mistake found, in line order, each naming the file and line.
pub fn parse_progression_library(
    file_name: &str,
    text: &str,
) -> Result<ProgressionLibrary, Vec<ParseError>> {
    let mut reader: Reader = Reader {
        file_name,
        errors: Vec::new(),
        numeral_standard: None,
        progressions: Vec::new(),
        // The name line of the first minor progression, for the missing
        // standard.
        first_minor: None,
        open: None,
    };
    for (index, raw_line) in text.lines().enumerate() {
        let line: &str = without_comment(raw_line).trim();
        if !line.is_empty() {
            reader.read_line(index + 1, line);
        }
    }
    reader.finish()
}

struct Reader<'a> {
    file_name: &'a str,
    errors: Vec<ParseError>,
    numeral_standard: Option<NumeralStandard>,
    progressions: Vec<Progression>,
    first_minor: Option<(String, usize)>,
    open: Option<OpenBlock>,
}

impl Reader<'_> {
    fn report(&mut self, line_number: usize, message: String) {
        self.errors
            .push(ParseError::new(self.file_name, line_number, message));
    }

    fn read_line(&mut self, line_number: usize, line: &str) {
        let result: Result<(), String> = if let Some(rest) = keyword_rest(line, "progression") {
            self.finish_block();
            self.start_block(line_number, rest)
        } else if let Some(rest) = keyword_rest(line, "chord_numeral_standard") {
            self.read_numeral_standard(rest)
        } else if let Some(block) = self.open.as_mut() {
            read_block_line(block, line)
        } else {
            Err(format!(
                "\"{line}\" is outside a progression. Start one with progression \"Name\""
            ))
        };
        if let Err(message) = result {
            self.report(line_number, message);
        }
    }

    /// `chord_numeral_standard key_dependent`, before the first progression.
    fn read_numeral_standard(&mut self, rest: &str) -> Result<(), String> {
        if self.open.is_some() || !self.progressions.is_empty() {
            return Err(String::from(
                "write chord_numeral_standard at the top of the file, before the first \
                 progression",
            ));
        }
        if self.numeral_standard.is_some() {
            return Err(String::from(
                "a second chord_numeral_standard line. A library has one",
            ));
        }
        let standard: NumeralStandard = NumeralStandard::from_text(rest).ok_or_else(|| {
            format!(
                "\"{rest}\" is not a chord numeral standard. Write chord_numeral_standard \
                 key_dependent or chord_numeral_standard major_referential"
            )
        })?;
        self.numeral_standard = Some(standard);
        Ok(())
    }

    fn start_block(&mut self, line_number: usize, rest: &str) -> Result<(), String> {
        let name: &str = quoted(rest).ok_or_else(|| {
            String::from("write the progression's name in quotes, e.g. progression \"ii-V\"")
        })?;
        if self
            .progressions
            .iter()
            .any(|progression: &Progression| progression.name() == name)
        {
            self.report(
                line_number,
                format!(
                    "progression \"{name}\" is already in this file. Give each progression its \
                     own name"
                ),
            );
        }
        self.open = Some(OpenBlock {
            name: name.to_string(),
            name_line: line_number,
            groups: None,
            mode: None,
            time_signature: None,
            bars: Vec::new(),
            errors_at_start: self.errors.len(),
        });
        Ok(())
    }

    /// Builds the block that was open, unless it was already reported on.
    fn finish_block(&mut self) {
        let Some(block) = self.open.take() else {
            return;
        };
        if self.errors.len() > block.errors_at_start {
            return;
        }
        let missing = |what: &str| {
            format!(
                "progression \"{}\" has no {what}. Write it on a line of the progression",
                block.name
            )
        };
        let (Some((idiom, kind)), Some(mode), Some(time_signature)) =
            (&block.groups, block.mode, block.time_signature)
        else {
            let message: String = if block.groups.is_none() {
                missing("group, e.g. group \"Blues\" \"Full form\"")
            } else if block.mode.is_none() {
                missing("mode: write mode major or mode minor")
            } else {
                missing("meter, e.g. meter 4/4")
            };
            self.report(block.name_line, message);
            return;
        };
        if mode == Mode::Minor && self.first_minor.is_none() {
            self.first_minor = Some((block.name.clone(), block.name_line));
        }
        match Progression::new(
            &block.name,
            idiom,
            kind,
            mode,
            time_signature,
            block.bars.clone(),
        ) {
            Ok(progression) => self.progressions.push(progression),
            Err(error) => self.report(block.name_line, format!("\"{}\": {error}", block.name)),
        }
    }

    fn finish(mut self) -> Result<ProgressionLibrary, Vec<ParseError>> {
        self.finish_block();
        if self.numeral_standard.is_none()
            && let Some((name, line_number)) = self.first_minor.clone()
        {
            self.report(
                line_number,
                format!(
                    "progression \"{name}\" is minor, so the file must say how its numerals are \
                     counted. Add chord_numeral_standard key_dependent or \
                     chord_numeral_standard major_referential at the top of the file"
                ),
            );
        }
        if self.errors.is_empty() {
            Ok(ProgressionLibrary {
                numeral_standard: self.numeral_standard,
                progressions: self.progressions,
            })
        } else {
            self.errors.sort_by_key(ParseError::line_number);
            Err(self.errors)
        }
    }
}

/// Reads one line inside a progression block: its groups, mode, meter or
/// bars.
fn read_block_line(block: &mut OpenBlock, line: &str) -> Result<(), String> {
    if let Some(rest) = keyword_rest(line, "group") {
        let wrong = || {
            String::from("write the two group names in quotes, e.g. group \"Blues\" \"Full form\"")
        };
        let (idiom, after): (&str, &str) = quoted_prefix(rest).ok_or_else(wrong)?;
        let kind: &str = quoted(after).ok_or_else(wrong)?;
        block.groups = Some((idiom.to_string(), kind.to_string()));
        Ok(())
    } else if let Some(rest) = keyword_rest(line, "mode") {
        block.mode = Some(parse_mode(rest)?);
        Ok(())
    } else if let Some(rest) = keyword_rest(line, "meter") {
        block.time_signature = Some(parse_meter(rest)?);
        Ok(())
    } else if line.starts_with('|') {
        let first_bar_number: usize = block.bars.len() + 1;
        block
            .bars
            .extend(parse_bars_of_numerals(line, first_bar_number)?);
        Ok(())
    } else {
        Err(format!(
            "\"{line}\" is not a progression line. A progression holds group, mode, meter and \
             bars"
        ))
    }
}

/// `major` or `minor`: which kind of key the numerals are written for.
fn parse_mode(text: &str) -> Result<Mode, String> {
    match text {
        "major" => Ok(Mode::Major),
        "minor" => Ok(Mode::Minor),
        other => Err(format!(
            "\"{other}\" is not a mode a progression can have. Write mode major or mode minor; \
             a mode such as dorian is an annotation"
        )),
    }
}

/// `| I7 | IV7 | I7 V7 |` → the chords of each bar.
fn parse_bars_of_numerals(
    line: &str,
    first_bar_number: usize,
) -> Result<Vec<Vec<RomanChord>>, String> {
    let inside: &str = line
        .strip_prefix('|')
        .and_then(|rest: &str| rest.trim_end().strip_suffix('|'))
        .ok_or_else(|| String::from("end the bars with | , e.g. | I7 | IV7 |"))?;
    let mut bars: Vec<Vec<RomanChord>> = Vec::new();
    for (offset, bar_text) in inside.split('|').enumerate() {
        let bar_number: usize = first_bar_number + offset;
        let mut chords: Vec<RomanChord> = Vec::new();
        for word in bar_text.split_whitespace() {
            // A numeral starts with I, V, i, v or a flat or sharp sign; a
            // letter chord starts with A to G.
            if word.starts_with(|c: char| ('A'..='G').contains(&c)) {
                return Err(format!(
                    "bar {bar_number}: \"{word}\" is a letter chord. Letter chords are not built \
                     yet; write the progression in Roman numerals"
                ));
            }
            chords.push(parse_chord(word).map_err(|why| format!("bar {bar_number}: {why}"))?);
        }
        if chords.is_empty() {
            return Err(format!(
                "bar {bar_number} has no chord. Write at least one chord in it"
            ));
        }
        bars.push(chords);
    }
    Ok(bars)
}
