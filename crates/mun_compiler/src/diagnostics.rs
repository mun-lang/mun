use mun_hir::diagnostics::{Diagnostic as HirDiagnostic, DiagnosticSink};
use mun_hir::{FileId, HirDatabase, HirDisplay, Module};
use mun_syntax::{ast, AstNode, SyntaxKind};
use std::cell::RefCell;

use annotate_snippets::snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation};

use unicode_segmentation::UnicodeSegmentation;

/// Replace all tabs('\t') in string to spaces(' ')
fn replace_tabs(text: &mut String) {
    let mut tab_index = text.find('\t');
    while let Some(index) = tab_index {
        #[allow(clippy::range_plus_one)]
        text.replace_range(index..index + 1, " ");
        tab_index = text.find('\t');
    }
}

/// Returns amount of graphemes in text range
fn graphemes_amount_in_range(range: (usize, usize), text: &str) -> usize {
    let (_, text) = text.split_at(range.0);
    let (text, _) = text.split_at(range.1 - range.0);
    UnicodeSegmentation::graphemes(text, true).count()
}

/// Constructs proper range for `Snippet` from `annotate-snippets` crate with taking
/// folded text part in account, by subtraction first line offset from output range
fn construct_range(
    highliht_range: (usize, usize),
    text_part: &str,
    first_line_offset: usize,
) -> (usize, usize) {
    (
        highliht_range.0 - first_line_offset,
        highliht_range.0 - first_line_offset
            + graphemes_amount_in_range(
                (
                    highliht_range.0 - first_line_offset,
                    highliht_range.1 - first_line_offset,
                ),
                text_part,
            ),
    )
}

