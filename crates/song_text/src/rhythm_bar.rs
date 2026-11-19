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

use vocabulary::bar_meters::BarMeters;
use vocabulary::dynamics::DynamicMark;
use vocabulary::rhythm_pattern::{AccentLevel, ElementKind, ElementMarks};
use vocabulary::ticks::{TICKS_PER_QUARTER_NOTE, TICKS_PER_WHOLE_NOTE};
use vocabulary::time_signature::TimeSignature;

use crate::contents_text::parse_chord;
use crate::euclid_pattern::euclid_pattern;
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
    /// Octaves the element moves, from `+8va` (1), `-8va` (-1) or `loco` (0)
    /// written at the start of its bar or an earlier one on the line.
    pub octave_shift_octaves: i8,
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
    /// A dynamics line's: a level word (`pp p mp mf f ff`), `<`, `>` or `r`,
    /// then its length (`p:w`, `<:h`, `r:q`). No marks, no tie.
    Dynamics,
}

/// A note value with its dots, as written: what a song line's element
/// takes when it writes no length of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WrittenLength {
    base_ticks: u64,
    dots: u32,
}

/// What a song line carries forward from one element and bar to the next:
/// the last length written, dots included, across barlines and the text lines
/// a line goes on over (in a group it is the written value, before the group
/// squeezes it, so the element after `trp (C4:q D4 E4)` is a plain quarter,
/// Song Notation, section 4); and the octave annotation in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineCarry {
    written: Option<WrittenLength>,
    /// The function (`trp(q q q)`) the last element wrote in place of a
    /// length, as written. An element that writes no length plays it again
    /// (Song Notation, section 3, "Repeating"); a length written clears it.
    function_text: Option<String>,
    /// How many lengths elements have written; a group reads it to tell
    /// whether it holds any length of its own.
    lengths_written: u32,
    /// `+8va` is 1, `-8va` is -1, `loco` is 0, until the next annotation.
    octave_shift_octaves: i8,
}

impl LineCarry {
    /// Nothing written yet: the first element of a line writes its length,
    /// and the octave is the clef's.
    #[must_use]
    pub fn start_of_line() -> LineCarry {
        LineCarry {
            written: None,
            function_text: None,
            lengths_written: 0,
            octave_shift_octaves: 0,
        }
    }
}

