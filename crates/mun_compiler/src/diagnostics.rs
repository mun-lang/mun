use mun_hir::diagnostics::{Diagnostic as HirDiagnostic, DiagnosticSink};
use mun_hir::{FileId, HirDatabase, HirDisplay, Module};
use mun_syntax::{ast, AstNode, Parse, SourceFile, SyntaxKind, SyntaxNodePtr, TextRange};

use std::cell::RefCell;

use crate::annotate_snippets_builders::{AnnotationBuilder, SliceBuilder, SnippetBuilder};

use annotate_snippets::snippet::{AnnotationType, Snippet};

fn text_range_to_tuple(text_range: TextRange) -> (usize, usize) {
    (text_range.start().to_usize(), text_range.end().to_usize())
}

fn syntax_node_ptr_location(
    syntax_node_ptr: SyntaxNodePtr,
    parse: &Parse<SourceFile>,
) -> TextRange {
    match syntax_node_ptr.kind() {
        SyntaxKind::FUNCTION_DEF => {
            ast::FunctionDef::cast(syntax_node_ptr.to_node(parse.tree().syntax()))
                .map(|f| f.signature_range())
                .unwrap_or_else(|| syntax_node_ptr.range())
        }
        SyntaxKind::STRUCT_DEF => {
            ast::StructDef::cast(syntax_node_ptr.to_node(parse.tree().syntax()))
                .map(|s| s.signature_range())
                .unwrap_or_else(|| syntax_node_ptr.range())
        }
        _ => syntax_node_ptr.range(),
    }
}

