//! One bar of rhythm notation: note values, rests, dots, marks, ties and
//! the groups `trp` and `fit` (Song Notation, section 3), in either of its
//! two spellings: a rhythm library's, note value first (`q. e hr`), or a
//! song line's, what it plays first (`C4:q. D4:e r:h`, Song Notation,
//! section 4), where a length left off is the one before it in the line.
//!
//! Positions are worked out in subticks, eighths of a tick, so that a
//! thirty-second note with two dots (1102.5 ticks) adds up exactly and a
//! bar can be checked to the subtick. A subtick exists only inside this
//! module; what leaves it is whole ticks. An element's place inside a group
//! is worked out from the start of the group, never by adding rounded
//! steps, and rounded once, to the nearest tick (half up), so nothing
//! drifts.

use vocabulary::rhythm_pattern::{AccentLevel, ElementKind, ElementMarks};
use vocabulary::ticks::{TICKS_PER_QUARTER_NOTE, TICKS_PER_WHOLE_NOTE};

use crate::contents_text::parse_chord;
use crate::length_words::{MAX_DOTS, describe_length, dotted_length_subticks};

const SUBTICKS_PER_TICK: u64 = 8;

/// An element placed in its bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarElement {
    /// Ticks from the start of the bar.
    pub start_ticks: u64,
    pub length_ticks: u64,
    pub kind: ElementKind,
    pub marks: ElementMarks,
    pub tied_to_next: bool,
    /// What the element plays, exactly as written (`C4`, `ii7`,
    /// `[D3 F3 A3]`, `"Start SQ-1"`), for the song parser to read. `None`
    /// for a rest, for every element of a rhythm library, and for `/` in a
    /// song line, which plays what the sounding element before it played.
    pub contents_text: Option<String>,
    /// True for an element written with nothing before its colon (`:q`): it
    /// takes what it plays from the track's `sample_source` line. Such an
    /// element has no `contents_text`, like `/`, but plays nothing of its own
    /// earlier in the line.
    pub from_source: bool,
}

/// Which of the two spellings a bar is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarNotation {
    /// A rhythm library's: note value, then `r` and dots, then marks and
    /// tie (`q.`, `hr`, `e>~`). Nothing is played, so nothing is written.
    Rhythm,
    /// A song line's: what it plays (a pitch, chord, notes, cue name, `r`
    /// for a rest or `/` for the same again), then its length, then marks
    /// and tie (`Eb4:sx`, `C5e.>`, `I:q`, `r:h`, `/`).
    SongLine,
}

/// A note value with its dots, as written: what a song line's element
/// takes when it writes no length of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WrittenLength {
    base_ticks: u64,
    dots: u32,
}

/// The length a song line carries forward: the last one written, dots
/// included, across barlines and the text lines a line goes on over. In a
/// group it is the written value, before the group squeezes it, so the
/// element after `trp (C4:q D4 E4)` is a plain quarter (Song Notation,
/// section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthCarry {
    written: Option<WrittenLength>,
}

impl LengthCarry {
    /// Nothing written yet: the first element of a line writes its length.
    #[must_use]
    pub fn start_of_line() -> LengthCarry {
        LengthCarry { written: None }
    }
}

/// An element as written, before it is placed.
#[derive(Debug, Clone)]
struct WrittenElement {
    length_subticks: u64,
    kind: ElementKind,
    marks: ElementMarks,
    tied_to_next: bool,
    contents_text: Option<String>,
    from_source: bool,
}

/// What a group does to the natural length of what is inside it.
#[derive(Debug, Clone, Copy)]
enum GroupLength {
    /// `trp`: three in the time of two, so 2/3 of the written lengths.
    TwoThirds,
    /// `fitQ(...)` and the others: exactly this many ticks.
    Fixed { length_ticks: u64 },
}

/// Parses the text between two barlines, in `notation`. A song line's
/// carried length comes in and goes out through `carry`.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead; the caller adds
/// the file and line. Also when the elements do not add up exactly to
/// `bar_length_ticks`.
pub fn parse_bar(
    bar_text: &str,
    bar_number: usize,
    bar_length_ticks: u64,
    notation: BarNotation,
    carry: &mut LengthCarry,
) -> Result<Vec<BarElement>, String> {
    let mut cursor: Cursor = Cursor::new(bar_text);
    let mut placed: Vec<BarElement> = Vec::new();
    let mut position_subticks: u64 = 0;

    loop {
        cursor.skip_whitespace();
        if cursor.peek().is_none() {
            break;
        }
        if cursor.starts_with("trp") || cursor.starts_with("fit") {
            let (group_length, inner): (GroupLength, Vec<WrittenElement>) =
                parse_group(&mut cursor, bar_length_ticks, notation, carry)?;
            let natural_subticks: u64 = inner.iter().map(|element| element.length_subticks).sum();
            let group_subticks: u64 = match group_length {
                GroupLength::TwoThirds => {
                    if !(natural_subticks * 2).is_multiple_of(3) {
                        return Err(format!(
                            "bar {bar_number}: this trp group cannot be divided into whole \
                             parts. Use fitH(...) or fitQ(...) to say how long it should be"
                        ));
                    }
                    natural_subticks * 2 / 3
                }
                GroupLength::Fixed { length_ticks } => length_ticks * SUBTICKS_PER_TICK,
            };
            placed.extend(place(
                position_subticks,
                group_subticks,
                &inner,
                bar_number,
            )?);
            position_subticks += group_subticks;
        } else {
            let element: WrittenElement = parse_element(&mut cursor, notation, carry)?;
            let element_subticks: u64 = element.length_subticks;
            placed.extend(place(
                position_subticks,
                element_subticks,
                &[element],
                bar_number,
            )?);
            position_subticks += element_subticks;
        }
    }

    if placed.is_empty() {
        let example: &str = match notation {
            BarNotation::Rhythm => "q q q q",
            BarNotation::SongLine => "C4:q D4 E4 r",
        };
        return Err(format!(
            "bar {bar_number} is empty. Write its notes and rests, e.g. {example}"
        ));
    }
    let bar_subticks: u64 = bar_length_ticks * SUBTICKS_PER_TICK;
    if position_subticks < bar_subticks {
        let short_ticks: u64 = (bar_subticks - position_subticks).div_ceil(SUBTICKS_PER_TICK);
        let words = describe_length(short_ticks);
        let advice: String = match (words.letters, notation) {
            (Some(letters), BarNotation::Rhythm) => format!(" Add {letters} or {letters}r."),
            (Some(letters), BarNotation::SongLine) => {
                format!(" Add a note or a rest of that length, e.g. r:{letters}.")
            }
            (None, _) => String::from(" Add a note or rest to fill the bar."),
        };
        return Err(format!(
            "bar {bar_number} is short by {}.{advice}",
            words.description
        ));
    }
    if position_subticks > bar_subticks {
        let over_ticks: u64 = (position_subticks - bar_subticks).div_ceil(SUBTICKS_PER_TICK);
        return Err(format!(
            "bar {bar_number} is over by {}. Shorten or remove an element.",
            describe_length(over_ticks).description
        ));
    }
    Ok(placed)
}