/// An octave annotation at the start of a bar's text, and the text after it:
/// `+8va` and `-8va` move what follows one octave up or down from the
/// clef's octave, `loco` returns to the clef's octave (Song Notation,
/// section 4). The annotation says where the octave is, so writing `+8va`
/// twice is still one octave up.
fn split_octave_annotation(bar_text: &str) -> (Option<i8>, &str) {
    let trimmed: &str = bar_text.trim_start();
    let word_end: usize = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let (word, rest): (&str, &str) = trimmed.split_at(word_end);
    match word {
        "+8va" => (Some(1), rest),
        "-8va" => (Some(-1), rest),
        "loco" => (Some(0), rest),
        _ => (None, bar_text),
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
/// the file and line. Also when the elements do not add up exactly to the
/// length of `bar_meter`, which the message names.
pub fn parse_bar(
    bar_text: &str,
    bar_number: usize,
    bar_meter: TimeSignature,
    notation: BarNotation,
    carry: &mut LineCarry,
) -> Result<Vec<BarElement>, String> {
    let bar_length_ticks: u64 = bar_meter.bar_length().ticks();
    let mut cursor: Cursor = Cursor::new(bar_text);
    let mut placed: Vec<BarElement> = Vec::new();
    let mut position_subticks: u64 = 0;

    loop {
        cursor.skip_whitespace();
        if notation == BarNotation::SongLine {
            take_octave_annotations(&mut cursor, carry);
        }
        if cursor.peek().is_none() {
            break;
        }
        if notation == BarNotation::SongLine && cursor.element_text().contains(":euclid(") {
            if !placed.is_empty() {
                return Err(format!(
                    "bar {bar_number}: euclid fills the whole bar. Write it alone in its bar, \
                     e.g. | C2:euclid(3,8) |"
                ));
            }
            let mut euclid: Vec<BarElement> =
                parse_euclid_bar(&mut cursor, bar_number, bar_length_ticks)?;
            for element in &mut euclid {
                element.octave_shift_octaves = carry.octave_shift_octaves;
            }
            return Ok(euclid);
        }
        let placed_before: usize = placed.len();
        let function: Option<(GroupLength, Vec<WrittenElement>)> =
            if notation == BarNotation::SongLine {
                parse_song_function(&mut cursor, bar_length_ticks, carry)?
            } else {
                None
            };
        let group: Option<(GroupLength, Vec<WrittenElement>)> = if function.is_some() {
            function
        } else if cursor.starts_with("trp") || cursor.starts_with("fit") {
            Some(parse_standalone_group(
                &mut cursor,
                bar_length_ticks,
                notation,
                carry,
            )?)
        } else {
            None
        };
        if let Some((group_length, inner)) = group {
            let group_subticks: u64 = group_length_subticks(group_length, &inner, bar_number)?;
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
        // The octave annotation in force when the element was written.
        for element in &mut placed[placed_before..] {
            element.octave_shift_octaves = carry.octave_shift_octaves;
        }
    }

    check_bar_is_full(&placed, position_subticks, bar_number, bar_meter, notation)?;
    Ok(placed)
}

/// The bar has elements and they add up exactly to the length of its meter.
fn check_bar_is_full(
    placed: &[BarElement],
    position_subticks: u64,
    bar_number: usize,
    bar_meter: TimeSignature,
    notation: BarNotation,
) -> Result<(), String> {
    let bar_length_ticks: u64 = bar_meter.bar_length().ticks();
    if placed.is_empty() {
        let example: &str = match notation {
            BarNotation::Rhythm => "q q q q",
            BarNotation::SongLine => "C4:q D4 E4 r",
            BarNotation::Dynamics => "p:q < f r",
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
            (Some(letters), BarNotation::SongLine | BarNotation::Dynamics) => {
                format!(" Add a note or a rest of that length, e.g. r:{letters}.")
            }
            (None, _) => String::from(" Add a note or rest to fill the bar."),
        };
        return Err(format!(
            "bar {bar_number} is {bar_meter} and short by {}.{advice}",
            words.description
        ));
    }
    if position_subticks > bar_subticks {
        let over_ticks: u64 = (position_subticks - bar_subticks).div_ceil(SUBTICKS_PER_TICK);
        return Err(format!(
            "bar {bar_number} is {bar_meter} and over by {}. Shorten or remove an element.",
            describe_length(over_ticks).description
        ));
    }
    Ok(())
}

/// `+8va`, `-8va` or `loco` standing before an element, at any place in a
/// bar: it sets the octave of what follows, until the next annotation (Song
/// Notation, section 3).
fn take_octave_annotations(cursor: &mut Cursor, carry: &mut LineCarry) {
    loop {
        let word: String = cursor.element_text();
        let octave_shift_octaves: i8 = match word.as_str() {
            "+8va" => 1,
            "-8va" => -1,
            "loco" => 0,
            _ => return,
        };
        carry.octave_shift_octaves = octave_shift_octaves;
        for _ in word.chars() {
            cursor.next();
        }
        cursor.skip_whitespace();
    }
}

/// A group written as an element of its own: `trp(q q q)` in a rhythm,
/// `trp(F2:q A2:q C2:q)` on a song line, where each element names what it plays.
fn parse_standalone_group(
    cursor: &mut Cursor,
    bar_length_ticks: u64,
    notation: BarNotation,
    carry: &mut LineCarry,
) -> Result<(GroupLength, Vec<WrittenElement>), String> {
    let group_start_position: usize = cursor.position;
    let parsed: (GroupLength, Vec<WrittenElement>) =
        parse_group(cursor, bar_length_ticks, notation, false, carry)?;
    if notation == BarNotation::SongLine {
        // A group that holds its own notes is the function the next `/`
        // plays again (Song Notation, section 3, "Repeating").
        carry.function_text = Some(
            cursor.characters[group_start_position..cursor.position]
                .iter()
                .collect(),
        );
    }
    Ok(parsed)
}

/// How long a group plays, in subticks: `trp` is 2/3 of its written
/// lengths; the `fit` functions have a length of their own.
fn group_length_subticks(
    group_length: GroupLength,
    inner: &[WrittenElement],
    bar_number: usize,
) -> Result<u64, String> {
    let natural_subticks: u64 = inner.iter().map(|element| element.length_subticks).sum();
    match group_length {
        GroupLength::TwoThirds => {
            if !(natural_subticks * 2).is_multiple_of(3) {
                return Err(format!(
                    "bar {bar_number}: this trp group cannot be divided into whole \
                     parts. Use fitH(...) or fitQ(...) to say how long it should be"
                ));
            }
            Ok(natural_subticks * 2 / 3)
        }
        GroupLength::Fixed { length_ticks } => Ok(length_ticks * SUBTICKS_PER_TICK),
    }
}

/// `C2:euclid(3,8)>` or `:euclid(3,8)`: hits spread over the steps of the
/// whole bar, which share it evenly (the same rounding as a sample
/// source's elements). A hit is a one-step sounding element carrying what
/// is written before the colon and the marks written after; the steps
/// between hits are rests.
/// The `euclid(3,8)` after the colon: its hits and its steps.
fn read_euclid_hits_and_steps(
    cursor: &mut Cursor,
    bar_number: usize,
    element_text: &str,
) -> Result<(usize, usize), String> {
    let example: &str = "euclid(3,8)";
    if !cursor.starts_with("euclid(") {
        return Err(format!(
            "bar {bar_number}: \"{element_text}\": write euclid as hits and steps, e.g. {example}"
        ));
    }
    for _ in 0.."euclid(".len() {
        cursor.next();
    }
    let hits_text: String = cursor.take_while(char::is_ascii_digit);
    let comma_follows: bool = cursor.peek() == Some(',');
    cursor.next();
    let steps_text: String = cursor.take_while(char::is_ascii_digit);
    let closed: bool = cursor.peek() == Some(')');
    cursor.next();
    let (Ok(hits), Ok(steps), true, true) = (
        hits_text.parse::<usize>(),
        steps_text.parse::<usize>(),
        comma_follows,
        closed,
    ) else {
        return Err(format!(
            "bar {bar_number}: \"{element_text}\": write euclid as hits and steps, e.g. {example}"
        ));
    };
    if hits == 0 || hits > steps {
        return Err(format!(
            "bar {bar_number}: euclid({hits},{steps}) needs at least 1 hit and no more hits than \
             steps. Write e.g. {example}: 3 hits over 8 steps"
        ));
    }
    Ok((hits, steps))
}

fn parse_euclid_bar(
    cursor: &mut Cursor,
    bar_number: usize,
    bar_length_ticks: u64,
) -> Result<Vec<BarElement>, String> {
    let element_text: String = cursor.element_text();
    let contents: WrittenContents = parse_song_contents(cursor, &element_text)?;
    if cursor.peek() == Some(':') {
        cursor.next();
    }
    let (hits, steps): (usize, usize) =
        read_euclid_hits_and_steps(cursor, bar_number, &element_text)?;
    let marks: ElementMarks = parse_marks(cursor)?;
    if cursor.peek() == Some('~') {
        return Err(format!(
            "bar {bar_number}: a tie cannot follow euclid. Tie notes one by one if you need one"
        ));
    }
    cursor.skip_whitespace();
    if cursor.peek().is_some() {
        return Err(format!(
            "bar {bar_number}: euclid fills the whole bar. Write nothing else after it in the bar"
        ));
    }
    let (contents_text, from_source): (Option<String>, bool) = match contents {
        WrittenContents::Rest => {
            return Err(format!(
                "bar {bar_number}: a rest cannot be spread with euclid. Write what the hits play \
                 before the colon, e.g. C2:euclid(3,8)"
            ));
        }
        WrittenContents::Same => (None, false),
        WrittenContents::FromSource => (None, true),
        WrittenContents::Text(text) => (Some(text), false),
    };
    // The place of step i is i × bar ÷ steps, rounded half up, so no rounded
    // step is added to another: ticks × (steps ÷ steps) → ticks.
    let steps_count: u64 = steps as u64;
    let tick_of =
        |index: u64| -> u64 { (2 * index * bar_length_ticks + steps_count) / (2 * steps_count) };
    let mut placed: Vec<BarElement> = Vec::new();
    for (index, hit) in euclid_pattern(hits, steps).into_iter().enumerate() {
        let start_ticks: u64 = tick_of(index as u64);
        let end_ticks: u64 = tick_of(index as u64 + 1);
        if end_ticks <= start_ticks {
            return Err(format!(
                "bar {bar_number}: euclid has too many steps for the bar. Write fewer"
            ));
        }
        if hit {
            placed.push(BarElement {
                start_ticks,
                length_ticks: end_ticks - start_ticks,
                kind: ElementKind::Sounding,
                marks,
                tied_to_next: false,
                contents_text: contents_text.clone(),
                from_source,
                octave_shift_octaves: 0,
            });
        } else if let Some(last) = placed
            .last_mut()
            .filter(|last: &&mut BarElement| last.kind == ElementKind::Rest)
        {
            // Rests between hits are one rest.
            last.length_ticks = end_ticks - last.start_ticks;
        } else {
            placed.push(BarElement {
                start_ticks,
                length_ticks: end_ticks - start_ticks,
                kind: ElementKind::Rest,
                marks: ElementMarks::NONE,
                tied_to_next: false,
                contents_text: None,
                from_source: false,
                octave_shift_octaves: 0,
            });
        }
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
    bar_meters: &BarMeters,
    notation: BarNotation,
    carry: &mut LineCarry,
) -> Result<Vec<Vec<BarElement>>, String> {
    let line: &str = line.trim();
    if !line.starts_with('|') || !line.ends_with('|') || line.len() < 2 {
        let example: &str = match notation {
            BarNotation::Rhythm => "| q q q q |",
            BarNotation::SongLine => "| C4:q D4 E4 F4 |",
            BarNotation::Dynamics => "| p:w | < | f | mf |",
        };
        return Err(format!(
            "a line of bars starts and ends with |, e.g. {example}"
        ));
    }
    let inside: &str = &line[1..line.len() - 1];
    let mut bars: Vec<Vec<BarElement>> = Vec::new();
    for (offset, bar_text) in split_at_barlines(inside).into_iter().enumerate() {
        let bar_number: usize = bars_so_far + offset + 1;
        let bar_text: &str = if notation == BarNotation::SongLine {
            let (annotation, rest): (Option<i8>, &str) = split_octave_annotation(bar_text);
            if let Some(octave_shift_octaves) = annotation {
                carry.octave_shift_octaves = octave_shift_octaves;
            }
            rest
        } else {
            bar_text
        };
        // bar_number counts from 1; the meters count bars from 0.
        let bar_meter: TimeSignature = bar_meters.meter_of_bar(bar_number - 1);
        bars.push(parse_bar(bar_text, bar_number, bar_meter, notation, carry)?);
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
    bar_meters: &BarMeters,
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
        let bar_length_ticks: u64 = bar_meters.bar_length_ticks(bar_number - 1);
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
            octave_shift_octaves: 0,
        });
    }
    Ok(placed)
}

/// Splits at `|`, except inside a quoted cue name.
pub(crate) fn split_at_barlines(text: &str) -> Vec<&str> {
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
            octave_shift_octaves: 0,
        });
    }
    Ok(result)
}

