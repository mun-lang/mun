use mun_syntax::TextUnit;

use superslice::Ext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    newlines: Vec<TextUnit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl LineIndex {
    pub fn new(text: &str) -> LineIndex {
        let mut newlines = vec![0.into()];
        let mut curr_row = 0.into();
        for c in text.chars() {
            curr_row += TextUnit::of_char(c);
            if c == '\n' {
                newlines.push(curr_row);
            }
        }

        LineIndex { newlines }
    }

    pub fn line_col(&self, offset: TextUnit) -> LineCol {
        let line = self.newlines.upper_bound(&offset) - 1;
        let line_start_offset = self.newlines[line];
        let col = offset - line_start_offset;

        LineCol {
            line: line as u32,
            col: col.to_usize() as u32,
        }
    }

    /// Get part of text between two lines
    pub fn text_part<'a>(
        &self,
        first_line: u32,
        last_line: u32,
        text: &'a str,
        text_len: usize,
    ) -> Option<&'a str> {
        let start_of_part = self.newlines.get(first_line as usize)?.to_usize();
        let end_of_part = self
            .newlines
            .get(last_line as usize + 1)
            .map(|u| u.to_usize() - 1)
            .unwrap_or(text_len);
        Some(&text[start_of_part..end_of_part])
    }

    /// Get offset to specific line
    #[inline]
    pub fn line_offset(&self, line_index: u32) -> usize {
        self.newlines[line_index as usize].to_usize()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_index() {
        let text = "hello\nworld";
        let index = LineIndex::new(text);
        assert_eq!(index.line_col(0.into()), LineCol { line: 0, col: 0 });
        assert_eq!(index.line_col(1.into()), LineCol { line: 0, col: 1 });
        assert_eq!(index.line_col(5.into()), LineCol { line: 0, col: 5 });
        assert_eq!(index.line_col(6.into()), LineCol { line: 1, col: 0 });
        assert_eq!(index.line_col(7.into()), LineCol { line: 1, col: 1 });
    }
    #[test]
    fn test_text_part() {
        let text = "ℱ٥ℜ\n†ěṦτ\nℙน尺קő$ع";
        let text_len = text.len();
        let index = LineIndex::new(text);
        assert_eq!(index.text_part(0, 0, &text, text_len), Some("ℱ٥ℜ"));
        assert_eq!(index.text_part(0, 1, &text, text_len), Some("ℱ٥ℜ\n†ěṦτ"));
        assert_eq!(
            index.text_part(1, 2, &text, text_len),
            Some("†ěṦτ\nℙน尺קő$ع")
        );
        assert_eq!(index.text_part(0, 2, &text, text_len), Some(text));
    }
    #[test]
    fn test_line_offset() {
        let text = "for\ntest\npurpose";
        let index = LineIndex::new(text);
        assert_eq!(index.line_offset(0), 0);
        assert_eq!(index.line_offset(1), 4);
        assert_eq!(index.line_offset(2), 9);
    }
}