/// `| q q q q | q q q q |` → the bars between the barlines. `bars_so_far`
/// is how many bars came before, so a mistake names its bar in the whole line.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead.
pub fn parse_bars(
    line: &str,
    bars_so_far: usize,
    bar_length_ticks: u64,
    notation: BarNotation,
    carry: &mut LengthCarry,
) -> Result<Vec<Vec<BarElement>>, String> {
    let line: &str = line.trim();
    if !line.starts_with('|') || !line.ends_with('|') || line.len() < 2 {
        let example: &str = match notation {
            BarNotation::Rhythm => "| q q q q |",
            BarNotation::SongLine => "| C4:q D4 E4 F4 |",
        };
        return Err(format!(
            "a line of bars starts and ends with |, e.g. {example}"
        ));
    }
    let inside: &str = &line[1..line.len() - 1];
    let mut bars: Vec<Vec<BarElement>> = Vec::new();
    for (offset, bar_text) in split_at_barlines(inside).into_iter().enumerate() {
        let bar_number: usize = bars_so_far + offset + 1;
        bars.push(parse_bar(
            bar_text,
            bar_number,
            bar_length_ticks,
            notation,
            carry,
        )?);
    }
    Ok(bars)
}

/// The bars of a `sample_source` line: chords or notes with no lengths, which
/// share each bar evenly (Song Notation, section 5). `IV IV V` in a 4/4 bar
/// puts each chord on a third of the bar, 6720 ticks.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead: an empty bar, a
/// rest, or a length, mark or tie on an element.
pub fn parse_source_bars(
    line: &str,
    bars_so_far: usize,
    bar_length_ticks: u64,
) -> Result<Vec<Vec<BarElement>>, String> {
    let line: &str = line.trim();
    if !line.starts_with('|') || !line.ends_with('|') || line.len() < 2 {
        return Err(String::from(
            "a line of bars starts and ends with |, e.g. | I | IV IV V |",
        ));
    }
    let inside: &str = &line[1..line.len() - 1];
    let mut bars: Vec<Vec<BarElement>> = Vec::new();
    for (offset, bar_text) in split_at_barlines(inside).into_iter().enumerate() {
        let bar_number: usize = bars_so_far + offset + 1;
        bars.push(parse_source_bar(bar_text, bar_number, bar_length_ticks)?);
    }
    Ok(bars)
}

fn parse_source_bar(
    bar_text: &str,
    bar_number: usize,
    bar_length_ticks: u64,
) -> Result<Vec<BarElement>, String> {
    let mut cursor: Cursor = Cursor::new(bar_text);
    let mut contents: Vec<Option<String>> = Vec::new();
    loop {
        cursor.skip_whitespace();
        if cursor.peek().is_none() {
            break;
        }
        let element_text: String = cursor.element_text();
        match parse_song_contents(&mut cursor, &element_text)? {
            WrittenContents::Rest => {
                return Err(format!(
                    "bar {bar_number}: a sample_source line holds chords or notes, not rests. \
                     Put the rests on the rhythm line below it, e.g. r:q"
                ));
            }
            WrittenContents::FromSource => {
                return Err(format!(
                    "bar {bar_number}: \"{element_text}\" has nothing to play. A sample_source \
                     line holds chords or notes, e.g. I or [C3 E3 G3]"
                ));
            }
            WrittenContents::Same => contents.push(None),
            WrittenContents::Text(text) => contents.push(Some(text)),
        }
        match cursor.peek() {
            None => {}
            Some(character) if character.is_whitespace() => {}
            Some(_) => {
                return Err(format!(
                    "bar {bar_number}: \"{element_text}\": a sample_source line has no lengths, \
                     marks or ties. Write the chord or note alone, and its length on the rhythm \
                     line below it, e.g. :q"
                ));
            }
        }
    }
    let count: u64 = contents.len() as u64;
    if count == 0 {
        return Err(format!(
            "bar {bar_number} is empty. Write at least one chord or note in it, e.g. | I |"
        ));
    }
    // The place of element i is i × bar ÷ count, rounded half up, so no
    // rounded step is added to another: ticks × (elements ÷ elements) → ticks.
    let tick_of = |index: u64| -> u64 { (2 * index * bar_length_ticks + count) / (2 * count) };
    let mut placed: Vec<BarElement> = Vec::new();
    for (index, text) in contents.into_iter().enumerate() {
        let start_ticks: u64 = tick_of(index as u64);
        let end_ticks: u64 = tick_of(index as u64 + 1);
        if end_ticks <= start_ticks {
            return Err(format!(
                "bar {bar_number} has too many elements for the bar. Write fewer"
            ));
        }
        placed.push(BarElement {
            start_ticks,
            length_ticks: end_ticks - start_ticks,
            kind: ElementKind::Sounding,
            marks: ElementMarks::NONE,
            tied_to_next: false,
            contents_text: text,
            from_source: false,
        });
    }
    Ok(placed)
}

