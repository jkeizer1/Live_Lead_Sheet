//! A line of text being typed: the characters and where the cursor is.
//! The window hands it typed characters and the editing keys; the render
//! crate draws it. It holds no window or file.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextField {
    text: String,
    /// The cursor's place counted in characters (not bytes), so it never
    /// lands inside a multi-byte character such as a sharp sign.
    cursor_chars: usize,
}

impl TextField {
    /// A field holding `text`, the cursor at its end.
    #[must_use]
    pub fn new(text: &str) -> TextField {
        TextField {
            text: text.to_string(),
            cursor_chars: text.chars().count(),
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn cursor_chars(&self) -> usize {
        self.cursor_chars
    }

    /// The text before the cursor, so the screen can place the cursor
    /// after its width.
    #[must_use]
    pub fn text_before_cursor(&self) -> &str {
        &self.text[..self.byte_offset(self.cursor_chars)]
    }

    /// Types characters at the cursor. Control characters (a typed Enter
    /// or Tab arrives as one) are dropped: they are keys, not text.
    pub fn insert(&mut self, typed: &str) {
        for character in typed.chars().filter(|c: &char| !c.is_control()) {
            let at: usize = self.byte_offset(self.cursor_chars);
            self.text.insert(at, character);
            self.cursor_chars += 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_chars == 0 {
            return;
        }
        self.cursor_chars -= 1;
        let at: usize = self.byte_offset(self.cursor_chars);
        self.text.remove(at);
    }

    pub fn delete(&mut self) {
        if self.cursor_chars < self.text.chars().count() {
            let at: usize = self.byte_offset(self.cursor_chars);
            self.text.remove(at);
        }
    }

    pub fn left(&mut self) {
        self.cursor_chars = self.cursor_chars.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.cursor_chars = (self.cursor_chars + 1).min(self.text.chars().count());
    }

    pub fn home(&mut self) {
        self.cursor_chars = 0;
    }

    pub fn end(&mut self) {
        self.cursor_chars = self.text.chars().count();
    }

    /// The byte position of the `chars`th character (the end of the text
    /// when `chars` is past it).
    fn byte_offset(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map_or(self.text.len(), |(offset, _)| offset)
    }
}

#[cfg(test)]
mod tests {
    use super::TextField;

    #[test]
    fn typing_goes_in_at_the_cursor() {
        let mut field: TextField = TextField::new("Bass");
        field.home();
        field.insert("Sub ");
        assert_eq!(field.text(), "Sub Bass");
        assert_eq!(field.cursor_chars(), 4);
    }

    #[test]
    fn backspace_and_delete_remove_either_side_of_the_cursor() {
        let mut field: TextField = TextField::new("Keys");
        field.left();
        field.backspace();
        assert_eq!(field.text(), "Kes");
        field.delete();
        assert_eq!(field.text(), "Ke");
        field.home();
        field.backspace();
        assert_eq!(field.text(), "Ke");
    }

    #[test]
    fn a_multi_byte_character_is_one_step() {
        let mut field: TextField = TextField::new("F♯ minor");
        field.home();
        field.right();
        field.right();
        assert_eq!(field.text_before_cursor(), "F♯");
        field.backspace();
        assert_eq!(field.text(), "F minor");
    }

    #[test]
    fn control_characters_are_not_text() {
        let mut field: TextField = TextField::new("");
        field.insert("a\nb\tc\r");
        assert_eq!(field.text(), "abc");
    }

    #[test]
    fn the_cursor_stays_inside_the_text() {
        let mut field: TextField = TextField::new("ab");
        field.right();
        field.right();
        assert_eq!(field.cursor_chars(), 2);
        field.home();
        field.left();
        assert_eq!(field.cursor_chars(), 0);
        field.end();
        assert_eq!(field.cursor_chars(), 2);
    }
}
