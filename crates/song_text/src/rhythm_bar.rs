//! One bar of rhythm notation: note values, rests, dots, marks, ties and
//! the groups `trp` and `fit` (Song Notation, section 3).
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
    /// The text after the colon, if any, exactly as written; the song parser
    /// reads it as a chord, notes or a cue name.
    pub contents_text: Option<String>,
}

/// An element as written, before it is placed.
#[derive(Debug, Clone)]
struct WrittenElement {
    length_subticks: u64,
    kind: ElementKind,
    marks: ElementMarks,
    tied_to_next: bool,
    contents_text: Option<String>,
}

/// What a group does to the natural length of what is inside it.
#[derive(Debug, Clone, Copy)]
enum GroupLength {
    /// `trp`: three in the time of two, so 2/3 of the written lengths.
    TwoThirds,
    /// `fitQ(...)` and the others: exactly this many ticks.
    Fixed { length_ticks: u64 },
}

/// Parses the text between two barlines.
///
/// # Errors
///
/// A message saying what is wrong and what to type instead; the caller adds
/// the file and line. Also when the elements do not add up exactly to
/// `bar_length_ticks`, or when an element carries contents (`q:C2`) and
/// `allow_contents` is false (a rhythm library holds rhythm only).
pub fn parse_bar(
    bar_text: &str,
    bar_number: usize,
    bar_length_ticks: u64,
    allow_contents: bool,
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
                parse_group(&mut cursor, bar_length_ticks, allow_contents)?;
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
            let element: WrittenElement = parse_element(&mut cursor, allow_contents)?;
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
        return Err(format!(
            "bar {bar_number} is empty. Write its notes and rests, e.g. q q q q"
        ));
    }
    let bar_subticks: u64 = bar_length_ticks * SUBTICKS_PER_TICK;
    if position_subticks < bar_subticks {
        let short_ticks: u64 = (bar_subticks - position_subticks).div_ceil(SUBTICKS_PER_TICK);
        let words = describe_length(short_ticks);
        let advice: String = match words.letters {
            Some(letters) => format!(" Add {letters} or {letters}r."),
            None => String::from(" Add a note or rest to fill the bar."),
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
    allow_contents: bool,
) -> Result<Vec<Vec<BarElement>>, String> {
    let line: &str = line.trim();
    if !line.starts_with('|') || !line.ends_with('|') || line.len() < 2 {
        return Err(String::from(
            "a line of bars starts and ends with |, e.g. | q q q q |",
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
            allow_contents,
        )?);
    }
    Ok(bars)
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
        // Cannot be large: a bar is at most 255 counts of at most 20160 ticks.
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
        });
    }
    Ok(result)
}

