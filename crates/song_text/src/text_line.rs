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

/// Drops a `#` comment, unless the `#` is inside a quoted name ("F#").
#[must_use]
pub fn without_comment(line: &str) -> &str {
    let mut inside_quotes: bool = false;
    for (position, character) in line.char_indices() {
        match character {
            '"' => inside_quotes = !inside_quotes,
            '#' if !inside_quotes => return &line[..position],
            _ => {}
        }
    }
    line
}

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
    fn a_hash_inside_quotes_is_not_a_comment() {
        assert_eq!(without_comment("key \"F#\" # note"), "key \"F#\" ");
    }
}
