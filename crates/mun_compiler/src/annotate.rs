//! This module provides builders for integrating the [`annotate-snippets`] crate with Mun.
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

    pub fn origin(mut self, relative_file_path: &str) -> SliceBuilder {
        self.slice.origin = Some(relative_file_path.to_string());
        self
    }

    pub fn source_annotation(
        mut self,
        range: (usize, usize),
        label: &str,
        source_annotation_type: AnnotationType,
    ) -> SliceBuilder {
        self.slice.annotations.push(SourceAnnotation {
            range,
            label: label.to_string(),
            annotation_type: source_annotation_type,
        });
        self
    }

    pub fn build(mut self, source_text: &str, line_index: &LineIndex) -> Slice {
        // Variable for storing the first and last line of the used source code
        let mut fl_lines: Option<(u32, u32)> = None;

        // Find the range of lines that include all highlighted segments
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

            // Extract the required range of lines
            self.slice.source = line_index
                .text_part(fl_lines.0, fl_lines.1, source_text, source_text.len())
                .unwrap()
                .to_string();

            // Convert annotation ranges based on the cropped region, indexable by unicode
            // graphemes (required for aligned annotations)
            let convertor_function = |source: &String, annotation_range_border: usize| {
                UnicodeSegmentation::graphemes(
                &source[0..(annotation_range_border - first_line_offset)],
                true).count()
                // this addend is a fix for annotate-snippets issue number 24
                + (line_index.line_col((annotation_range_border as u32).into()).line
                    - fl_lines.0) as usize
            };
            for annotation in self.slice.annotations.iter_mut() {
                annotation.range = (
                    convertor_function(&self.slice.source, annotation.range.0),
                    convertor_function(&self.slice.source, annotation.range.1),
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

    pub fn id(mut self, id: &str) -> AnnotationBuilder {
        self.annotation.id = Some(id.to_string());
        self
    }

    pub fn label(mut self, label: &str) -> AnnotationBuilder {
        self.annotation.label = Some(label.to_string());
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
            .id("1")
            .label("test annotation")
            .build());
    }
    #[test]
    fn slice_builder_snapshot() {
        let source_code = "fn foo()->f64{\n48\n}";
        let line_index: LineIndex = LineIndex::new(source_code);

        insta::assert_debug_snapshot!(SliceBuilder::new(true)
            .origin("/tmp/usr/test.mun")
            .source_annotation((13, 19), "test source annotation", AnnotationType::Note)
            .build(source_code, &line_index));
    }
    #[test]
    fn snippet_builder_snapshot() {
        let source_code = "fn foo()->f64{\n48\n}\n\nfn bar()->bool{\n23\n}";
        let line_index: LineIndex = LineIndex::new(source_code);

        insta::assert_debug_snapshot!(SnippetBuilder::new()
            .title(
                AnnotationBuilder::new(AnnotationType::Note)
                    .id("1")
                    .label("test annotation")
                    .build()
            )
            .footer(
                AnnotationBuilder::new(AnnotationType::Warning)
                    .id("2")
                    .label("test annotation")
                    .build()
            )
            .slice(
                SliceBuilder::new(true)
                    .origin("/tmp/usr/test.mun")
                    .source_annotation((14, 20), "test source annotation", AnnotationType::Note,)
                    .source_annotation((35, 41), "test source annotation", AnnotationType::Error,)
                    .build(source_code, &line_index)
            )
            .build());
    }
}
