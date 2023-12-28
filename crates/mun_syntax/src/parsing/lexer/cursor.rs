use crate::TextSize;

use std::str::Chars;

/// A simple view into the characters of a string.
pub(crate) struct Cursor<'s> {
    text: &'s str,
    len: TextSize,
}

impl<'s> Cursor<'s> {
    /// Creates a new `Cursor` from a string.
    pub fn new(text: &'s str) -> Cursor<'s> {
        Cursor {
            text,
            len: 0.into(),
        }
    }

    /// Gets the length of the remaining string.
    pub fn into_len(self) -> TextSize {
        self.len
    }

    /// Gets the current character, if one exists
    pub fn current(&self) -> Option<char> {
        self.chars().next()
    }

    /// Gets the nth character from the current offset. For example, 0 will return the current
    /// character, 1 will return the next, etc.
    pub fn nth(&self, n: u32) -> Option<char> {
        self.chars().nth(n as usize)
    }

    /// Checks whether the current character is the specified character.
    pub fn matches(&self, c: char) -> bool {
        self.current() == Some(c)
    }

    /// Checks whether the current characters match the specified string.
    pub fn matches_str(&self, s: &str) -> bool {
        let chars = self.chars();
        chars.as_str().starts_with(s)
    }

    //    /// Checks whether the current character satisfies the specified predicate
    //    pub fn matches_if<F: Fn(char) -> bool>(&self, predicate: F) -> bool {
    //        self.current().map(predicate) == Some(true)
    //    }

    /// Checks whether the nth character satisfies the specified predicate
    pub fn matches_nth_if<F: Fn(char) -> bool>(&self, n: u32, predicate: F) -> bool {
        self.nth(n).map(predicate) == Some(true)
    }

    /// Move to the next character
    pub fn bump(&mut self) -> Option<char> {
        let ch = self.chars().next()?;
        self.len += TextSize::of(ch);
        Some(ch)
    }

    /// Moves to the next character as long as `predicate` is satisfied.
    pub fn bump_while<F: Fn(char) -> bool>(&mut self, predicate: F) {
        loop {
            match self.current() {
                Some(c) if predicate(c) => {
                    self.bump();
                }
                _ => return,
            }
        }
    }

    /// Returns the text up to the current point.
    pub fn current_token_text(&self) -> &str {
        let len: u32 = self.len.into();
        &self.text[..len as usize]
    }

    /// Returns an iterator over the remaining characters.
    fn chars(&self) -> Chars<'_> {
        let len: u32 = self.len.into();
        self.text[len as usize..].chars()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current() {
        let cursor = Cursor::new("hello");
        assert_eq!(cursor.current(), Some('h'));
    }

    #[test]
    fn test_nth() {
        let cursor = Cursor::new("hello");
        assert_eq!(cursor.nth(0), Some('h'));
        assert_eq!(cursor.nth(1), Some('e'));
        assert_eq!(cursor.nth(2), Some('l'));
        assert_eq!(cursor.nth(3), Some('l'));
        assert_eq!(cursor.nth(4), Some('o'));
        assert_eq!(cursor.nth(5), None);
    }

    #[test]
    fn test_matches() {
        let cursor = Cursor::new("hello");
        assert!(cursor.matches('h'));
        assert!(!cursor.matches('t'));
    }

    #[test]
    fn test_matches_str() {
        let cursor = Cursor::new("hello");
        assert!(cursor.matches_str("h"));
        assert!(cursor.matches_str("he"));
        assert!(cursor.matches_str("hel"));
        assert!(cursor.matches_str("hello"));
        assert!(!cursor.matches_str("world"));
    }

    //    #[test]
    //    fn test_matches_if() {
    //        let cursor = Cursor::new("hello");
    //        assert!(cursor.matches_if(|c| c == 'h'));
    //        assert!(!cursor.matches_if(|c| c == 't'));
    //    }

    #[test]
    fn test_matches_nth_if() {
        let cursor = Cursor::new("hello");
        assert!(cursor.matches_nth_if(0, |c| c == 'h'));
        assert!(!cursor.matches_nth_if(1, |c| c == 'h'));
        assert!(cursor.matches_nth_if(4, |c| c == 'o'));
        assert!(!cursor.matches_nth_if(400, |c| c == 'h'));
    }

    #[test]
    fn test_bump() {
        let mut cursor = Cursor::new("hello");
        assert_eq!(cursor.current(), Some('h'));
        cursor.bump();
        assert_eq!(cursor.current(), Some('e'));
        cursor.bump();
        assert_eq!(cursor.current(), Some('l'));
        cursor.bump();
        assert_eq!(cursor.current(), Some('l'));
        cursor.bump();
        assert_eq!(cursor.current(), Some('o'));
        cursor.bump();
        assert_eq!(cursor.current(), None);
        cursor.bump();
        assert_eq!(cursor.current(), None);
    }

    #[test]
    fn test_bump_while() {
        let mut cursor = Cursor::new("hello");
        assert_eq!(cursor.current(), Some('h'));
        cursor.bump_while(|c| c != 'o');
        assert_eq!(cursor.current(), Some('o'));
    }
}