/// Constructs diagnostic messages for the given file.
pub fn diagnostics(db: &impl HirDatabase, file_id: FileId) -> Vec<Snippet> {
    let parse = db.parse(file_id);

    let mut result = Vec::new();

    let source_code = db.file_text(file_id).to_string().replace("\t", " ");
    let source_code_len = source_code.len();

    let relative_file_path = db.file_relative_path(file_id).display().to_string();

    let line_index = db.line_index(file_id);

    result.extend(parse.errors().iter().map(|err| {
        let mut snippet = SnippetBuilder::new()
            .title(
                AnnotationBuilder::new(AnnotationType::Error)
                    .label("syntax error".to_string())
                    .build(),
            )
            .slice(
                SliceBuilder::new(true)
                    .origin(relative_file_path.clone())
                    .source_annotation(
                        (
                            err.location().offset().to_usize(),
                            err.location().end_offset().to_usize(),
                        ),
                        err.to_string(),
                        AnnotationType::Error,
                    )
                    .build(&source_code, source_code_len, &line_index),
            )
            .build();
        // Add one to right range to make highlighting range here visible on output
        snippet.slices[0].annotations[0].range.1 += 1;

        snippet
    }));

    let result = RefCell::new(result);
    let mut sink = DiagnosticSink::new(|d| {
        result.borrow_mut().push(
            SnippetBuilder::new()
                .title(
                    AnnotationBuilder::new(AnnotationType::Error)
                        .label("unexpected error".to_string())
                        .build(),
                )
                .slice(
                    SliceBuilder::new(true)
                        .origin(relative_file_path.clone())
                        .source_annotation(
                            text_range_to_tuple(d.highlight_range()),
                            d.message(),
                            AnnotationType::Error,
                        )
                        .build(&source_code, source_code_len, &line_index),
                )
                .build(),
        );
    })
    .on::<mun_hir::diagnostics::UnresolvedValue, _>(|d| {
        let unresolved_value = d.expr.to_node(&parse.tree().syntax()).text().to_string();

        result.borrow_mut().push(
            SnippetBuilder::new()
                .title(
                    AnnotationBuilder::new(AnnotationType::Error)
                        .label(format!(
                            "cannot find value `{}` in this scope",
                            unresolved_value
                        ))
                        .build(),
                )
                .slice(
                    SliceBuilder::new(true)
                        .origin(relative_file_path.clone())
                        .source_annotation(
                            text_range_to_tuple(d.highlight_range()),
                            "not found in this scope".to_string(),
                            AnnotationType::Error,
                        )
                        .build(&source_code, source_code_len, &line_index),
                )
                .build(),
        );
    })
    .on::<mun_hir::diagnostics::UnresolvedType, _>(|d| {
        let unresolved_type = d
            .type_ref
            .to_node(&parse.syntax_node())
            .syntax()
            .text()
            .to_string();

        result.borrow_mut().push(
            SnippetBuilder::new()
                .title(
                    AnnotationBuilder::new(AnnotationType::Error)
                        .label(format!(
                            "cannot find type `{}` in this scope",
                            unresolved_type
                        ))
                        .build(),
                )
                .slice(
                    SliceBuilder::new(true)
                        .origin(relative_file_path.clone())
                        .source_annotation(
                            text_range_to_tuple(d.highlight_range()),
                            "not found in this scope".to_string(),
                            AnnotationType::Error,
                        )
                        .build(&source_code, source_code_len, &line_index),
                )
                .build(),
        );
    })
    .on::<mun_hir::diagnostics::ExpectedFunction, _>(|d| {
        result.borrow_mut().push(
            SnippetBuilder::new()
                .title(
                    AnnotationBuilder::new(AnnotationType::Error)
                        .label(d.message())
                        .build(),
                )
                .slice(
                    SliceBuilder::new(true)
                        .origin(relative_file_path.clone())
                        .source_annotation(
                            text_range_to_tuple(d.highlight_range()),
                            format!("expected function, found `{}`", d.found.display(db)),
                            AnnotationType::Error,
                        )
                        .build(&source_code, source_code_len, &line_index),
                )
                .build(),
        );
    })
    .on::<mun_hir::diagnostics::MismatchedType, _>(|d| {
        result.borrow_mut().push(
            SnippetBuilder::new()
                .title(
                    AnnotationBuilder::new(AnnotationType::Error)
                        .label(d.message())
                        .build(),
                )
                .slice(
                    SliceBuilder::new(true)
                        .origin(relative_file_path.clone())
                        .source_annotation(
                            text_range_to_tuple(d.highlight_range()),
                            format!(
                                "expected `{}`, found `{}`",
                                d.expected.display(db),
                                d.found.display(db)
                            ),
                            AnnotationType::Error,
                        )
                        .build(&source_code, source_code_len, &line_index),
                )
                .build(),
        );
    })
    .on::<mun_hir::diagnostics::DuplicateDefinition, _>(|d| {
        let first_definition_location = syntax_node_ptr_location(d.first_definition, &parse);
        let definition_location = syntax_node_ptr_location(d.definition, &parse);

        let duplication_object_type = if matches!(d.first_definition.kind(), SyntaxKind::STRUCT_DEF)
            && matches!(d.definition.kind(), SyntaxKind::STRUCT_DEF)
        {
            "type"
        } else {
            "value"
        };

        result.borrow_mut().push(
            SnippetBuilder::new()
                .title(
                    AnnotationBuilder::new(AnnotationType::Error)
                        .label(d.message())
                        .build(),
                )
                .slice(
                    SliceBuilder::new(true)
                        .origin(relative_file_path.clone())
                        // First definition
                        .source_annotation(
                            text_range_to_tuple(first_definition_location),
                            format!(
                                "previous definition of the {} `{}` here",
                                duplication_object_type, d.name
                            ),
                            AnnotationType::Warning,
                        )
                        // Second definition
                        .source_annotation(
                            text_range_to_tuple(definition_location),
                            format!("`{}` redefined here", d.name),
                            AnnotationType::Error,
                        )
                        .build(&source_code, source_code_len, &line_index),
                )
                .footer(
                    AnnotationBuilder::new(AnnotationType::Note)
                        .label(format!(
                            "`{}` must be defined only once in the {} namespace of this module",
                            d.name, duplication_object_type
                        ))
                        .build(),
                )
                .build(),
        );
    })
    .on::<mun_hir::diagnostics::PossiblyUninitializedVariable, _>(|d| {
        let variable_name = d.pat.to_node(&parse.syntax_node()).text();

        result.borrow_mut().push(
            SnippetBuilder::new()
                .title(
                    AnnotationBuilder::new(AnnotationType::Error)
                        .label(format!("{}: `{}`", d.message(), variable_name))
                        .build(),
                )
                .slice(
                    SliceBuilder::new(true)
                        .origin(relative_file_path.clone())
                        .source_annotation(
                            text_range_to_tuple(d.highlight_range()),
                            format!("use of possibly-uninitialized `{}`", variable_name),
                            AnnotationType::Error,
                        )
                        .build(&source_code, source_code_len, &line_index),
                )
                .build(),
        );
    })
    .on::<mun_hir::diagnostics::AccessUnknownField, _>(|d| {
        let location = ast::FieldExpr::cast(d.expr.to_node(&parse.syntax_node()))
            .map(|f| f.field_range())
            .unwrap_or_else(|| d.highlight_range());

        result.borrow_mut().push(
            SnippetBuilder::new()
                .title(
                    AnnotationBuilder::new(AnnotationType::Error)
                        .label(format!(
                            "no field `{}` on type `{}`",
                            d.name,
                            d.receiver_ty.display(db),
                        ))
                        .build(),
                )
                .slice(
                    SliceBuilder::new(true)
                        .origin(relative_file_path.clone())
                        .source_annotation(
                            text_range_to_tuple(location),
                            "unknown field".to_string(),
                            AnnotationType::Error,
                        )
                        .build(&source_code, source_code_len, &line_index),
                )
                .build(),
        );
    });

    Module::from(file_id).diagnostics(db, &mut sink);

    drop(sink);

    result.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Config, Driver, PathOrInline, RelativePathBuf};

    #[test]
    fn test_text_range_to_tuple() {
        let text_range = TextRange::from_to(3.into(), 5.into());
        assert_eq!(text_range_to_tuple(text_range), (3, 5));
    }

    /// Compile passed source code and return all compilation errors
    fn compilation_errors(source_code: &str) -> String {
        let config = Config {
            color: Color::Disable,
            ..Config::default()
        };

        let input = PathOrInline::Inline {
            rel_path: RelativePathBuf::from("main.mun"),
            contents: source_code.to_owned(),
        };

        let (driver, _) = Driver::with_file(config, input).unwrap();

        let mut compilation_errors = Vec::<u8>::new();

        let _ = driver.emit_diagnostics(&mut compilation_errors).unwrap();

        String::from_utf8(compilation_errors).unwrap()
    }

    #[test]
    fn test_syntax_error() {
        insta::assert_display_snapshot!(compilation_errors("\n\nfn main(\n struct Foo\n"));
    }

    #[test]
    fn test_unresolved_value_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet b = a;\n\nlet d = c;\n}"
        ));
    }

    #[test]
    fn test_unresolved_type_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet a = Foo{};\n\nlet b = Bar{};\n}"
        ));
    }

    #[test]
    fn test_expected_function_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet a = Foo();\n\nlet b = Bar();\n}"
        ));
    }

    #[test]
    fn test_mismatched_type_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet a: float = false;\n\nlet b: bool = 22;\n}"
        ));
    }

    #[test]
    fn test_duplicate_definition_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn foo(){}\n\nfn foo(){}\n\nstruct Bar;\n\nstruct Bar;\n\nfn BAZ(){}\n\nstruct BAZ;"
        ));
    }

    #[test]
    fn test_possibly_uninitialized_variable_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet a;\nif 5>6 {\na = 5\n}\nlet b = a;\n}"
        ));
    }

    #[test]
    fn test_access_unknown_field_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nstruct Foo {\ni: bool\n}\n\nfn main() {\nlet a = Foo { i: false };\nlet b = a.t;\n}"
        ));
    }
}
