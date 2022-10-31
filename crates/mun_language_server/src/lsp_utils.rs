use crate::from_lsp;
use mun_hir::line_index::LineIndex;

/// Given a set of text document changes apply them to the given string.
pub(crate) fn apply_document_changes(
    old_text: &mut String,
    content_changes: Vec<lsp_types::TextDocumentContentChangeEvent>,
) {
    // The changes are specified with ranges where they apply. These ranges are given as line-column
    // pairs. We can compute the offset in the text using a `LineIndex` however, changes to the text
    // may invalidate this too.
    // As a simple optimization we keep track of the lines that are possibly invalid in the
    // LineIndex based on where we insert new text. If a changes is within the invalid range we
    // recompute the LineIndex. Some clients (e.g. Code) sort the ranges in reverse which should
    // ensure that we almost never invalidate the LineIndex.

    let mut line_index = LineIndex::new(old_text);

    enum IndexValid {
        All,
        UpToLineExclusive(u32),
    }

    impl IndexValid {
        fn covers(&self, line: u32) -> bool {
            match *self {
                IndexValid::UpToLineExclusive(to) => to > line,
                _ => true,
            }
        }
    }

    let mut index_valid = IndexValid::All;
    for change in content_changes {
        match change.range {
            Some(range) => {
                if !index_valid.covers(range.end.line) {
                    line_index = LineIndex::new(old_text);
                }
                index_valid = IndexValid::UpToLineExclusive(range.start.line);
                let range = from_lsp::text_range(&line_index, range);
                old_text.replace_range(std::ops::Range::<usize>::from(range), &change.text);
            }
            None => {
                *old_text = change.text;
                index_valid = IndexValid::UpToLineExclusive(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lsp_utils::apply_document_changes;
    use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

    #[test]
    fn test_apply_document_changes() {
        macro_rules! change {
            [$($sl:expr, $sc:expr; $el:expr, $ec:expr => $text:expr),+] => {
                vec![$(TextDocumentContentChangeEvent {
                    range: Some(Range {
                        start: Position { line: $sl, character: $sc },
                        end: Position { line: $el, character: $ec },
                    }),
                    range_length: None,
                    text: String::from($text),
                }),+]
            };
        }

        let mut text = String::new();
        apply_document_changes(&mut text, vec![]);
        assert_eq!(text, "");

        // Test if full updates work (without a range)
        apply_document_changes(
            &mut text,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: String::from("the"),
            }],
        );
        assert_eq!(text, "the");
        apply_document_changes(&mut text, change![0, 3; 0, 3 => " quick"]);
        assert_eq!(text, "the quick");
        apply_document_changes(&mut text, change![0, 0; 0, 4 => "", 0, 5; 0, 5 => " foxes"]);
        assert_eq!(text, "quick foxes");
        apply_document_changes(&mut text, change![0, 11; 0, 11 => "\ndream"]);
        assert_eq!(text, "quick foxes\ndream");
        apply_document_changes(&mut text, change![1, 0; 1, 0 => "have "]);
        assert_eq!(text, "quick foxes\nhave dream");
        apply_document_changes(
            &mut text,
            change![0, 0; 0, 0 => "the ", 1, 4; 1, 4 => " quiet", 1, 16; 1, 16 => "s\n"],
        );
        assert_eq!(text, "the quick foxes\nhave quiet dreams\n");
        apply_document_changes(
            &mut text,
            change![0, 15; 0, 15 => "\n", 2, 17; 2, 17 => "\n"],
        );
        assert_eq!(text, "the quick foxes\n\nhave quiet dreams\n\n");
        apply_document_changes(
            &mut text,
            change![1, 0; 1, 0 => "DREAM", 2, 0; 2, 0 => "they ", 3, 0; 3, 0 => "DON'T THEY?"],
        );
        assert_eq!(
            text,
            "the quick foxes\nDREAM\nthey have quiet dreams\nDON'T THEY?\n"
        );
        apply_document_changes(&mut text, change![0, 10; 1, 5 => "", 2, 0; 2, 12 => ""]);
        assert_eq!(text, "the quick \nthey have quiet dreams\n");

        text = String::from("❤️");
        apply_document_changes(&mut text, change![0, 0; 0, 0 => "a"]);
        assert_eq!(text, "a❤️");

        text = String::from("a\nb");
        apply_document_changes(&mut text, change![0, 1; 1, 0 => "\nțc", 0, 1; 1, 1 => "d"]);
        assert_eq!(text, "adcb");

        text = String::from("a\nb");
        apply_document_changes(&mut text, change![0, 1; 1, 0 => "ț\nc", 0, 2; 0, 2 => "c"]);
        assert_eq!(text, "ațc\ncb");
    }
}