/// The function names a song line may write after the colon.
const FUNCTION_NAMES: [&str; 6] = ["trp", "fitQ", "fitE", "fitH", "fitW", "fitBar"];

/// A song line's element whose length is a function: `C2:trp(q q q)`,
/// `I:fitBar(q e e h)>`, `:trp(q r q)`; or an element that writes no length
/// while the line's last length was a function (`/`, `D2`), which plays the
/// function again. What is written before the colon plays on every sounding
/// element the function gives; marks after the `)` apply to each of them.
/// `None` when the element is an ordinary one, left for the ordinary parser
/// (which also reports any mistake in it).
fn parse_song_function(
    cursor: &mut Cursor,
    bar_length_ticks: u64,
    carry: &mut LineCarry,
) -> Result<Option<(GroupLength, Vec<WrittenElement>)>, String> {
    // A group standing alone (`trp(F2:q A2:q C2:q)`) is no function form:
    // the group parser reads it.
    if cursor.starts_with("trp") || cursor.starts_with("fit") {
        return Ok(None);
    }
    let start_position: usize = cursor.position;
    let element_text: String = cursor.element_text();
    let Ok(contents) = parse_song_contents(cursor, &element_text) else {
        cursor.position = start_position;
        return Ok(None);
    };
    if cursor.peek() == Some(':') {
        cursor.next();
    }
    let mut written_here: bool = true;
    let function_text: String = if let Some(text) = take_function_text(cursor)? {
        text
    } else if matches!(parse_written_length(cursor), Ok(None)) {
        written_here = false;
        // No length written: the function before it plays again.
        let Some(text) = carry.function_text.clone() else {
            cursor.position = start_position;
            return Ok(None);
        };
        text
    } else {
        cursor.position = start_position;
        return Ok(None);
    };

    let mut function_cursor: Cursor = Cursor::new(&function_text);
    // The function is read on its own, so a length left off inside it is the
    // one before it in the function, never the line's.
    let (group_length, lengths): (GroupLength, Vec<WrittenElement>) = parse_group(
        &mut function_cursor,
        bar_length_ticks,
        BarNotation::SongLine,
        true,
        &mut LineCarry::start_of_line(),
    )?;
    // A group played again keeps the marks it was written with; a function
    // written after a colon takes its marks after the ).
    if written_here
        && lengths.iter().any(|length: &WrittenElement| {
            length.marks != ElementMarks::NONE || length.tied_to_next
        })
    {
        return Err(format!(
            "\"{function_text}\": write marks and ties after the ), where they apply to every \
             note the function gives, e.g. C2:trp(q q q)>"
        ));
    }

    let marks: ElementMarks = parse_marks(cursor)?;
    if cursor.peek() == Some('~') {
        return Err(String::from(
            "a tie cannot follow a function. Tie notes one by one if you need one",
        ));
    }
    match cursor.peek() {
        None => {}
        Some(character) if character.is_whitespace() => {}
        Some(character) => {
            return Err(format!(
                "'{character}' after the function \"{function_text}\". Write marks right after \
                 the ), then a space"
            ));
        }
    }
    let Some(elements) = give_function_its_contents(lengths, contents, marks)? else {
        if written_here {
            return Err(format!(
                "every element of {function_text} names what it plays, so what is before the \
                 colon plays nothing. Write the function alone, e.g. trp(F2:q A2:q C2:q)"
            ));
        }
        // Only `/` plays such a group again; a note with no length after it
        // is an ordinary element with the carried length.
        cursor.position = start_position;
        carry.function_text = None;
        return Ok(None);
    };
    carry.function_text = Some(function_text);
    Ok(Some((group_length, elements)))
}

