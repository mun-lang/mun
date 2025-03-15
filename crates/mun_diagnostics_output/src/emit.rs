use std::{collections::HashMap, io::Cursor, sync::Arc};

use annotate_snippets::{Annotation, AnnotationType, Renderer, Slice, Snippet, SourceAnnotation};
use mun_diagnostics::DiagnosticForWith;
use mun_hir::{DiagnosticSink, HirDatabase};
use mun_hir_input::{FileId, LineIndex};
use mun_paths::RelativePathBuf;
use mun_syntax::SyntaxError;

use crate::DisplayColor;

/// Emits all diagnostic messages currently in the database; returns true if
/// errors were emitted.
pub fn emit_diagnostics(
    db: &impl mun_hir::HirDatabase,
    writer: &mut dyn std::io::Write,
    display_color: DisplayColor,
) -> Result<bool, anyhow::Error> {
    let emit_colors = display_color.should_enable();
    let mut has_error = false;

    for package in mun_hir::Package::all(db) {
        for module in package.modules(db) {
            if let Some(file_id) = module.file_id(db) {
                let parse = db.parse(file_id);
                let source_code = db.file_text(file_id);
                let relative_file_path = db.file_relative_path(file_id);
                let line_index = db.line_index(file_id);

                // Emit all syntax diagnostics
                for syntax_error in parse.errors().iter() {
                    emit_syntax_error(
                        syntax_error,
                        relative_file_path.as_str(),
                        &source_code,
                        &line_index,
                        emit_colors,
                        writer,
                    )?;
                    has_error = true;
                }

                // Emit all HIR diagnostics
                let mut error = None;
                module.diagnostics(
                    db,
                    &mut DiagnosticSink::new(|d| {
                        has_error = true;
                        if let Err(e) = emit_hir_diagnostic(d, db, file_id, emit_colors, writer) {
                            error = Some(e);
                        };
                    }),
                );

                // If an error occurred when emitting HIR diagnostics, return early with the
                // error.
                if let Some(e) = error {
                    return Err(e.into());
                }
            }
        }
    }

    Ok(has_error)
}

/// Returns all diagnostics as a human readable string
pub fn emit_diagnostics_to_string(
    db: &impl mun_hir::HirDatabase,
    display_color: DisplayColor,
) -> anyhow::Result<Option<String>> {
    let mut compiler_errors: Vec<u8> = Vec::new();
    if emit_diagnostics(db, &mut Cursor::new(&mut compiler_errors), display_color)? {
        Ok(Some(String::from_utf8(compiler_errors).map_err(|e| {
            anyhow::anyhow!(
                "could not convert compiler diagnostics to valid UTF8: {}",
                e
            )
        })?))
    } else {
        Ok(None)
    }
}

/// Emits all diagnostics that are a result of HIR validation.
pub fn emit_hir_diagnostic(
    diagnostic: &dyn mun_hir::Diagnostic,
    db: &impl HirDatabase,
    file_id: FileId,
    display_colors: bool,
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    diagnostic.with_diagnostic(db, |diagnostic| {
        emit_diagnostic(diagnostic, db, file_id, display_colors, writer)
    })
}

/// Writes the specified syntax error to the output stream.
pub fn emit_syntax_error(
    syntax_error: &SyntaxError,
    relative_file_path: &str,
    source_code: &str,
    line_index: &LineIndex,
    display_colors: bool,
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    let syntax_error_text = syntax_error.to_string();
    let location = syntax_error.location();
    let line = line_index.line_col(location.offset()).line;
    let line_offset = line_index.line_offset(line);

    let snippet = Snippet {
        title: Some(Annotation {
            id: None,
            label: Some("syntax error"),
            annotation_type: AnnotationType::Error,
        }),
        footer: vec![],
        slices: vec![Slice {
            source: &source_code[line_offset..],
            line_start: line as usize + 1,
            origin: Some(relative_file_path),
            annotations: vec![SourceAnnotation {
                range: (
                    usize::from(location.offset()) - line_offset,
                    usize::from(location.end_offset()) - line_offset + 1,
                ),
                label: &syntax_error_text,
                annotation_type: AnnotationType::Error,
            }],
            fold: true,
        }],
    };

    let renderer = if display_colors {
        Renderer::styled()
    } else {
        Renderer::plain()
    };
    let display = renderer.render(snippet);
    write!(writer, "{display}")
}