/// Constructs diagnostic messages for the given file.
pub fn diagnostics(db: &impl HirDatabase, file_id: FileId) -> Vec<Snippet> {
    let parse = db.parse(file_id);

    let mut result = Vec::new();

    let mut source_code = db.file_text(file_id).to_string();
    replace_tabs(&mut source_code);
    let source_code_len = source_code.len();

    let relative_file_path = db.file_relative_path(file_id).display().to_string();

    let line_index = db.line_index(file_id);

    result.extend(parse.errors().iter().map(|err| {
        let first_line = line_index.line_col(err.location().offset()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(err.location().end_offset()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        Snippet {
            title: Some(Annotation {
                label: Some("syntax error".to_string()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: text_part.to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: format!("syntax error: {}", err),
                    annotation_type: AnnotationType::Error,
                    range: {
                        let mut range = construct_range(
                            (
                                err.location().offset().to_usize(),
                                err.location().end_offset().to_usize(),
                            ),
                            text_part,
                            first_line_offset,
                        );
                        // adds here one to make highlight range visible in output
                        range.1 += 1;
                        range
                    },
                }],
            }],
        }
    }));

    let result = RefCell::new(result);
    let mut sink = DiagnosticSink::new(|d| {
        let first_line = line_index.line_col(d.highlight_range().start()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(d.highlight_range().end()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        result.borrow_mut().push(Snippet {
            title: Some(Annotation {
                label: Some(d.message()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: text_part.to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: d.message(),
                    annotation_type: AnnotationType::Error,
                    range: construct_range(
                        (
                            d.highlight_range().start().to_usize(),
                            d.highlight_range().end().to_usize(),
                        ),
                        text_part,
                        first_line_offset,
                    ),
                }],
            }],
        });
    })
    .on::<mun_hir::diagnostics::UnresolvedValue, _>(|d| {
        let first_line = line_index.line_col(d.highlight_range().start()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(d.highlight_range().end()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        let unresolved_value = d.expr.to_node(&parse.tree().syntax()).text().to_string();

        result.borrow_mut().push(Snippet {
            title: Some(Annotation {
                label: Some(d.message()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: text_part.to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: format!("could not find value `{}` in this scope", unresolved_value),
                    annotation_type: AnnotationType::Error,
                    range: construct_range(
                        (
                            d.highlight_range().start().to_usize(),
                            d.highlight_range().end().to_usize(),
                        ),
                        text_part,
                        first_line_offset,
                    ),
                }],
            }],
        });
    })
    .on::<mun_hir::diagnostics::UnresolvedType, _>(|d| {
        let first_line = line_index.line_col(d.highlight_range().start()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(d.highlight_range().end()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        let unresolved_type = d
            .type_ref
            .to_node(&parse.syntax_node())
            .syntax()
            .text()
            .to_string();

        result.borrow_mut().push(Snippet {
            title: Some(Annotation {
                label: Some(d.message()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: text_part.to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: format!("could not find type `{}` in this scope", unresolved_type),
                    annotation_type: AnnotationType::Error,
                    range: construct_range(
                        (
                            d.highlight_range().start().to_usize(),
                            d.highlight_range().end().to_usize(),
                        ),
                        text_part,
                        first_line_offset,
                    ),
                }],
            }],
        });
    })
    .on::<mun_hir::diagnostics::ExpectedFunction, _>(|d| {
        let first_line = line_index.line_col(d.highlight_range().start()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(d.highlight_range().end()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        result.borrow_mut().push(Snippet {
            title: Some(Annotation {
                label: Some(d.message()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: text_part.to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: format!("expected function, found `{}`", d.found.display(db)),
                    annotation_type: AnnotationType::Error,
                    range: construct_range(
                        (
                            d.highlight_range().start().to_usize(),
                            d.highlight_range().end().to_usize(),
                        ),
                        text_part,
                        first_line_offset,
                    ),
                }],
            }],
        });
    })
    .on::<mun_hir::diagnostics::MismatchedType, _>(|d| {
        let first_line = line_index.line_col(d.highlight_range().start()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(d.highlight_range().end()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        result.borrow_mut().push(Snippet {
            title: Some(Annotation {
                label: Some(d.message()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: text_part.to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: format!(
                        "expected `{}`, found `{}`",
                        d.expected.display(db),
                        d.found.display(db)
                    ),
                    annotation_type: AnnotationType::Error,
                    range: construct_range(
                        (
                            d.highlight_range().start().to_usize(),
                            d.highlight_range().end().to_usize(),
                        ),
                        text_part,
                        first_line_offset,
                    ),
                }],
            }],
        });
    })
    .on::<mun_hir::diagnostics::DuplicateDefinition, _>(|d| {
        let location = match d.definition.kind() {
            SyntaxKind::FUNCTION_DEF => {
                ast::FunctionDef::cast(d.definition.to_node(&parse.tree().syntax()))
                    .map(|f| f.signature_range())
                    .unwrap_or_else(|| d.highlight_range())
            }
            _ => d.highlight_range(),
        };

        let first_line = line_index.line_col(location.start()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(location.end()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        result.borrow_mut().push(Snippet {
            title: Some(Annotation {
                label: Some(d.message()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: text_part.to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: d.message(),
                    annotation_type: AnnotationType::Error,
                    range: construct_range(
                        (location.start().to_usize(), location.end().to_usize()),
                        text_part,
                        first_line_offset,
                    ),
                }],
            }],
        });
    })
    .on::<mun_hir::diagnostics::PossiblyUninitializedVariable, _>(|d| {
        let first_line = line_index.line_col(d.highlight_range().start()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(d.highlight_range().end()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        result.borrow_mut().push(Snippet {
            title: Some(Annotation {
                label: Some(d.message()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: text_part.to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: format!(
                        "use of possibly-uninitialized variable: `{}`",
                        d.pat.to_node(&parse.syntax_node()).text().to_string()
                    ),
                    annotation_type: AnnotationType::Error,
                    range: construct_range(
                        (
                            d.highlight_range().start().to_usize(),
                            d.highlight_range().end().to_usize(),
                        ),
                        text_part,
                        first_line_offset,
                    ),
                }],
            }],
        });
    })
    .on::<mun_hir::diagnostics::AccessUnknownField, _>(|d| {
        let location = ast::FieldExpr::cast(d.expr.to_node(&parse.syntax_node()))
            .map(|f| f.field_range())
            .unwrap_or_else(|| d.highlight_range());

        let first_line = line_index.line_col(location.start()).line;
        let first_line_offset = line_index.line_offset(first_line);

        let text_part = line_index
            .text_part(
                first_line,
                line_index.line_col(location.end()).line,
                &source_code,
                source_code_len,
            )
            .unwrap();

        result.borrow_mut().push(Snippet {
            title: Some(Annotation {
                label: Some(d.message()),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source: line_index
                    .text_part(
                        first_line,
                        line_index.line_col(location.end()).line,
                        &source_code,
                        source_code_len,
                    )
                    .unwrap()
                    .to_string(),
                line_start: first_line as usize + 1,
                origin: Some(relative_file_path.clone()),
                fold: true,
                annotations: vec![SourceAnnotation {
                    label: format!(
                        "no field `{}` on type `{}`",
                        d.name,
                        d.receiver_ty.display(db),
                    ),
                    annotation_type: AnnotationType::Error,
                    range: construct_range(
                        (location.start().to_usize(), location.end().to_usize()),
                        text_part,
                        first_line_offset,
                    ),
                }],
            }],
        });
    });

    Module::from(file_id).diagnostics(db, &mut sink);

    drop(sink);

    result.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_replace_tabs() {
        let mut some_text = "\tTextExmaple\t\tSomeText".to_string();
        replace_tabs(&mut some_text);
        assert_eq!(some_text, " TextExmaple  SomeText".to_string());

        let mut code_source = "fn fibonacci(foo: Args):int {
\tlet n = foo.n;
\tif n <= 1 {
\t\tn
\t} else {
\t\tfibonacci(Args { n: n - 1, }) + fibonacci(Args { n: n - 2, })
\t}
}"
        .to_string();
        replace_tabs(&mut code_source);
        assert_eq!(
            code_source,
            "fn fibonacci(foo: Args):int {
 let n = foo.n;
 if n <= 1 {
  n
 } else {
  fibonacci(Args { n: n - 1, }) + fibonacci(Args { n: n - 2, })
 }
}"
            .to_string()
        );
    }

    #[test]
    fn test_graphemes_amount_in_range() {
        let text = "ℱ٥ℜ\n†ěṦτ";
        assert_eq!(graphemes_amount_in_range((0, 8), &text), 3); // "ℱ٥ℜ"
        assert_eq!(graphemes_amount_in_range((5, 12), &text), 3); // "ℜ\n†"
        assert_eq!(graphemes_amount_in_range((8, 9), &text), 1); // "\n"
        assert_eq!(graphemes_amount_in_range((9, 19), &text), 4); // "†ěṦτ"
    }
}