/// Splits at `|`, except inside a quoted cue name.
fn split_at_barlines(text: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = Vec::new();
    let mut inside_quotes: bool = false;
    let mut part_start: usize = 0;
    for (position, character) in text.char_indices() {
        match character {
            '"' => inside_quotes = !inside_quotes,
            '|' if !inside_quotes => {
                parts.push(&text[part_start..position]);
                part_start = position + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[part_start..]);
    parts
}

/// Places the written elements of a group (or one plain element) between
/// `start_subticks` and `start_subticks + length_subticks`, stretching or
/// squeezing them in proportion to their written lengths.
///
/// An element's start is `group start + (written start ÷ written total) ×
/// group length`, in subticks, rounded once to ticks.
fn place(
    start_subticks: u64,
    length_subticks: u64,
    written: &[WrittenElement],
    bar_number: usize,
) -> Result<Vec<BarElement>, String> {
    let written_total_subticks: u64 = written.iter().map(|element| element.length_subticks).sum();
    let tick_of = |written_offset_subticks: u64| -> u64 {
        // (start × total + offset × length) subticks·subticks ÷
        // (8 subticks/tick × total subticks) → ticks, rounded half up.
        let numerator: u128 = u128::from(start_subticks) * u128::from(written_total_subticks)
            + u128::from(written_offset_subticks) * u128::from(length_subticks);
        let denominator: u128 = u128::from(SUBTICKS_PER_TICK) * u128::from(written_total_subticks);
        let rounded: u128 = (2 * numerator + denominator) / (2 * denominator);
        // Cannot be large: a bar is at most 255 pulses of at most 20160 ticks.
        u64::try_from(rounded).unwrap_or(u64::MAX)
    };

    let mut result: Vec<BarElement> = Vec::with_capacity(written.len());
    let mut written_offset_subticks: u64 = 0;
    for element in written {
        let start_ticks: u64 = tick_of(written_offset_subticks);
        written_offset_subticks += element.length_subticks;
        let end_ticks: u64 = tick_of(written_offset_subticks);
        if end_ticks <= start_ticks {
            return Err(format!(
                "bar {bar_number}: a group squeezes an element to nothing. \
                 Write fewer elements in it"
            ));
        }
        result.push(BarElement {
            start_ticks,
            length_ticks: end_ticks - start_ticks,
            kind: element.kind,
            marks: element.marks,
            tied_to_next: element.tied_to_next,
            contents_text: element.contents_text.clone(),
            from_source: element.from_source,
        });
    }
    Ok(result)
}

/// `trp (...)`, `fitQ(...)`, `fitE`, `fitH`, `fitW`, `fitBar`.
fn parse_group(
    cursor: &mut Cursor,
    bar_length_ticks: u64,
    notation: BarNotation,
    carry: &mut LengthCarry,
) -> Result<(GroupLength, Vec<WrittenElement>), String> {
    let name: String = cursor.take_while(char::is_ascii_alphabetic);
    let group_length: GroupLength = match name.as_str() {
        "trp" => GroupLength::TwoThirds,
        "fitQ" => GroupLength::Fixed {
            length_ticks: TICKS_PER_QUARTER_NOTE,
        },
        "fitE" => GroupLength::Fixed {
            length_ticks: TICKS_PER_QUARTER_NOTE / 2,
        },
        "fitH" => GroupLength::Fixed {
            length_ticks: TICKS_PER_WHOLE_NOTE / 2,
        },
        "fitW" => GroupLength::Fixed {
            length_ticks: TICKS_PER_WHOLE_NOTE,
        },
        "fitBar" => GroupLength::Fixed {
            length_ticks: bar_length_ticks,
        },
        _ => {
            return Err(format!(
                "\"{name}\" is not a group. Use trp, fitQ, fitE, fitH, fitW or fitBar, \
                 e.g. trp (q q q)"
            ));
        }
    };
    cursor.skip_whitespace();
    if cursor.next() != Some('(') {
        return Err(format!(
            "{name} must be followed by ( and the elements, e.g. {name} (q q q)"
        ));
    }
    let mut inner: Vec<WrittenElement> = Vec::new();
    loop {
        cursor.skip_whitespace();
        match cursor.peek() {
            None => {
                return Err(format!("the group {name} (...) is missing its closing )"));
            }
            Some(')') => {
                cursor.next();
                break;
            }
            Some(_) => {
                if cursor.starts_with("trp") || cursor.starts_with("fit") {
                    return Err(format!(
                        "a group inside a group (in {name}) is not supported. Write the \
                         inner group as separate elements"
                    ));
                }
                inner.push(parse_element(cursor, notation, carry)?);
            }
        }
    }
    if inner.is_empty() {
        return Err(format!(
            "the group {name} () is empty. Put notes or rests in it"
        ));
    }
    match cursor.peek() {
        None => {}
        Some(character) if character.is_whitespace() => {}
        Some(character) => {
            return Err(format!(
                "'{character}' after the group {name} (...). Leave a space after the )"
            ));
        }
    }
    Ok((group_length, inner))
}

/// One element in `notation`.
fn parse_element(
    cursor: &mut Cursor,
    notation: BarNotation,
    carry: &mut LengthCarry,
) -> Result<WrittenElement, String> {
    match notation {
        BarNotation::Rhythm => parse_rhythm_element(cursor),
        BarNotation::SongLine => parse_song_element(cursor, carry),
    }
}

/// A rhythm library's element: value, then `r` and dots in either order
/// (`hr.` and `h.r` are the same dotted half rest), then marks, then a tie.
fn parse_rhythm_element(cursor: &mut Cursor) -> Result<WrittenElement, String> {
    let base_ticks: u64 = parse_note_value(cursor)?;

    let mut is_rest: bool = false;
    let mut dots: u32 = 0;
    loop {
        match cursor.peek() {
            Some('r') if !is_rest => {
                cursor.next();
                is_rest = true;
            }
            Some('.') => {
                cursor.next();
                dots += 1;
                if dots > MAX_DOTS {
                    return Err(format!(
                        "more than {MAX_DOTS} dots on one element. Use a tie"
                    ));
                }
            }
            _ => break,
        }
    }

    let marks: ElementMarks = parse_marks(cursor)?;
    let tied_to_next: bool = cursor.peek() == Some('~');
    if tied_to_next {
        cursor.next();
    }

    match cursor.peek() {
        None | Some(')') => {}
        Some(character) if character.is_whitespace() => {}
        Some(':') => {
            return Err(String::from(
                "':' in a rhythm pattern. A rhythm carries no notes or chords; \
                 they are written in a song's lines",
            ));
        }
        Some(character) => {
            return Err(format!(
                "'{character}' is not expected here. After a note value come r and dots, \
                 then marks (> >> ' _ ^), then ~ for a tie"
            ));
        }
    }

    let kind: ElementKind = if is_rest {
        ElementKind::Rest
    } else {
        ElementKind::Sounding
    };
    check_rest(kind, marks, tied_to_next)?;
    Ok(WrittenElement {
        length_subticks: dotted_length_subticks(base_ticks, dots, SUBTICKS_PER_TICK),
        kind,
        marks,
        tied_to_next,
        contents_text: None,
        from_source: false,
    })
}

/// What a song line's element plays, as written.
enum WrittenContents {
    /// Nothing before the colon (`:q`): what the `sample_source` line holds.
    FromSource,
    /// `r`.
    Rest,
    /// `/`: what the sounding element before it played.
    Same,
    /// A pitch, chord, notes in brackets or a quoted cue name.
    Text(String),
}

/// A song line's element: what it plays, then `:` and its length (the
/// colon may be left off except after a chord name), then marks, then a
/// tie. `C4:q`, `Eb4sx`, `D4~`, `I:q>`, `[C3 E3 G3]h`, `r:h.`, `/>`.
fn parse_song_element(
    cursor: &mut Cursor,
    carry: &mut LengthCarry,
) -> Result<WrittenElement, String> {
    let element_text: String = cursor.element_text();
    if element_text.starts_with(|character: char| "whqest".contains(character)) {
        return Err(format!(
            "\"{element_text}\": a song line writes what it plays first, then its length, \
             e.g. C4:q, I:q or r:q, not q:C4"
        ));
    }
    let contents: WrittenContents = parse_song_contents(cursor, &element_text)?;
    if cursor.peek() == Some(':') {
        cursor.next();
    }
    let written_length: Option<WrittenLength> = parse_written_length(cursor)?;
    let marks: ElementMarks = parse_marks(cursor)?;
    let tied_to_next: bool = cursor.peek() == Some('~');
    if tied_to_next {
        cursor.next();
    }
    match cursor.peek() {
        None | Some(')') => {}
        Some(character) if character.is_whitespace() => {}
        Some('#') => return Err(misplaced_sharp(&element_text)),
        Some(character) => {
            return Err(format!(
                "'{character}' is not expected in \"{element_text}\". Write what it plays, \
                 then its length, then marks (> >> ' _ ^), then ~ for a tie, e.g. C4:q>~"
            ));
        }
    }
    let length: WrittenLength = match written_length {
        Some(written) => {
            carry.written = Some(written);
            written
        }
        None => carry.written.ok_or_else(|| {
            format!(
                "\"{element_text}\" has no length, and nothing before it in this line \
                 gives one. Write it, e.g. {}:q",
                element_text.trim_end_matches(['>', '\'', '_', '^', '~', ':'])
            )
        })?,
    };
    let (kind, contents_text, from_source): (ElementKind, Option<String>, bool) = match contents {
        WrittenContents::Rest => (ElementKind::Rest, None, false),
        WrittenContents::Same => (ElementKind::Sounding, None, false),
        WrittenContents::FromSource => (ElementKind::Sounding, None, true),
        WrittenContents::Text(text) => (ElementKind::Sounding, Some(text), false),
    };
    check_rest(kind, marks, tied_to_next)?;
    Ok(WrittenElement {
        length_subticks: dotted_length_subticks(length.base_ticks, length.dots, SUBTICKS_PER_TICK),
        kind,
        marks,
        tied_to_next,
        contents_text,
        from_source,
    })
}

/// A rest takes only a fermata, and is never tied.
fn check_rest(kind: ElementKind, marks: ElementMarks, tied_to_next: bool) -> Result<(), String> {
    if kind != ElementKind::Rest {
        return Ok(());
    }
    if marks.accent() != AccentLevel::Normal || marks.staccato() || marks.legato() {
        return Err(String::from(
            "a rest takes no accent, staccato or legato mark. Only a fermata (^) \
             may stand on a rest",
        ));
    }
    if tied_to_next {
        return Err(String::from("a rest cannot be tied. Tie notes, not rests"));
    }
    Ok(())
}

/// The start of a song line's element: `r`, `/`, a quoted cue name, notes
/// in brackets, a pitch (`Eb4`: it ends at its octave, so a length may
/// follow with no colon), or a chord name with its voicing if any (`ii7`,
/// `ii7[F3 A3 C4]`). A chord name ends at a space, `:`, `[`, a mark or a
/// tie, so a length after it needs the colon. What the text means is for
/// the song parser to say.
fn parse_song_contents(cursor: &mut Cursor, element_text: &str) -> Result<WrittenContents, String> {
    match cursor.peek() {
        Some('r') => {
            cursor.next();
            Ok(WrittenContents::Rest)
        }
        Some('/') => {
            cursor.next();
            Ok(WrittenContents::Same)
        }
        Some('"') => Ok(WrittenContents::Text(take_quoted(cursor)?)),
        Some('[') => Ok(WrittenContents::Text(take_bracketed(cursor)?)),
        Some(letter) if "ABCDEFG".contains(letter) => {
            let mut text: String = String::new();
            text.push(letter);
            cursor.next();
            if let Some(accidental @ ('#' | 'b')) = cursor.peek() {
                text.push(accidental);
                cursor.next();
            }
            if cursor.peek() == Some('-') {
                text.push('-');
                cursor.next();
            }
            let octave: String = cursor.take_while(char::is_ascii_digit);
            if octave.is_empty() {
                return Err(format!(
                    "\"{element_text}\" is not a note: a note needs its octave, e.g. C4, \
                     F#2 or Bb1"
                ));
            }
            text.push_str(&octave);
            Ok(WrittenContents::Text(text))
        }
        // A sharp before a chord numeral is part of the chord (#iv°); a
        // sharp anywhere else in front of an element is misplaced.
        Some('#')
            if !cursor
                .peek_second()
                .is_some_and(|after: char| "IiVv".contains(after)) =>
        {
            Err(misplaced_sharp(element_text))
        }
        // Nothing before the colon: the element's length is written and what
        // it plays comes from the sample source (Song Notation, section 4).
        Some(':') => Ok(WrittenContents::FromSource),
        Some('>' | '\'' | '_' | '^' | '~') | None => Err(format!(
            "\"{element_text}\" does not say what it plays. Write a note, chord, r or / \
             first, e.g. C4:q"
        )),
        Some(_) => {
            let name: String = cursor.take_while(|character: &char| {
                !character.is_whitespace() && !":[)>'_^~".contains(*character)
            });
            if let Some(advice) = chord_with_length_run_on(&name) {
                return Err(advice);
            }
            let mut text: String = name;
            if cursor.peek() == Some('[') {
                text.push_str(&take_bracketed(cursor)?);
            }
            Ok(WrittenContents::Text(text))
        }
    }
}

/// A chord name with its length run on (`Iq`, `ii7h.`): after a chord
/// name the length needs a colon, because a chord name has no fixed end.
/// The advice, when the text is a chord followed by a length.
fn chord_with_length_run_on(name: &str) -> Option<String> {
    let without_dots: &str = name.trim_end_matches('.');
    for value in ["sx", "tx", "w", "h", "q", "e"] {
        let Some(chord) = without_dots.strip_suffix(value) else {
            continue;
        };
        if !chord.is_empty() && parse_chord(chord).is_ok() {
            let length: &str = &name[chord.len()..];
            return Some(format!(
                "\"{name}\" is not a chord. A chord needs a colon before its length: \
                 write {chord}:{length}"
            ));
        }
    }
    None
}

/// A `#` is always a sharp; one in the wrong place is told where it goes.
fn misplaced_sharp(element_text: &str) -> String {
    format!(
        "\"{element_text}\": a # is a sharp. Write it right after the note's letter, e.g. \
         C#4, or before a chord numeral, e.g. #iv°"
    )
}

/// `"Start SQ-1"`, quotes included.
fn take_quoted(cursor: &mut Cursor) -> Result<String, String> {
    let mut text: String = String::new();
    if cursor.next() != Some('"') {
        return Err(String::from("a quoted name starts with \""));
    }
    text.push('"');
    loop {
        match cursor.next() {
            None => {
                return Err(String::from("a quoted name is missing its closing \""));
            }
            Some('"') => {
                text.push('"');
                return Ok(text);
            }
            Some(character) => text.push(character),
        }
    }
}

/// `[D3 F3 A3]`, brackets included.
fn take_bracketed(cursor: &mut Cursor) -> Result<String, String> {
    let mut text: String = String::new();
    if cursor.next() != Some('[') {
        return Err(String::from("notes in brackets start with ["));
    }
    text.push('[');
    loop {
        match cursor.next() {
            None | Some('|') => return Err(String::from("a '[' is missing its closing ]")),
            Some('[') => {
                return Err(String::from(
                    "'[' inside square brackets. Close the first with ]",
                ));
            }
            Some(']') => {
                text.push(']');
                return Ok(text);
            }
            Some(character) => text.push(character),
        }
    }
}

/// A length if one is written here: a note value and its dots.
fn parse_written_length(cursor: &mut Cursor) -> Result<Option<WrittenLength>, String> {
    let starts_a_length: bool = match cursor.peek() {
        Some('w' | 'h' | 'q' | 'e') => true,
        Some('s' | 't') => cursor.peek_second() == Some('x'),
        _ => false,
    };
    if !starts_a_length {
        return Ok(None);
    }
    let base_ticks: u64 = parse_note_value(cursor)?;
    let mut dots: u32 = 0;
    while cursor.peek() == Some('.') {
        cursor.next();
        dots += 1;
        if dots > MAX_DOTS {
            return Err(format!(
                "more than {MAX_DOTS} dots on one element. Use a tie"
            ));
        }
    }
    Ok(Some(WrittenLength { base_ticks, dots }))
}

/// `w h q e sx tx` → the plain length in ticks.
fn parse_note_value(cursor: &mut Cursor) -> Result<u64, String> {
    match cursor.next() {
        Some('w') => Ok(TICKS_PER_WHOLE_NOTE),
        Some('h') => Ok(TICKS_PER_WHOLE_NOTE / 2),
        Some('q') => Ok(TICKS_PER_QUARTER_NOTE),
        Some('e') => Ok(TICKS_PER_QUARTER_NOTE / 2),
        Some('s') if cursor.peek() == Some('x') => {
            cursor.next();
            Ok(TICKS_PER_QUARTER_NOTE / 4)
        }
        Some('t') if cursor.peek() == Some('x') => {
            cursor.next();
            Ok(TICKS_PER_QUARTER_NOTE / 8)
        }
        Some(other) => Err(format!(
            "'{other}' is not a note value. Use w, h, q, e, sx or tx, \
             e.g. q for a quarter note"
        )),
        None => Err(String::from("a note value is missing")),
    }
}

/// `>` `>>` `'` `_` `^`, each at most once, and only one accent level.
fn parse_marks(cursor: &mut Cursor) -> Result<ElementMarks, String> {
    let mut accent: AccentLevel = AccentLevel::Normal;
    let mut staccato: bool = false;
    let mut legato: bool = false;
    let mut fermata: bool = false;
    loop {
        match cursor.peek() {
            Some('>') => {
                cursor.next();
                let this_level: AccentLevel = if cursor.peek() == Some('>') {
                    cursor.next();
                    AccentLevel::Marcato
                } else {
                    AccentLevel::Accent
                };
                if accent != AccentLevel::Normal {
                    return Err(String::from(
                        "two accent marks on one element. Use > or >>, not both",
                    ));
                }
                accent = this_level;
            }
            Some('\'') => {
                cursor.next();
                staccato = true;
            }
            Some('_') => {
                cursor.next();
                legato = true;
            }
            Some('^') => {
                cursor.next();
                fermata = true;
            }
            _ => break,
        }
    }
    Ok(ElementMarks::new(accent, staccato, legato, fermata))
}

struct Cursor {
    characters: Vec<char>,
    position: usize,
}

impl Cursor {
    fn new(text: &str) -> Cursor {
        Cursor {
            characters: text.chars().collect(),
            position: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.characters.get(self.position).copied()
    }

    fn next(&mut self) -> Option<char> {
        let character: Option<char> = self.peek();
        if character.is_some() {
            self.position += 1;
        }
        character
    }

    fn peek_second(&self) -> Option<char> {
        self.characters.get(self.position + 1).copied()
    }

    /// The element starting here, as written, for messages: up to the next
    /// space or `)`, keeping quoted names and brackets whole. Nothing is
    /// consumed.
    fn element_text(&self) -> String {
        let mut text: String = String::new();
        let mut inside_quotes: bool = false;
        let mut inside_brackets: bool = false;
        for character in &self.characters[self.position..] {
            match character {
                '"' => inside_quotes = !inside_quotes,
                '[' if !inside_quotes => inside_brackets = true,
                ']' if !inside_quotes => inside_brackets = false,
                ')' if !inside_quotes && !inside_brackets => break,
                character if character.is_whitespace() && !inside_quotes && !inside_brackets => {
                    break;
                }
                _ => {}
            }
            text.push(*character);
        }
        text
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.position += 1;
        }
    }

    fn starts_with(&self, text: &str) -> bool {
        text.chars().enumerate().all(|(offset, expected)| {
            self.characters.get(self.position + offset) == Some(&expected)
        })
    }

    fn take_while(&mut self, accept: fn(&char) -> bool) -> String {
        let mut taken: String = String::new();
        while let Some(character) = self.peek() {
            if !accept(&character) {
                break;
            }
            taken.push(character);
            self.position += 1;
        }
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOUR_FOUR_BAR_TICKS: u64 = 20_160;

    fn rhythm(bar_text: &str) -> Result<Vec<BarElement>, String> {
        parse_bar(
            bar_text,
            1,
            FOUR_FOUR_BAR_TICKS,
            BarNotation::Rhythm,
            &mut LengthCarry::start_of_line(),
        )
    }

    fn song(bar_text: &str) -> Result<Vec<BarElement>, String> {
        parse_bar(
            bar_text,
            1,
            FOUR_FOUR_BAR_TICKS,
            BarNotation::SongLine,
            &mut LengthCarry::start_of_line(),
        )
    }

    fn starts_and_lengths(bar_text: &str) -> Vec<(u64, u64)> {
        rhythm(bar_text)
            .unwrap()
            .iter()
            .map(|element| (element.start_ticks, element.length_ticks))
            .collect()
    }

    fn error_of(bar_text: &str) -> String {
        rhythm(bar_text).unwrap_err()
    }

    #[test]
    fn charleston_adds_up() {
        assert_eq!(
            starts_and_lengths("q. e hr"),
            vec![(0, 7560), (7560, 2520), (10_080, 10_080)]
        );
    }

    #[test]
    fn r_and_dots_go_in_either_order() {
        assert_eq!(starts_and_lengths("q hr."), starts_and_lengths("q h.r"));
        assert_eq!(starts_and_lengths("q hr."), vec![(0, 5040), (5040, 15_120)]);
    }

    #[test]
    fn triplets_are_three_in_the_time_of_two() {
        let places: Vec<(u64, u64)> = starts_and_lengths("trp (q q q) trp (q q q)");
        let expected: Vec<(u64, u64)> = (0..6).map(|index| (index * 3360, 3360)).collect();
        assert_eq!(places, expected);
    }

    #[test]
    fn five_eighths_fit_one_quarter() {
        let places: Vec<(u64, u64)> = starts_and_lengths("fitQ(e e e e e) q q q");
        assert_eq!(
            places[..5].to_vec(),
            (0..5).map(|index| (index * 1008, 1008)).collect::<Vec<_>>()
        );
        assert_eq!(places[5], (5040, 5040));
    }

    #[test]
    fn seven_over_a_bar_are_even() {
        let places: Vec<(u64, u64)> = starts_and_lengths("fitBar(e e e e e e e)");
        assert_eq!(
            places,
            (0..7).map(|index| (index * 2880, 2880)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn eleven_over_a_bar_round_once_and_still_fill_the_bar() {
        let places: Vec<(u64, u64)> = starts_and_lengths("fitBar(e e e e e e e e e e e)");
        assert_eq!(places.len(), 11);
        let mut expected_start_ticks: u64 = 0;
        for (start_ticks, length_ticks) in &places {
            assert_eq!(*start_ticks, expected_start_ticks);
            // 20160 ticks ÷ 11 = 1832.7 ticks: each is 1832 or 1833.
            assert!(*length_ticks == 1832 || *length_ticks == 1833);
            expected_start_ticks += length_ticks;
        }
        assert_eq!(expected_start_ticks, FOUR_FOUR_BAR_TICKS);
    }

    #[test]
    fn marks_and_ties_are_read() {
        let elements: Vec<BarElement> = rhythm("q> q>>' e_ e^~ q").unwrap();
        assert_eq!(elements[0].marks.accent(), AccentLevel::Accent);
        assert_eq!(elements[1].marks.accent(), AccentLevel::Marcato);
        assert!(elements[1].marks.staccato());
        assert!(elements[2].marks.legato());
        assert!(elements[3].marks.fermata());
        assert!(elements[3].tied_to_next);
        assert!(!elements[4].tied_to_next);
    }

    #[test]
    fn a_short_bar_says_what_to_add() {
        assert_eq!(
            error_of("q q q e"),
            "bar 1 is short by an eighth note. Add e or er."
        );
    }

    #[test]
    fn a_long_bar_says_so() {
        assert_eq!(
            error_of("q q q q e"),
            "bar 1 is over by an eighth note. Shorten or remove an element."
        );
    }

    #[test]
    fn mistakes_name_what_to_type_instead() {
        assert!(error_of("q q q z").contains("'z' is not a note value"));
        assert!(error_of("q q qr> q").contains("a rest takes no accent"));
        assert!(error_of("q q qr~ q").contains("a rest cannot be tied"));
        assert!(error_of("q q q q:C2").contains("A rhythm carries no notes"));
        assert!(error_of("trp (q q q").contains("missing its closing )"));
        assert!(error_of("fitBar(trp (q q q) q)").contains("group inside a group"));
        assert!(error_of("q q q q....").contains("more than 3 dots"));
        assert!(error_of("").contains("is empty"));
    }

    fn contents_of(bar_text: &str) -> Vec<Option<String>> {
        song(bar_text)
            .unwrap()
            .into_iter()
            .map(|element: BarElement| element.contents_text)
            .collect()
    }

    fn song_starts_and_lengths(bar_text: &str) -> Vec<(u64, u64)> {
        song(bar_text)
            .unwrap()
            .iter()
            .map(|element| (element.start_ticks, element.length_ticks))
            .collect()
    }

    #[test]
    fn a_song_element_writes_what_it_plays_first_and_may_hold_spaces_in_brackets_or_quotes() {
        assert_eq!(
            contents_of("ii7:q C2 [D3 F3 A3] ii7[F3 A3 C4]"),
            vec![
                Some("ii7".to_string()),
                Some("C2".to_string()),
                Some("[D3 F3 A3]".to_string()),
                Some("ii7[F3 A3 C4]".to_string()),
            ]
        );
        assert_eq!(
            contents_of("\"Start SQ-1\":w"),
            vec![Some("\"Start SQ-1\"".to_string())]
        );
    }

    #[test]
    fn a_slash_plays_the_same_again_and_a_rest_plays_nothing() {
        let elements: Vec<BarElement> = song("I:q / r /").unwrap();
        let contents: Vec<Option<String>> = elements
            .iter()
            .map(|element: &BarElement| element.contents_text.clone())
            .collect();
        assert_eq!(contents, vec![Some("I".to_string()), None, None, None]);
        assert_eq!(elements[1].kind, ElementKind::Sounding);
        assert_eq!(elements[2].kind, ElementKind::Rest);
        assert_eq!(elements[3].kind, ElementKind::Sounding);
    }

    #[test]
    fn a_length_left_off_is_the_one_before_it_dots_included() {
        // Four sixteenths, two dotted eighths, an eighth and a quarter: the
        // lengths written once, at each change.
        assert_eq!(
            song_starts_and_lengths("Eb4:sx D4 Eb4 F4 G4:e. A4 B4:e C5:q"),
            vec![
                (0, 1260),
                (1260, 1260),
                (2520, 1260),
                (3780, 1260),
                (5040, 3780),
                (8820, 3780),
                (12_600, 2520),
                (15_120, 5040),
            ]
        );
    }

    #[test]
    fn the_colon_may_be_left_off_after_a_pitch_and_marks_follow_directly() {
        assert_eq!(
            song_starts_and_lengths("Eb4sx D4 Eb4e~ Eb4sx F4 Eb4 D4 C4e D4~ D4 C4"),
            song_starts_and_lengths("Eb4:sx D4 Eb4:e~ Eb4:sx F4 Eb4 D4 C4:e D4~ D4 C4")
        );
        let elements: Vec<BarElement> = song("C5:q> D5~ D5' /^").unwrap();
        assert_eq!(elements[0].marks.accent(), AccentLevel::Accent);
        assert!(elements[1].tied_to_next);
        assert!(elements[2].marks.staccato());
        assert!(elements[3].marks.fermata());
        // The colon may also be written with no length after it.
        assert_eq!(
            song_starts_and_lengths("C4:q D4:~ D4: E4:>"),
            song_starts_and_lengths("C4:q D4~ D4 E4>")
        );
    }

    #[test]
    fn a_rest_writes_r_and_takes_a_length_like_a_note() {
        // An eighth rest, four sixteenths, and a rest of the rest of the bar
        // (a dotted half is too long; a half and an eighth rest fill it).
        assert_eq!(
            song_starts_and_lengths("r:e C3:sx D3 Eb3 D3 r:h r:e"),
            vec![
                (0, 2520),
                (2520, 1260),
                (3780, 1260),
                (5040, 1260),
                (6300, 1260),
                (7560, 10_080),
                (17_640, 2520),
            ]
        );
    }

    #[test]
    fn the_length_carries_across_barlines_but_starts_afresh_each_line() {
        let mut carry: LengthCarry = LengthCarry::start_of_line();
        let bars: Vec<Vec<BarElement>> = parse_bars(
            "| C4:h D4 | E4 F4 |",
            0,
            FOUR_FOUR_BAR_TICKS,
            BarNotation::SongLine,
            &mut carry,
        )
        .unwrap();
        assert_eq!(bars[1][0].length_ticks, 10_080);
        let error: String = song("C4 D4:q E4 F4").unwrap_err();
        assert!(
            error.contains("\"C4\" has no length, and nothing before it in this line gives one. Write it, e.g. C4:q"),
            "{error}"
        );
    }

    #[test]
    fn after_a_group_the_written_length_carries_not_the_squeezed_one() {
        // trp (C4:q D4 E4) fills a half; F4 and G4 are plain quarters.
        assert_eq!(
            song_starts_and_lengths("trp (C4:q D4 E4) F4 G4"),
            vec![
                (0, 3360),
                (3360, 3360),
                (6720, 3360),
                (10_080, 5040),
                (15_120, 5040)
            ]
        );
    }

    #[test]
    fn a_misplaced_sharp_is_told_where_a_sharp_goes() {
        for bar in ["r:h # C4", "C4#:w", "#C4:w"] {
            let error: String = song(bar).unwrap_err();
            assert!(
                error.contains("a # is a sharp. Write it right after the note's letter, e.g. C#4"),
                "{bar}: {error}"
            );
        }
        assert!(song("C#4:w").is_ok());
        assert!(song("#iv°:w").is_ok());
    }

    #[test]
    fn the_old_order_is_refused_with_the_new_spelling() {
        let error: String = song("q:C4 q q q").unwrap_err();
        assert!(error.contains("writes what it plays first"), "{error}");
        assert!(error.contains("C4:q"), "{error}");
    }

    #[test]
    fn contents_work_inside_a_group() {
        let contents: Vec<Option<String>> = contents_of("trp (C2:q E2 G2) C3:h");
        assert_eq!(contents.len(), 4);
        assert_eq!(contents[2], Some("G2".to_string()));
    }

    #[test]
    fn contents_are_refused_where_only_rhythm_is_allowed() {
        assert!(error_of("q:C2 q q q").contains("carries no notes"));
    }

    #[test]
    fn a_length_with_nothing_before_the_colon_takes_its_contents_from_the_source() {
        let bar: Vec<BarElement> = parse_bar(
            ":h :q r:q",
            1,
            4 * 5040,
            BarNotation::SongLine,
            &mut LengthCarry::start_of_line(),
        )
        .unwrap();
        let from_source: Vec<bool> = bar.iter().map(|element| element.from_source).collect();
        assert_eq!(from_source, vec![true, true, false]);
        assert!(bar.iter().all(|element| element.contents_text.is_none()));
        assert_eq!(bar[0].length_ticks, 2 * 5040);
    }

    #[test]
    fn broken_contents_say_what_to_type() {
        assert!(
            song(">q C2 C2 C2")
                .unwrap_err()
                .contains("does not say what it plays")
        );
        assert!(
            song("[C3 E3:q / / /")
                .unwrap_err()
                .contains("missing its closing ]")
        );
        assert!(
            song("\"Start:w")
                .unwrap_err()
                .contains("missing its closing")
        );
        assert!(
            song("C:q D4 E4 F4")
                .unwrap_err()
                .contains("needs its octave")
        );
        assert!(
            song("C4:qz D4 E4 F4")
                .unwrap_err()
                .contains("'z' is not expected")
        );
    }
}
