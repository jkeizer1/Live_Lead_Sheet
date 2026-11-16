//! Repeat signs inside a line: `|% I:w | V %|` plays the two bars twice,
//! `|%3 ... %|` three times (Song Notation, section 5, "Sections").
//!
//! The loader writes the bars out, as it does for `repeat`: the text of the
//! group is repeated before the bars are read, so a length carried by `/`
//! or left off, and a tie, continue across the sign as if the bars had been
//! written again.

/// The most plays a sign may ask for. A typing slip such as `|%2000` would
/// otherwise make a line of thousands of bars; no song needs more.
const MOST_PLAYS: u32 = 99;

/// Plays of a group when the opening sign carries no number.
const PLAYS_WITHOUT_NUMBER: u32 = 2;

/// The text of a line's bars with every repeat sign written out.
///
/// The `|` of an opening `|%` is the bar line before the group's first bar,
/// and the `|` of a closing `%|` is the bar line after its last bar, so
/// `| A |% B | C %| D |` becomes `| A | B | C | B | C | D |`.
///
/// # Errors
///
/// A message saying what is wrong with the signs and what to write: a group
/// opened and not closed, closed and not opened, opened inside another
/// group, with a number below 1 or above 99, or holding no bars.
pub fn write_out_repeat_signs(text: &str) -> Result<String, String> {
    let mut written: String = String::new();
    let mut rest: &str = text;
    while let Some(open_at) = find_opening(rest) {
        let (before, from_open): (&str, &str) = rest.split_at(open_at);
        if before.contains("%|") {
            return Err(closing_without_opening());
        }
        // Past "|%".
        let after_open: &str = &from_open[2..];
        let (plays, after_number): (u32, &str) = read_plays(after_open)?;
        let Some(close_at) = after_number.find("%|") else {
            return Err(String::from(
                "a repeat sign |% is opened and not closed. Close the bars to repeat with %|, \
                 e.g. |% I:w | V %|",
            ));
        };
        let group: &str = &after_number[..close_at];
        if find_opening(group).is_some() {
            return Err(String::from(
                "a repeat sign |% is opened inside another. Repeat signs do not nest; close the \
                 first with %| before opening the next",
            ));
        }
        if !group.contains(|character: char| !character.is_whitespace() && character != '|') {
            return Err(String::from(
                "there are no bars between |% and %|. Write the bars to repeat, e.g. |% I:w | V %|",
            ));
        }
        written.push_str(before);
        written.push('|');
        for play in 0..plays {
            if play > 0 {
                written.push('|');
            }
            written.push_str(group);
        }
        written.push('|');
        rest = &after_number[close_at + 2..];
    }
    if rest.contains("%|") {
        return Err(closing_without_opening());
    }
    written.push_str(rest);
    Ok(written)
}

/// Where the next `|%` starts. A `%|` is found by its own search, so `%||%`
/// (a close, then an open) reads as two signs.
fn find_opening(text: &str) -> Option<usize> {
    text.find("|%")
}

/// The number after an opening `|%` (`|%3`), or two plays when there is
/// none; and the text after the number.
fn read_plays(after_open: &str) -> Result<(u32, &str), String> {
    let digits_end: usize = after_open
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(after_open.len());
    let (digits, after_number): (&str, &str) = after_open.split_at(digits_end);
    if digits.is_empty() {
        return Ok((PLAYS_WITHOUT_NUMBER, after_number));
    }
    let plays: u32 = digits.parse::<u32>().map_err(|_| too_many(digits))?;
    if plays < 1 {
        return Err(String::from(
            "a repeat sign |%0 plays the bars no times. Use |%2 for twice, or take the bars out",
        ));
    }
    if plays > MOST_PLAYS {
        return Err(too_many(digits));
    }
    Ok((plays, after_number))
}

fn closing_without_opening() -> String {
    String::from(
        "a repeat sign %| closes a group that was not opened. Open it with |% before its \
         first bar, e.g. |% I:w | V %|",
    )
}

fn too_many(digits: &str) -> String {
    format!(
        "|%{digits} asks for too many plays. A repeat sign plays the bars at most {MOST_PLAYS} times"
    )
}
