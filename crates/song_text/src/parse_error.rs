//! A mistake in a text file, reported as EFS-6.3 asks: the file, the line,
//! what is wrong and what to type instead.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    file_name: String,
    line_number: usize,
    message: String,
}

impl ParseError {
    /// `line_number` counts from 1, as an editor shows it.
    #[must_use]
    pub fn new(file_name: &str, line_number: usize, message: String) -> ParseError {
        ParseError {
            file_name: file_name.to_string(),
            line_number,
            message,
        }
    }

    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    #[must_use]
    pub fn line_number(&self) -> usize {
        self.line_number
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} line {}: {}",
            self.file_name, self.line_number, self.message
        )
    }
}

impl std::error::Error for ParseError {}
