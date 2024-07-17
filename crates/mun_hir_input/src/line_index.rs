use mun_syntax::TextSize;
use rustc_hash::FxHashMap;

/// A [`LineIndex`] enables efficient mapping between offsets and line/column
/// positions in a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    /// Offsets from the beginning of each line
    newlines: Vec<TextSize>,

    /// List of non-ASCII characters on each line
    utf16_lines: FxHashMap<u32, Vec<Utf16Char>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineCol {
    /// The line index (zero-based)
    pub line: u32,

    /// The column index when the text is represented as UTF16 text (zero-based)
    pub col_utf16: u32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Utf16Char {
    /// Start offset of a character inside a line, zero-based
    pub(crate) start: TextSize,

    /// End offset of a character inside a line, zero-based
    pub(crate) end: TextSize,
}

impl Utf16Char {
    /// Returns the length in 8-bit UTF-8 code units.
    fn len(&self) -> TextSize {
        self.end - self.start
    }

    /// Returns the length in 16-bit UTF-16 code units.
    fn len_utf16(&self) -> usize {
        if self.len() == TextSize::from(4) {
            2
        } else {
            1
        }
    }
}

impl LineIndex {
    /// Constructs a new [`LineIndex`] from the given text.
    pub fn new(text: &str) -> LineIndex {
        let mut utf16_lines = FxHashMap::default();
        let mut utf16_chars = Vec::new();

        // Iterate over all the characters in the text and record all the newlines and
        // UTF16 characters.
        let mut newlines = vec![0.into()];
        let mut curr_row = 0.into();
        let mut curr_col = 0.into();
        let mut line = 0;
        for c in text.chars() {
            let c_len = TextSize::of(c);
            curr_row += c_len;
            if c == '\n' {
                newlines.push(curr_row);

                // Save any utf-16 characters seen in the previous line
                if !utf16_chars.is_empty() {
                    utf16_lines.insert(line, utf16_chars);
                    utf16_chars = Vec::new();
                }

                // Prepare for processing the next line
                curr_col = 0.into();
                line += 1;
                continue;
            }

            if !c.is_ascii() {
                utf16_chars.push(Utf16Char {
                    start: curr_col,
                    end: curr_col + c_len,
                });
            }

            curr_col += c_len;
        }

        // Save any utf-16 characters seen in the last line
        if !utf16_chars.is_empty() {
            utf16_lines.insert(line, utf16_chars);
        }

        LineIndex {
            newlines,
            utf16_lines,
        }
    }

    /// Returns the line and column index at the given offset in the text
    pub fn line_col(&self, offset: TextSize) -> LineCol {
        let line = self
            .newlines
            .binary_search_by(|x| {
                if x <= &offset {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                }
            })
            .unwrap_or_else(|i| i)
            - 1;
        let line_start_offset = self.newlines[line];
        let col = offset - line_start_offset;

        LineCol {
            line: line as u32,
            col_utf16: self.utf8_to_utf16_col(line as u32, col) as u32,
        }
    }

    /// Returns the offset in the text for the given line and column index
    pub fn offset(&self, line_col: LineCol) -> TextSize {
        let col = self.utf16_to_utf8_col(line_col.line, line_col.col_utf16);
        self.newlines[line_col.line as usize] + col
    }

    /// Retrieves the text between `first_line` and `last_line`, if any.
    pub fn text_part<'a>(
        &self,
        first_line: u32,
        last_line: u32,
        text: &'a str,
        text_len: usize,
    ) -> Option<&'a str> {
        let start_of_part = (*self.newlines.get(first_line as usize)?).into();
        let end_of_part = self
            .newlines
            .get(last_line as usize + 1)
            .map_or(text_len, |u| usize::from(*u) - 1usize);
        Some(&text[start_of_part..end_of_part])
    }

    /// Retrieves the offset to the line corresponding to `line_index`.
    #[inline]
    pub fn line_offset(&self, line_index: u32) -> usize {
        self.newlines[line_index as usize].into()
    }

    /// Given a line and column number for utf16 text convert it to the offset
    /// in utf8 text.
    fn utf16_to_utf8_col(&self, line: u32, mut col: u32) -> TextSize {
        if let Some(utf16_chars) = self.utf16_lines.get(&line) {
            for c in utf16_chars {
                if col > u32::from(c.start) {
                    col += u32::from(c.len()) - c.len_utf16() as u32;
                } else {
                    // From here on, all utf16 characters come *after* the character we are mapping,
                    // so we don't need to take them into account
                    break;
                }
            }
        }

        col.into()
    }

    /// Given a line and column number for utf8 text, convert it to the offset
    /// in utf16 text.
    fn utf8_to_utf16_col(&self, line: u32, col: TextSize) -> usize {
        let mut res: usize = col.into();
        if let Some(utf16_chars) = self.utf16_lines.get(&line) {
            for c in utf16_chars {
                if c.end <= col {
                    res -= usize::from(c.len()) - c.len_utf16();
                } else {
                    // From here on, all utf16 characters come *after* the character we are mapping,
                    // so we don't need to take them into account
                    break;
                }
            }
        }
        res
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_index() {
        let text = "hello\nworld";
        let index = LineIndex::new(text);
        assert_eq!(
            index.line_col(0.into()),
            LineCol {
                line: 0,
                col_utf16: 0
            }
        );
        assert_eq!(
            index.line_col(1.into()),
            LineCol {
                line: 0,
                col_utf16: 1
            }
        );
        assert_eq!(
            index.line_col(5.into()),
            LineCol {
                line: 0,
                col_utf16: 5
            }
        );
        assert_eq!(
            index.line_col(6.into()),
            LineCol {
                line: 1,
                col_utf16: 0
            }
        );
        assert_eq!(
            index.line_col(7.into()),
            LineCol {
                line: 1,
                col_utf16: 1
            }
        );
    }
    #[test]
    fn test_text_part() {
        let text = "ℱ٥ℜ\n†ěṦτ\nℙน尺קő$ع";
        let text_len = text.len();
        let index = LineIndex::new(text);
        assert_eq!(index.text_part(0, 0, text, text_len), Some("ℱ٥ℜ"));
        assert_eq!(index.text_part(0, 1, text, text_len), Some("ℱ٥ℜ\n†ěṦτ"));
        assert_eq!(
            index.text_part(1, 2, text, text_len),
            Some("†ěṦτ\nℙน尺קő$ع")
        );
        assert_eq!(index.text_part(0, 2, text, text_len), Some(text));
    }
    #[test]
    fn test_text_part_utf16() {
        let text = "a\n❤️\nb";
        let index = LineIndex::new(text);
        let start = index.offset(LineCol {
            line: 1,
            col_utf16: 0,
        });
        let end = index.offset(LineCol {
            line: 1,
            col_utf16: 1,
        });
        assert_eq!(
            index.text_part(1, 1, text, (end - start).into()),
            Some("❤️")
        );
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