/// What each element of a function plays. An element that names what it
/// plays keeps it; one that gives only a length plays `contents`, what is
/// before the colon, and takes `marks` after the `)`. `None` when every
/// element names its own and `contents` has nothing left to play.
fn give_function_its_contents(
    lengths: Vec<WrittenElement>,
    contents: WrittenContents,
    marks: ElementMarks,
) -> Result<Option<Vec<WrittenElement>>, String> {
    let outer_is_same: bool = matches!(contents, WrittenContents::Same);
    let (rest_everywhere, contents_text, from_source): (bool, Option<String>, bool) = match contents
    {
        WrittenContents::Rest => (true, None, false),
        WrittenContents::Same => (false, None, false),
        WrittenContents::FromSource => (false, None, true),
        WrittenContents::Text(text) => (false, Some(text), false),
    };
    if rest_everywhere {
        check_rest(ElementKind::Rest, marks, false)?;
    }
    let plays_own = |element: &WrittenElement| -> bool {
        element.contents_text.is_some() || element.from_source
    };
    let takes_outer: bool = lengths.iter().any(|element: &WrittenElement| {
        element.kind == ElementKind::Sounding && !plays_own(element)
    });
    if !takes_outer && !rest_everywhere && !outer_is_same {
        return Ok(None);
    }
    Ok(Some(
        lengths
            .into_iter()
            .map(|length: WrittenElement| {
                if rest_everywhere || length.kind == ElementKind::Rest {
                    WrittenElement {
                        kind: ElementKind::Rest,
                        marks: ElementMarks::NONE,
                        contents_text: None,
                        from_source: false,
                        ..length
                    }
                } else if plays_own(&length) {
                    WrittenElement {
                        marks: if marks == ElementMarks::NONE {
                            length.marks
                        } else {
                            marks
                        },
                        ..length
                    }
                } else {
                    WrittenElement {
                        marks,
                        contents_text: contents_text.clone(),
                        from_source,
                        ..length
                    }
                }
            })
            .collect(),
    ))
}

