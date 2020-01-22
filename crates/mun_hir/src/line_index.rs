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

    pub fn line_str<'a>(&self, line: u32, text: &'a str) -> Option<&'a str> {
        let start_of_line = self.newlines.get(line as usize)?.to_usize();
        let end_of_line = self
            .newlines
            .get((line + 1) as usize)
            .map(|u| u.to_usize() - 1)
            .unwrap_or(text.len() as usize);
        Some(&text[start_of_line..end_of_line])
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
    fn test_line_str() {
        let text = "ℱ٥ℜ\n†ěṦτ\nℙน尺קő$ع";
        let index = LineIndex::new(text);
        assert_eq!(index.line_str(0, &text), Some("ℱ٥ℜ"));
        assert_eq!(index.line_str(1, &text), Some("†ěṦτ"));
        assert_eq!(index.line_str(2, &text), Some("ℙน尺קő$ع"));
        assert_eq!(index.line_str(3, &text), None);
    }
}
