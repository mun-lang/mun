//! This file contains a helpful builders for structs from [`annotate-snippets`] crate.
//!
//! [`annotate-snippets`]: https://docs.rs/annotate-snippets/0.6.1/annotate_snippets/

use annotate_snippets::snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation};
use mun_hir::line_index::LineIndex;

use unicode_segmentation::UnicodeSegmentation;

pub struct SnippetBuilder {
    snippet: Snippet,
}

impl Default for SnippetBuilder {
    fn default() -> Self {
        SnippetBuilder {
            snippet: Snippet {
                title: None,
                footer: vec![],
                slices: vec![],
            },
        }
    }
}

impl SnippetBuilder {
    pub fn new() -> SnippetBuilder {
        SnippetBuilder::default()
    }
    pub fn title(mut self, title: Annotation) -> SnippetBuilder {
        self.snippet.title = Some(title);
        self
    }
    pub fn footer(mut self, footer: Annotation) -> SnippetBuilder {
        self.snippet.footer.push(footer);
        self
    }
    pub fn slice(mut self, slice: Slice) -> SnippetBuilder {
        self.snippet.slices.push(slice);
        self
    }
    pub fn build(self) -> Snippet {
        self.snippet
    }
}

pub struct SliceBuilder {
    slice: Slice,
}

impl SliceBuilder {
    pub fn new(fold: bool) -> SliceBuilder {
        SliceBuilder {
            slice: Slice {
                source: String::new(),
                line_start: 0,
                origin: None,
                annotations: Vec::new(),
                fold,
            },
        }
    }

    pub fn origin(mut self, relative_file_path: String) -> SliceBuilder {
        self.slice.origin = Some(relative_file_path);
        self
    }

    pub fn source_annotation(
        mut self,
        range: (usize, usize),
        label: String,
        source_annotation_type: AnnotationType,
    ) -> SliceBuilder {
        self.slice.annotations.push(SourceAnnotation {
            range,
            label,
            annotation_type: source_annotation_type,
        });
        self
    }

    pub fn build(
        mut self,
        source_text: &str,
        source_text_len: usize,
        line_index: &LineIndex,
    ) -> Slice {
        // Variable for storing first and last line of the needed source code part
        let mut fl_lines: Option<(u32, u32)> = None;

        // Finding borders of source code part that include all highlight ranges
        for annotation in &self.slice.annotations {
            if let Some(range) = fl_lines {
                fl_lines = Some((
                    line_index
                        .line_col(range.0.into())
                        .line
                        .min(line_index.line_col((annotation.range.0 as u32).into()).line),
                    line_index
                        .line_col(range.1.into())
                        .line
                        .max(line_index.line_col((annotation.range.1 as u32).into()).line),
                ));
            } else {
                fl_lines = Some((
                    line_index.line_col((annotation.range.0 as u32).into()).line,
                    line_index.line_col((annotation.range.1 as u32).into()).line,
                ));
            }
        }

        if let Some(fl_lines) = fl_lines {
            self.slice.line_start = fl_lines.0 as usize + 1;
            let first_line_offset = line_index.line_offset(fl_lines.0);

            // Cutting needed part from source code
            self.slice.source = line_index
                .text_part(fl_lines.0, fl_lines.1, source_text, source_text_len)
                .unwrap()
                .to_string();

            // Recalculating every annotation range by taking in account cutted off source text and unicode graphemes
            for annotation in self.slice.annotations.iter_mut() {
                annotation.range = (
                    UnicodeSegmentation::graphemes(
                        &self.slice.source[0..(annotation.range.0 as usize - first_line_offset)],
                        true,
                    )
                    .count(),
                    annotation.range.0 as usize - first_line_offset
                        + UnicodeSegmentation::graphemes(
                            &self.slice.source[annotation.range.0 as usize - first_line_offset
                                ..(annotation.range.1 as usize - first_line_offset)],
                            true,
                        )
                        .count()
                        + (fl_lines.1 - fl_lines.0) as usize, // that line is a temporary fix for annotate-snippets issue number 24
                );
            }
        }
        self.slice
    }
}

pub struct AnnotationBuilder {
    annotation: Annotation,
}

impl AnnotationBuilder {
    pub fn new(annotation_type: AnnotationType) -> AnnotationBuilder {
        AnnotationBuilder {
            annotation: Annotation {
                id: None,
                label: None,
                annotation_type,
            },
        }
    }

    pub fn id(mut self, id: String) -> AnnotationBuilder {
        self.annotation.id = Some(id);
        self
    }

    pub fn label(mut self, label: String) -> AnnotationBuilder {
        self.annotation.label = Some(label);
        self
    }

    pub fn build(self) -> Annotation {
        self.annotation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn annotation_builder_snapshot() {
        insta::assert_debug_snapshot!(AnnotationBuilder::new(AnnotationType::Note)
            .id("1".to_string())
            .label("test annotation".to_string())
            .build());
    }
    #[test]
    fn slice_builder_snapshot() {
        let source_code = "fn foo():float{\n48\n}";
        let line_index: LineIndex = LineIndex::new(source_code);

        insta::assert_debug_snapshot!(SliceBuilder::new(true)
            .origin("/tmp/usr/test.mun".to_string())
            .source_annotation(
                (14, 20),
                "test source annotation".to_string(),
                AnnotationType::Note
            )
            .build(source_code, source_code.len(), &line_index));
    }
    #[test]
    fn snippet_builder_snapshot() {
        let title = AnnotationBuilder::new(AnnotationType::Note)
            .id("1".to_string())
            .label("test annotation".to_string())
            .build();
        let footer = AnnotationBuilder::new(AnnotationType::Warning)
            .id("2".to_string())
            .label("test annotation".to_string())
            .build();

        let source_code = "fn foo():float{\n48\n}";
        let line_index: LineIndex = LineIndex::new(source_code);

        let slice = SliceBuilder::new(true)
            .origin("/tmp/usr/test.mun".to_string())
            .source_annotation(
                (14, 20),
                "test source annotation".to_string(),
                AnnotationType::Note,
            )
            .build(source_code, source_code.len(), &line_index);

        insta::assert_debug_snapshot!(SnippetBuilder::new()
            .title(title)
            .footer(footer)
            .slice(slice)
            .build());
    }
}