/// `trp(q q q)` at the cursor, as written, parentheses included; the cursor
/// moves past the `)`. `None` when no function name and `(` stand here.
fn take_function_text(cursor: &mut Cursor) -> Result<Option<String>, String> {
    let mut name_length: usize = 0;
    while cursor
        .characters
        .get(cursor.position + name_length)
        .is_some_and(char::is_ascii_alphabetic)
    {
        name_length += 1;
    }
    let opens: bool = cursor.characters.get(cursor.position + name_length) == Some(&'(');
    if name_length < 2 || !opens {
        return Ok(None);
    }
    let name: String = cursor.characters[cursor.position..cursor.position + name_length]
        .iter()
        .collect();
    if !FUNCTION_NAMES.contains(&name.as_str()) {
        return Err(format!(
            "\"{name}\" is not a function. Use trp, fitQ, fitE, fitH, fitW or fitBar, \
             e.g. C2:trp(q q q)"
        ));
    }
    let mut text: String = String::new();
    loop {
        match cursor.next() {
            None => {
                return Err(format!("the function {name}(...) is missing its closing )"));
            }
            Some(')') => {
                text.push(')');
                return Ok(Some(text));
            }
            Some(character) => text.push(character),
        }
    }
}

/// The group a name stands for: `trp` or one of the `fit` functions.
fn group_length_named(name: &str, bar_length_ticks: u64) -> Result<GroupLength, String> {
    let fixed = |length_ticks: u64| -> Result<GroupLength, String> {
        Ok(GroupLength::Fixed { length_ticks })
    };
    match name {
        "trp" => Ok(GroupLength::TwoThirds),
        "fitQ" => fixed(TICKS_PER_QUARTER_NOTE),
        "fitE" => fixed(TICKS_PER_QUARTER_NOTE / 2),
        "fitH" => fixed(TICKS_PER_WHOLE_NOTE / 2),
        "fitW" => fixed(TICKS_PER_WHOLE_NOTE),
        "fitBar" => fixed(bar_length_ticks),
        _ => Err(format!(
            "\"{name}\" is not a group. Use trp, fitQ, fitE, fitH, fitW or fitBar, \
             e.g. trp (q q q)"
        )),
    }
}

/// The elements of a group, from after its `(` to and including its `)`.
fn parse_group_elements(
    cursor: &mut Cursor,
    name: &str,
    notation: BarNotation,
    bare_lengths: bool,
    carry: &mut LineCarry,
) -> Result<Vec<WrittenElement>, String> {
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
                // A bare `r` inside the parentheses is a rest as long as the
                // element before it: `trp(q r q)` (Song Notation, section 3).
                let bare_rest: bool = cursor.peek() == Some('r')
                    && cursor
                        .peek_second()
                        .is_none_or(|after: char| after.is_whitespace() || after == ')');
                if bare_rest && (notation == BarNotation::Rhythm || bare_lengths) {
                    let Some(previous_length_subticks) = inner
                        .last()
                        .map(|element: &WrittenElement| element.length_subticks)
                    else {
                        return Err(format!(
                            "{name}(r ...) starts with a rest that has no length. Write the \
                             length first, e.g. {name}(qr q q)"
                        ));
                    };
                    cursor.next();
                    inner.push(WrittenElement {
                        length_subticks: previous_length_subticks,
                        kind: ElementKind::Rest,
                        marks: ElementMarks::NONE,
                        tied_to_next: false,
                        contents_text: None,
                        from_source: false,
                    });
                    continue;
                }
                // In a function after a colon (`C2:trp(q E2:q q)`) an element
                // may be a bare length: it plays what is before the colon.
                let bare_length: bool = bare_lengths
                    && cursor
                        .peek()
                        .is_some_and(|first: char| "whqest".contains(first));
                if bare_length {
                    carry.lengths_written += 1;
                    inner.push(parse_rhythm_element(cursor)?);
                } else {
                    inner.push(parse_element(cursor, notation, carry)?);
                }
            }
        }
    }
    Ok(inner)
}

/// `trp (...)`, `fitQ(...)`, `fitE`, `fitH`, `fitW`, `fitBar`.
fn parse_group(
    cursor: &mut Cursor,
    bar_length_ticks: u64,
    notation: BarNotation,
    bare_lengths: bool,
    carry: &mut LineCarry,
) -> Result<(GroupLength, Vec<WrittenElement>), String> {
    let name: String = cursor.take_while(char::is_ascii_alphabetic);
    let group_length: GroupLength = group_length_named(&name, bar_length_ticks)?;
    cursor.skip_whitespace();
    if cursor.next() != Some('(') {
        return Err(format!(
            "{name} must be followed by ( and the elements, e.g. {name} (q q q)"
        ));
    }
    // A group of notes with no length written anywhere inside (`fitH(D2 F1 A1)`)
    // is equal parts, with or without a length carried into it. Read it first
    // with a quarter note to stand for each; if any element writes a length,
    // read it again as written, so a missing length is reported.
    let after_bracket_position: usize = cursor.position;
    let equal_parts_possible: bool = notation == BarNotation::SongLine
        && carry.written.is_none()
        && matches!(group_length, GroupLength::Fixed { .. });
    let mut equal_parts: Option<Vec<WrittenElement>> = None;
    if equal_parts_possible {
        let mut probe: LineCarry = carry.clone();
        probe.written = Some(WrittenLength {
            base_ticks: TICKS_PER_QUARTER_NOTE,
            dots: 0,
        });
        equal_parts = parse_group_elements(cursor, &name, notation, bare_lengths, &mut probe)
            .ok()
            .filter(|_| probe.lengths_written == 0);
        if equal_parts.is_none() {
            cursor.position = after_bracket_position;
        }
    }
    let inner: Vec<WrittenElement> = match equal_parts {
        Some(parsed) => parsed,
        None => parse_group_elements(cursor, &name, notation, bare_lengths, carry)?,
    };
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
    carry: &mut LineCarry,
) -> Result<WrittenElement, String> {
    match notation {
        BarNotation::Rhythm => parse_rhythm_element(cursor),
        BarNotation::SongLine => parse_song_element(cursor, carry),
        BarNotation::Dynamics => parse_dynamics_element(cursor, carry),
    }
}

