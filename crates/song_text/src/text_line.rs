//! Reading one line of a Song Notation file: comments, keywords, quoted
//! names, meters and the words of a settings line (Song Notation,
//! section 2). Shared by the rhythm library, song and studio parsers.

use vocabulary::time_signature::TimeSignature;

/// A word of a settings line, with whether it was written in quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub quoted: bool,
}

/// `4/4` → a time signature.
///
/// # Errors
///
/// A message saying how to write the meter.
pub fn parse_meter(text: &str) -> Result<TimeSignature, String> {
    let wrong = || String::from("write the meter as two numbers, e.g. meter 4/4 or meter 6/8");
    let Some((pulses_text, note_text)) = text.trim().split_once('/') else {
        return Err(wrong());
    };
    let Ok(pulses_per_bar) = pulses_text.trim().parse::<u8>() else {
        return Err(wrong());
    };
    let Ok(pulses_per_whole_note) = note_text.trim().parse::<u8>() else {
        return Err(wrong());
    };
    TimeSignature::new(pulses_per_bar, pulses_per_whole_note).map_err(|error| error.to_string())
}

/// The text after `keyword` and a space, when the line starts with it.
#[must_use]
pub fn keyword_rest<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let rest: &str = line.strip_prefix(keyword)?;
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest.trim())
    } else {
        None
    }
}

/// `"Swing"` → `Swing`; nothing when it is not exactly one quoted name.
#[must_use]
pub fn quoted(text: &str) -> Option<&str> {
    let inner: &str = text.trim().strip_prefix('"')?.strip_suffix('"')?;
    if inner.is_empty() || inner.contains('"') {
        None
    } else {
        Some(inner)
    }
}

/// `"Keys"  rest of line` → (`Keys`, `rest of line`); nothing when the text
/// does not start with a quoted name.
#[must_use]
pub fn quoted_prefix(text: &str) -> Option<(&str, &str)> {
    let after_quote: &str = text.trim_start().strip_prefix('"')?;
    let end: usize = after_quote.find('"')?;
    let name: &str = &after_quote[..end];
    if name.is_empty() {
        None
    } else {
        Some((name, after_quote[end + 1..].trim()))
    }
}

/// Drops comments (Song Notation, section 2). A line whose first
/// character, after indentation, is `#` is a comment as a whole. After
/// content, `---` starts a comment to the end of the line, except inside a
/// quoted name. A `#` anywhere but first is a sharp (`F#3`, `C#4:q`,
/// `#iv°`, `key F# major`), never a comment.
#[must_use]
pub fn without_comment(line: &str) -> &str {
    if line.trim_start().starts_with('#') {
        return "";
    }
    let mut inside_quotes: bool = false;
    for (position, character) in line.char_indices() {
        match character {
            '"' => inside_quotes = !inside_quotes,
            '-' if !inside_quotes && line[position..].starts_with(END_OF_LINE_COMMENT) => {
                return &line[..position];
            }
            _ => {}
        }
    }
    line
}

/// Starts a comment after content on a line.
pub const END_OF_LINE_COMMENT: &str = "---";

/// Splits a line into words and quoted names: `cue "Start SQ-1" start`.
///
/// # Errors
///
/// When a quote is not closed.
pub fn words(line: &str) -> Result<Vec<Word>, String> {
    let mut result: Vec<Word> = Vec::new();
    let mut characters = line.chars().peekable();
    while let Some(&character) = characters.peek() {
        if character.is_whitespace() {
            characters.next();
        } else if character == '"' {
            characters.next();
            let mut text: String = String::new();
            loop {
                match characters.next() {
                    Some('"') => break,
                    Some(inner) => text.push(inner),
                    None => {
                        return Err(format!(
                            "the quote in `{}` is not closed. Add a closing \"",
                            line.trim()
                        ));
                    }
                }
            }
            result.push(Word { text, quoted: true });
        } else {
            let mut text: String = String::new();
            while let Some(&inner) = characters.peek() {
                if inner.is_whitespace() || inner == '"' {
                    break;
                }
                text.push(inner);
                characters.next();
            }
            result.push(Word {
                text,
                quoted: false,
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_keep_quoted_names_whole() {
        let result: Vec<Word> = words("cue \"Start SQ-1\"  start").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[1].text, "Start SQ-1");
        assert!(result[1].quoted);
        assert!(!result[2].quoted);
    }

    #[test]
    fn an_open_quote_is_reported() {
        assert!(words("cue \"Start").unwrap_err().contains("not closed"));
    }

    #[test]
    fn a_comment_is_a_whole_line_starting_with_a_hash() {
        assert_eq!(without_comment("# C – G – Am – F"), "");
        assert_eq!(without_comment("  #################"), "");
        assert_eq!(without_comment("#                C"), "");
    }

    #[test]
    fn a_hash_after_content_is_a_sharp() {
        assert_eq!(
            without_comment("| F#3:q C#4 G#2 A3 |"),
            "| F#3:q C#4 G#2 A3 |"
        );
        assert_eq!(without_comment("| #iv°:h I |"), "| #iv°:h I |");
        assert_eq!(without_comment("  key F# major"), "  key F# major");
        // Not a comment: it stays, and the parser reports it.
        assert_eq!(without_comment("| r |  # note"), "| r |  # note");
    }

    #[test]
    fn three_hyphens_start_a_comment_after_content() {
        assert_eq!(
            without_comment("  line \"Cues\"  | \"Start SQ-1\":w | r |  --- fires each pass"),
            "  line \"Cues\"  | \"Start SQ-1\":w | r |  "
        );
        assert_eq!(without_comment("  key F# major---bright"), "  key F# major");
        // Inside a quoted name they are part of the name.
        assert_eq!(
            without_comment("  cue \"A---B\" destination \"X\" start"),
            "  cue \"A---B\" destination \"X\" start"
        );
        // A negative octave is one hyphen, not a comment.
        assert_eq!(without_comment("| C-1:w |"), "| C-1:w |");
    }
}