/// `trp (...)`, `fitQ(...)`, `fitE`, `fitH`, `fitW`, `fitBar`.
fn parse_group(
    cursor: &mut Cursor,
    bar_length_ticks: u64,
    allow_contents: bool,
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
                inner.push(parse_element(cursor, allow_contents)?);
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

/// One element: value, then `r` and dots in either order (`hr.` and `h.r`
/// are the same dotted half rest), then marks, then a tie, then `:` and
/// the contents (a chord, notes or a cue name) when the file allows them.
fn parse_element(cursor: &mut Cursor, allow_contents: bool) -> Result<WrittenElement, String> {
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

    let mut contents_text: Option<String> = None;
    match cursor.peek() {
        None | Some(')') => {}
        Some(character) if character.is_whitespace() => {}
        Some(':') if !allow_contents => {
            return Err(String::from(
                "':' in a rhythm pattern. A rhythm carries no notes or chords; \
                 they are written in a song's lines",
            ));
        }
        Some(':') => {
            cursor.next();
            contents_text = Some(parse_contents_text(cursor)?);
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
    if kind == ElementKind::Rest {
        if marks.accent() != AccentLevel::Normal || marks.staccato() || marks.legato() {
            return Err(String::from(
                "a rest takes no accent, staccato or legato mark. Only a fermata (^) \
                 may stand on a rest",
            ));
        }
        if tied_to_next {
            return Err(String::from("a rest cannot be tied. Tie notes, not rests"));
        }
    }

    Ok(WrittenElement {
        length_subticks: dotted_length_subticks(base_ticks, dots, SUBTICKS_PER_TICK),
        kind,
        marks,
        tied_to_next,
        contents_text,
    })
}

/// The text after a colon: a quoted name (`"Start SQ-1"`), or characters up
/// to the next space outside square brackets (`ii7`, `C2`, `[D3 F3 A3]`,
/// `ii7[F3 A3 C4]`). What it means is for the song parser to say.
fn parse_contents_text(cursor: &mut Cursor) -> Result<String, String> {
    let mut text: String = String::new();
    if cursor.peek() == Some('"') {
        text.push('"');
        cursor.next();
        loop {
            match cursor.next() {
                None => {
                    return Err(String::from(
                        "a quoted name after ':' is missing its closing \"",
                    ));
                }
                Some('"') => {
                    text.push('"');
                    return Ok(text);
                }
                Some(character) => text.push(character),
            }
        }
    }
    let mut inside_brackets: bool = false;
    while let Some(character) = cursor.peek() {
        if !inside_brackets && (character.is_whitespace() || character == ')') {
            break;
        }
        match character {
            '[' if inside_brackets => {
                return Err(String::from(
                    "'[' inside square brackets. Close the first with ]",
                ));
            }
            '[' => inside_brackets = true,
            ']' => inside_brackets = false,
            _ => {}
        }
        text.push(character);
        cursor.next();
    }
    if inside_brackets {
        return Err(String::from("a '[' after ':' is missing its closing ]"));
    }
    if text.is_empty() {
        return Err(String::from(
            "nothing after ':'. Write a chord such as ii7, a note such as C2, \
             notes such as [C3 E3 G3], or a quoted cue name",
        ));
    }
    Ok(text)
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
                    AccentLevel::Strong
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

    fn starts_and_lengths(bar_text: &str) -> Vec<(u64, u64)> {
        parse_bar(bar_text, 1, FOUR_FOUR_BAR_TICKS, false)
            .unwrap()
            .iter()
            .map(|element| (element.start_ticks, element.length_ticks))
            .collect()
    }

    fn error_of(bar_text: &str) -> String {
        parse_bar(bar_text, 1, FOUR_FOUR_BAR_TICKS, false).unwrap_err()
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
        let elements: Vec<BarElement> =
            parse_bar("q> q>>' e_ e^~ q", 1, FOUR_FOUR_BAR_TICKS, false).unwrap();
        assert_eq!(elements[0].marks.accent(), AccentLevel::Accent);
        assert_eq!(elements[1].marks.accent(), AccentLevel::Strong);
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
        parse_bar(bar_text, 1, FOUR_FOUR_BAR_TICKS, true)
            .unwrap()
            .into_iter()
            .map(|element: BarElement| element.contents_text)
            .collect()
    }

    #[test]
    fn contents_follow_a_colon_and_may_hold_spaces_in_brackets_or_quotes() {
        assert_eq!(
            contents_of("q:ii7 q:C2 q:[D3 F3 A3] q:ii7[F3 A3 C4]"),
            vec![
                Some("ii7".to_string()),
                Some("C2".to_string()),
                Some("[D3 F3 A3]".to_string()),
                Some("ii7[F3 A3 C4]".to_string()),
            ]
        );
        assert_eq!(
            contents_of("w:\"Start SQ-1\""),
            vec![Some("\"Start SQ-1\"".to_string())]
        );
        assert_eq!(
            contents_of("q:I q q q"),
            vec![Some("I".to_string()), None, None, None]
        );
    }

    #[test]
    fn contents_work_inside_a_group() {
        let contents: Vec<Option<String>> = contents_of("trp (q:C2 q:E2 q:G2) h");
        assert_eq!(contents.len(), 4);
        assert_eq!(contents[2], Some("G2".to_string()));
    }

    #[test]
    fn contents_are_refused_where_only_rhythm_is_allowed() {
        assert!(error_of("q:C2 q q q").contains("carries no notes"));
    }

    #[test]
    fn broken_contents_say_what_to_type() {
        let error_for = |text: &str| parse_bar(text, 1, FOUR_FOUR_BAR_TICKS, true).unwrap_err();
        assert!(error_for("q: q q q").contains("nothing after ':'"));
        assert!(error_for("q:[C3 E3 q q q").contains("missing its closing ]"));
        assert!(error_for("w:\"Start").contains("missing its closing"));
    }
}