/// A dynamics line's element: a level word, `<`, `>` or `r`, then its length
/// (the colon may be left off). `p:w`, `<:h`, `mf`, `r:q`. A length left off
/// is the one before it, as on every song line.
fn parse_dynamics_element(
    cursor: &mut Cursor,
    carry: &mut LineCarry,
) -> Result<WrittenElement, String> {
    let element_text: String = cursor.element_text();
    let word: String = cursor.take_while(|character: &char| {
        character.is_ascii_alphabetic() || *character == '<' || *character == '>'
    });
    // `r` alone is a stretch with no change; the letters of a level are
    // taken as one word, so `pp` is not `p` then `p`.
    let is_rest: bool = word == "r";
    let is_mark: bool = DynamicMark::from_text(&word).is_some();
    if !is_rest && !is_mark {
        return Err(format!(
            "\"{element_text}\" is not a dynamics mark. Write pp, p, mp, mf, f, ff, \
             < (crescendo), > (diminuendo) or r (no change), then its length, e.g. p:w"
        ));
    }
    if cursor.peek() == Some(':') {
        cursor.next();
    }
    let written_length: Option<WrittenLength> = parse_written_length(cursor)?;
    match cursor.peek() {
        None | Some(')') => {}
        Some(character) if character.is_whitespace() => {}
        Some(character) => {
            return Err(format!(
                "'{character}' is not expected in \"{element_text}\". A dynamics mark takes \
                 only its length, e.g. p:w. Accents and ties belong on a note line"
            ));
        }
    }
    let length: WrittenLength = match written_length {
        Some(written) => {
            carry.written = Some(written);
            carry.function_text = None;
            written
        }
        None => carry.written.ok_or_else(|| {
            format!(
                "\"{element_text}\" has no length, and nothing before it in this line \
                 gives one. Write it, e.g. {word}:w"
            )
        })?,
    };
    Ok(WrittenElement {
        length_subticks: dotted_length_subticks(length.base_ticks, length.dots, SUBTICKS_PER_TICK),
        kind: if is_rest {
            ElementKind::Rest
        } else {
            ElementKind::Sounding
        },
        marks: ElementMarks::NONE,
        tied_to_next: false,
        contents_text: if is_rest { None } else { Some(word) },
        from_source: false,
    })
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
                 then marks (> >> < << ' _ ^), then ~ for a tie"
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
    carry: &mut LineCarry,
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
                 then its length, then marks (> >> < << ' _ ^), then ~ for a tie, e.g. C4:q>~"
            ));
        }
    }
    let length: WrittenLength = match written_length {
        Some(written) => {
            carry.written = Some(written);
            carry.function_text = None;
            carry.lengths_written += 1;
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
                !character.is_whitespace() && !":[)><'_^~".contains(*character)
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

/// `>` `>>` `<` `<<` `'` `_` `^`, each at most once, and only one of the
/// four that change the level (`>`, `>>`, `<`, `<<`).
fn parse_marks(cursor: &mut Cursor) -> Result<ElementMarks, String> {
    let mut accent: AccentLevel = AccentLevel::Normal;
    let mut staccato: bool = false;
    let mut legato: bool = false;
    let mut fermata: bool = false;
    loop {
        match cursor.peek() {
            Some(first @ ('>' | '<')) => {
                cursor.next();
                let doubled: bool = cursor.peek() == Some(first);
                if doubled {
                    cursor.next();
                }
                let this_level: AccentLevel = match (first, doubled) {
                    ('>', false) => AccentLevel::Accent,
                    ('>', true) => AccentLevel::Marcato,
                    (_, false) => AccentLevel::Soft,
                    (_, true) => AccentLevel::Softer,
                };
                if accent != AccentLevel::Normal {
                    return Err(String::from(
                        "two level marks on one element. Use one of > >> < <<, not two",
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

    fn four_four() -> TimeSignature {
        TimeSignature::new(4, 4).unwrap()
    }

    fn rhythm(bar_text: &str) -> Result<Vec<BarElement>, String> {
        parse_bar(
            bar_text,
            1,
            four_four(),
            BarNotation::Rhythm,
            &mut LineCarry::start_of_line(),
        )
    }

    fn song(bar_text: &str) -> Result<Vec<BarElement>, String> {
        parse_bar(
            bar_text,
            1,
            four_four(),
            BarNotation::SongLine,
            &mut LineCarry::start_of_line(),
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

    /// A bar as (start ticks, length ticks, kind, what it plays).
    type PlacedBar = Vec<(u64, u64, ElementKind, Option<String>)>;

    fn song_with_carry(bar_texts: &[&str]) -> Vec<PlacedBar> {
        let mut carry: LineCarry = LineCarry::start_of_line();
        bar_texts
            .iter()
            .map(|bar_text: &&str| {
                parse_bar(bar_text, 1, four_four(), BarNotation::SongLine, &mut carry)
                    .unwrap()
                    .into_iter()
                    .map(|element: BarElement| {
                        (
                            element.start_ticks,
                            element.length_ticks,
                            element.kind,
                            element.contents_text,
                        )
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_function_after_the_colon_plays_what_is_before_the_colon_on_each_hit() {
        // trp(q q q): three in the time of a half note (10080 ticks), 3360 ticks each.
        let bar: PlacedBar = song_with_carry(&["C2:trp(q q q) G2:q B2"]).remove(0);
        let c2: Option<String> = Some(String::from("C2"));
        assert_eq!(
            bar,
            vec![
                (0, 3360, ElementKind::Sounding, c2.clone()),
                (3360, 3360, ElementKind::Sounding, c2.clone()),
                (6720, 3360, ElementKind::Sounding, c2),
                (
                    10_080,
                    5040,
                    ElementKind::Sounding,
                    Some(String::from("G2"))
                ),
                (
                    15_120,
                    5040,
                    ElementKind::Sounding,
                    Some(String::from("B2"))
                ),
            ]
        );
    }

    #[test]
    fn a_function_may_hold_rests_and_takes_marks_after_the_bracket() {
        let bar: Vec<BarElement> = song("C2:trp(q r q)> D2:h").unwrap();
        assert_eq!(bar[1].kind, ElementKind::Rest);
        assert_eq!(bar[0].marks.accent(), AccentLevel::Accent);
        assert_eq!(bar[2].marks.accent(), AccentLevel::Accent);
    }

    #[test]
    fn a_slash_or_a_note_with_no_length_plays_the_function_again() {
        // Song Notation, "Repeating": two triplets fill the bar, with D2 in the second.
        let slash: Vec<PlacedBar> = song_with_carry(&["C2:trp(q q q) /"]);
        assert_eq!(slash[0].len(), 6);
        assert_eq!(slash[0][3].0, 10_080);
        let note: Vec<PlacedBar> = song_with_carry(&["C2:trp(q q q) D2"]);
        assert_eq!(note[0].len(), 6);
        assert_eq!(note[0][3].3, Some(String::from("D2")));
    }

    #[test]
    fn a_written_length_ends_the_repeating_function() {
        let bars: Vec<PlacedBar> = song_with_carry(&["C2:trp(q q q) D2:q E2", "F2 G2 A2 B2"]);
        // After D2:q the length is a plain quarter: E2 and then the next bar's F2.
        assert_eq!(bars[0].len(), 5);
        assert_eq!(bars[1][0].1, 5040);
    }

    #[test]
    fn the_fit_functions_fill_their_length_on_a_song_line() {
        // Five eighths squeezed into one quarter (5040 ticks), then three quarters.
        let bar: Vec<PlacedBar> = song_with_carry(&["C2:fitQ(e e e e e) D2:q E2 F2"]);
        assert_eq!(bar[0].len(), 8);
        assert_eq!(bar[0][1].0, 1008);
        let whole_bar: Vec<BarElement> = song("C2:fitBar(q e e h)").unwrap();
        assert_eq!(whole_bar.len(), 4);
        assert_eq!(
            whole_bar
                .iter()
                .map(|element| element.length_ticks)
                .sum::<u64>(),
            20_160
        );
    }

    #[test]
    fn a_function_from_the_source_and_on_a_chord_and_mistakes_in_it() {
        let from_source: Vec<BarElement> = song(":trp(q q q) :h").unwrap();
        assert!(from_source[0].from_source);
        let chord: Vec<BarElement> = song("I:trp(q q q) /").unwrap();
        assert_eq!(chord[0].contents_text, Some(String::from("I")));
        assert!(song("C2:trp(q q q) r:h").is_ok());
        assert!(
            song("C2:trp(q> q q) D2:h")
                .unwrap_err()
                .contains("after the )")
        );
        assert!(song("C2:trp(q q q)~ D2:h").unwrap_err().contains("tie"));
        assert!(song("C2:trp(q q q D2:h").unwrap_err().contains("closing )"));
        assert!(
            song("C2:wobble(q q q) D2:h")
                .unwrap_err()
                .contains("not a function")
        );
    }

    /// What each element of a bar plays, in order.
    fn plays(bar: &PlacedBar) -> Vec<Option<&str>> {
        bar.iter()
            .map(|element| element.3.as_deref())
            .collect::<Vec<Option<&str>>>()
    }

    #[test]
    fn a_group_on_a_song_line_may_name_a_note_for_each_element() {
        // trp: three quarters in the time of a half note.
        let bar: PlacedBar = song_with_carry(&["trp(F2:q A2:q C2:q) D2:h"]).remove(0);
        assert_eq!(
            plays(&bar),
            vec![Some("F2"), Some("A2"), Some("C2"), Some("D2")]
        );
        assert_eq!(bar[1].0, 3360);
        assert_eq!(bar[3].0, 10_080);
        // A length left off inside is the one before it; the proportions hold.
        let proportions: PlacedBar = song_with_carry(&["fitH(F2:q A2 C2:h) G2:h"]).remove(0);
        assert_eq!(
            proportions
                .iter()
                .map(|element| (element.0, element.1))
                .collect::<Vec<(u64, u64)>>(),
            vec![(0, 2520), (2520, 2520), (5040, 5040), (10_080, 10_080)]
        );
        // fitQ with eighths: three in one quarter, 1680 ticks each.
        let fit: PlacedBar = song_with_carry(&["fitQ(F2:e A2:e C2:e) G2:q A2:h"]).remove(0);
        assert_eq!(fit[2].0, 3360);
        assert_eq!(fit[3].0, 5040);
        // A chord and a rest inside.
        let chords: PlacedBar = song_with_carry(&["trp(I:q r:q V:q) IV:h"]).remove(0);
        assert_eq!(chords[1].2, ElementKind::Rest);
        assert_eq!(chords[2].3, Some(String::from("V")));
    }

    #[test]
    fn a_function_after_a_colon_may_mix_bare_lengths_and_notes() {
        let bar: PlacedBar = song_with_carry(&["C2:trp(q E2:q q) G2:h"]).remove(0);
        assert_eq!(
            plays(&bar),
            vec![Some("C2"), Some("E2"), Some("C2"), Some("G2")]
        );
        let accented: Vec<BarElement> = song("C2:trp(q E2:q q)> G2:h").unwrap();
        assert!(accented[1].marks != ElementMarks::NONE);
        assert!(
            song("C2:trp(q E2:q> q) G2:h")
                .unwrap_err()
                .contains("after the )")
        );
    }

    #[test]
    fn a_slash_plays_a_group_again_with_its_notes() {
        let bars: Vec<PlacedBar> = song_with_carry(&["trp(F2:q A2:q C2:q) /"]);
        assert_eq!(
            plays(&bars[0]),
            vec![
                Some("F2"),
                Some("A2"),
                Some("C2"),
                Some("F2"),
                Some("A2"),
                Some("C2")
            ]
        );
        assert_eq!(bars[0][3].0, 10_080);
    }

    #[test]
    fn notes_with_no_lengths_in_a_fit_group_are_equal_parts() {
        // Nothing carried into the line: six equal parts of a half note
        // (10080 ticks), 1680 ticks each, then the same again.
        let bar: PlacedBar =
            song_with_carry(&["fitH(D2 F1 A1 D3 A2 F1) fitH(A1 C1 E2 A2 C2 E3)"]).remove(0);
        assert_eq!(bar.len(), 12);
        assert_eq!((bar[1].0, bar[1].1), (1680, 1680));
        assert_eq!(bar[6].0, 10_080);
        // A length written anywhere inside means the lengths are as written,
        // and the first element must write its own.
        assert!(song("fitH(D2 F1:q) G2:h").is_err());
        // A length carried in changes nothing: the parts are still equal.
        let carried: PlacedBar =
            song_with_carry(&["D1:q F1:e A1:e fitH(A1 C1 E2 A2 C2 E3)"]).remove(0);
        assert_eq!(carried[3].1, 1680);
    }

    #[test]
    fn an_octave_annotation_may_stand_before_any_element_in_a_bar() {
        let bar: Vec<BarElement> = song("C4:q -8va D4:q E4:h").unwrap();
        let shifts: Vec<i8> = bar.iter().map(|e| e.octave_shift_octaves).collect();
        assert_eq!(shifts, vec![0, -1, -1]);
        let back: Vec<BarElement> = song("+8va C4:q loco D4:q -8va E4 F4").unwrap();
        let shifts: Vec<i8> = back.iter().map(|e| e.octave_shift_octaves).collect();
        assert_eq!(shifts, vec![1, 0, -1, -1]);
        // After a chord and an eighth rest, as on a chord line.
        let chords: Vec<BarElement> = song("I:e r:e V:q -8va viidim:h").unwrap();
        let shifts: Vec<i8> = chords.iter().map(|e| e.octave_shift_octaves).collect();
        assert_eq!(shifts, vec![0, 0, 0, -1]);
    }

    #[test]
    fn mistakes_in_a_group_that_names_notes() {
        // Nothing for what is before the colon to play.
        assert!(
            song("C2:trp(F2:q A2:q C2:q) G2:h")
                .unwrap_err()
                .contains("plays nothing")
        );
        // A note with no length after a group that holds its own notes is an
        // ordinary element with the carried length (a quarter), not a replay.
        let after: PlacedBar = song_with_carry(&["trp(F2:q A2:q C2:q) D2 E2"]).remove(0);
        assert_eq!(after[3].1, 5040);
        assert_eq!(after.len(), 5);
        // A bare length with nothing before the colon to play.
        assert!(
            song("trp(q q q) D2:h")
                .unwrap_err()
                .contains("what it plays first")
        );
        // The first element writes its own length.
        assert!(song("trp(F2 A2:q C2:q) D2:h").is_err());
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
        assert_eq!(expected_start_ticks, four_four().bar_length().ticks());
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
    fn soft_and_softer_marks_are_read() {
        let elements: Vec<BarElement> = rhythm("q< q<< q q").unwrap();
        assert_eq!(elements[0].marks.accent(), AccentLevel::Soft);
        assert_eq!(elements[1].marks.accent(), AccentLevel::Softer);
        assert_eq!(elements[2].marks.accent(), AccentLevel::Normal);
    }

    #[test]
    fn a_short_bar_says_what_to_add() {
        assert_eq!(
            error_of("q q q e"),
            "bar 1 is 4/4 and short by an eighth note. Add e or er."
        );
    }

    #[test]
    fn a_long_bar_says_so() {
        assert_eq!(
            error_of("q q q q e"),
            "bar 1 is 4/4 and over by an eighth note. Shorten or remove an element."
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
        let mut carry: LineCarry = LineCarry::start_of_line();
        let bars: Vec<Vec<BarElement>> = parse_bars(
            "| C4:h D4 | E4 F4 |",
            0,
            &BarMeters::uniform(TimeSignature::new(4, 4).unwrap()),
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
            four_four(),
            BarNotation::SongLine,
            &mut LineCarry::start_of_line(),
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
