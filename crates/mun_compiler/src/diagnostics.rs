use mun_hir::diagnostics::{Diagnostic as HirDiagnostic, DiagnosticSink};
use mun_hir::{FileId, HirDatabase, HirDisplay, Module};
use mun_syntax::{ast, AstNode, SyntaxKind, TextRange};

use std::cell::RefCell;

use crate::annotate_snippets_builders::{AnnotationBuilder, SliceBuilder, SnippetBuilder};

use annotate_snippets::snippet::{AnnotationType, Snippet};

fn text_range_to_tuple(text_range: TextRange) -> (usize, usize) {
    (text_range.start().to_usize(), text_range.end().to_usize())
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
        let first_definition_location = match d.first_definition.kind() {
            SyntaxKind::FUNCTION_DEF => {
                ast::FunctionDef::cast(d.first_definition.to_node(&parse.tree().syntax()))
                    .map(|f| f.signature_range())
                    .unwrap_or_else(|| d.first_definition.range())
            }
            SyntaxKind::STRUCT_DEF => {
                ast::StructDef::cast(d.first_definition.to_node(&parse.tree().syntax()))
                    .map(|s| s.signature_range())
                    .unwrap_or_else(|| d.first_definition.range())
            }
            _ => d.first_definition.range(),
        };
        let definition_location = match d.definition.kind() {
            SyntaxKind::FUNCTION_DEF => {
                ast::FunctionDef::cast(d.definition.to_node(&parse.tree().syntax()))
                    .map(|f| f.signature_range())
                    .unwrap_or_else(|| d.definition.range())
            }
            SyntaxKind::STRUCT_DEF => {
                ast::StructDef::cast(d.definition.to_node(&parse.tree().syntax()))
                    .map(|s| s.signature_range())
                    .unwrap_or_else(|| d.definition.range())
            }
            _ => d.definition.range(),
        };
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

    #[test]
    fn test_text_range_to_tuple() {
        let text_range = TextRange::from_to(3.into(), 5.into());
        assert_eq!(text_range_to_tuple(text_range), (3, 5));
    }
}