/// Emits a diagnostic by writting a snippet to the specified `writer`.
fn emit_diagnostic(
    diagnostic: &dyn mun_diagnostics::Diagnostic,
    db: &impl HirDatabase,
    file_id: FileId,
    display_colors: bool,
    writer: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    /// Will hold all snippets and their relevant information
    struct AnnotationFile {
        relative_file_path: RelativePathBuf,
        source_code: Arc<str>,
        line_index: Arc<LineIndex>,
        annotations: Vec<mun_diagnostics::SourceAnnotation>,
    }

    // Get the basic info from the diagnostic
    let title = diagnostic.title();
    let range = diagnostic.range();

    let annotations = {
        let mut annotations = Vec::new();
        let mut file_to_index = HashMap::new();

        // Add primary annotations
        annotations.push(AnnotationFile {
            relative_file_path: db.file_relative_path(file_id).to_relative_path_buf(),
            source_code: db.file_text(file_id),
            line_index: db.line_index(file_id),
            annotations: vec![match diagnostic.primary_annotation() {
                None => mun_diagnostics::SourceAnnotation {
                    range,
                    message: title.clone(),
                },
                Some(annotation) => annotation,
            }],
        });
        file_to_index.insert(file_id, 0);

        // Add the secondary annotations
        for annotation in diagnostic.secondary_annotations() {
            let file_id = annotation.range.file_id;

            // Find an entry for this `file_id`
            let file_idx = match file_to_index.get(&file_id) {
                None => {
                    // Doesn't exist yet, add it
                    annotations.push(AnnotationFile {
                        relative_file_path: db.file_relative_path(file_id),
                        source_code: db.file_text(file_id),
                        line_index: db.line_index(file_id),
                        annotations: Vec::new(),
                    });
                    let idx = annotations.len() - 1;
                    file_to_index.insert(file_id, idx);
                    idx
                }
                Some(idx) => *idx,
            };

            // Add this annotation to the list of snippets for the file
            annotations[file_idx].annotations.push(annotation.into());
        }

        annotations
    };

    let footer = diagnostic.footer();

    // Construct an annotation snippet to be able to emit it.
    let snippet = Snippet {
        title: Some(Annotation {
            id: None,
            label: Some(&title),
            annotation_type: AnnotationType::Error,
        }),
        slices: annotations
            .iter()
            .filter_map(|file| {
                let first_offset = {
                    let mut iter = file.annotations.iter();
                    match iter.next() {
                        Some(first) => {
                            let first = first.range.start();
                            iter.fold(first, |init, value| init.min(value.range.start()))
                        }
                        None => return None,
                    }
                };
                let first_offset_line = file.line_index.line_col(first_offset);
                let line_offset = file.line_index.line_offset(first_offset_line.line);
                Some(Slice {
                    source: &file.source_code[line_offset..],
                    line_start: first_offset_line.line as usize + 1,
                    origin: Some(file.relative_file_path.as_ref()),
                    annotations: file
                        .annotations
                        .iter()
                        .map(|annotation| SourceAnnotation {
                            range: (
                                usize::from(annotation.range.start()) - line_offset,
                                usize::from(annotation.range.end()) - line_offset,
                            ),
                            label: annotation.message.as_str(),
                            annotation_type: AnnotationType::Error,
                        })
                        .collect(),
                    fold: true,
                })
            })
            .collect(),
        footer: footer
            .iter()
            .map(|footer| Annotation {
                id: None,
                label: Some(footer.as_str()),
                annotation_type: AnnotationType::Note,
            })
            .collect(),
    };

    // Write the snippet to the output stream
    let renderer = if display_colors {
        Renderer::styled()
    } else {
        Renderer::plain()
    };
    let display = renderer.render(snippet);
    write!(writer, "{display}")
}
