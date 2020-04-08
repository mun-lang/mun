use mun_hir::diagnostics::Diagnostic as HirDiagnostic;
use mun_hir::{HirDatabase, HirDisplay};
use mun_syntax::{
    ast, AstNode, Parse, SourceFile, SyntaxError, SyntaxKind, SyntaxNodePtr, TextRange,
};

use std::sync::Arc;

use mun_hir::line_index::LineIndex;

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

pub(crate) fn syntax_error(
    syntax_error: &SyntaxError,
    _: &impl HirDatabase,
    _: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    let mut snippet = SnippetBuilder::new()
        .title(
            AnnotationBuilder::new(AnnotationType::Error)
                .label("syntax error".to_string())
                .build(),
        )
        .slice(
            SliceBuilder::new(true)
                .origin(relative_file_path.to_string())
                .source_annotation(
                    (
                        syntax_error.location().offset().to_usize(),
                        syntax_error.location().end_offset().to_usize(),
                    ),
                    syntax_error.to_string(),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .build();
    // Add one to right range to make highlighting range here visible on output
    snippet.slices[0].annotations[0].range.1 += 1;

    snippet
}

pub(crate) fn error(
    diagnostic: &dyn HirDiagnostic,
    _: &impl HirDatabase,
    _: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    SnippetBuilder::new()
        .title(
            AnnotationBuilder::new(AnnotationType::Error)
                .label("unexpected error".to_string())
                .build(),
        )
        .slice(
            SliceBuilder::new(true)
                .origin(relative_file_path.to_string())
                .source_annotation(
                    text_range_to_tuple(diagnostic.highlight_range()),
                    diagnostic.message(),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .build()
}

pub(crate) fn unresolved_value_error(
    diagnostic: &mun_hir::diagnostics::UnresolvedValue,
    _: &impl HirDatabase,
    parse: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    let unresolved_value = diagnostic
        .expr
        .to_node(&parse.tree().syntax())
        .text()
        .to_string();

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
                .origin(relative_file_path.to_string())
                .source_annotation(
                    text_range_to_tuple(diagnostic.highlight_range()),
                    "not found in this scope".to_string(),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .build()
}

pub(crate) fn unresolved_type_error(
    diagnostic: &mun_hir::diagnostics::UnresolvedType,
    _: &impl HirDatabase,
    parse: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    let unresolved_type = diagnostic
        .type_ref
        .to_node(&parse.syntax_node())
        .syntax()
        .text()
        .to_string();

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
                .origin(relative_file_path.to_string())
                .source_annotation(
                    text_range_to_tuple(diagnostic.highlight_range()),
                    "not found in this scope".to_string(),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .build()
}

pub(crate) fn expected_function_error(
    diagnostic: &mun_hir::diagnostics::ExpectedFunction,
    hir_database: &impl HirDatabase,
    _: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    SnippetBuilder::new()
        .title(
            AnnotationBuilder::new(AnnotationType::Error)
                .label(diagnostic.message())
                .build(),
        )
        .slice(
            SliceBuilder::new(true)
                .origin(relative_file_path.to_string())
                .source_annotation(
                    text_range_to_tuple(diagnostic.highlight_range()),
                    format!(
                        "expected function, found `{}`",
                        diagnostic.found.display(hir_database)
                    ),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .build()
}

pub(crate) fn mismatched_type_error(
    diagnostic: &mun_hir::diagnostics::MismatchedType,
    hir_database: &impl HirDatabase,
    _: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    SnippetBuilder::new()
        .title(
            AnnotationBuilder::new(AnnotationType::Error)
                .label(diagnostic.message())
                .build(),
        )
        .slice(
            SliceBuilder::new(true)
                .origin(relative_file_path.to_string())
                .source_annotation(
                    text_range_to_tuple(diagnostic.highlight_range()),
                    format!(
                        "expected `{}`, found `{}`",
                        diagnostic.expected.display(hir_database),
                        diagnostic.found.display(hir_database)
                    ),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .build()
}

pub(crate) fn duplicate_definition_error(
    diagnostic: &mun_hir::diagnostics::DuplicateDefinition,
    _: &impl HirDatabase,
    parse: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    let first_definition_location = syntax_node_ptr_location(diagnostic.first_definition, &parse);
    let definition_location = syntax_node_ptr_location(diagnostic.definition, &parse);

    let duplication_object_type =
        if matches!(diagnostic.first_definition.kind(), SyntaxKind::STRUCT_DEF)
            && matches!(diagnostic.definition.kind(), SyntaxKind::STRUCT_DEF)
        {
            "type"
        } else {
            "value"
        };

    SnippetBuilder::new()
        .title(
            AnnotationBuilder::new(AnnotationType::Error)
                .label(diagnostic.message())
                .build(),
        )
        .slice(
            SliceBuilder::new(true)
                .origin(relative_file_path.to_string())
                // First definition
                .source_annotation(
                    text_range_to_tuple(first_definition_location),
                    format!(
                        "previous definition of the {} `{}` here",
                        duplication_object_type, diagnostic.name
                    ),
                    AnnotationType::Warning,
                )
                // Second definition
                .source_annotation(
                    text_range_to_tuple(definition_location),
                    format!("`{}` redefined here", diagnostic.name),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .footer(
            AnnotationBuilder::new(AnnotationType::Note)
                .label(format!(
                    "`{}` must be defined only once in the {} namespace of this module",
                    diagnostic.name, duplication_object_type
                ))
                .build(),
        )
        .build()
}

pub(crate) fn possibly_uninitialized_variable_error(
    diagnostic: &mun_hir::diagnostics::PossiblyUninitializedVariable,
    _: &impl HirDatabase,
    parse: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    let variable_name = diagnostic.pat.to_node(&parse.syntax_node()).text();

    SnippetBuilder::new()
        .title(
            AnnotationBuilder::new(AnnotationType::Error)
                .label(format!("{}: `{}`", diagnostic.message(), variable_name))
                .build(),
        )
        .slice(
            SliceBuilder::new(true)
                .origin(relative_file_path.to_string())
                .source_annotation(
                    text_range_to_tuple(diagnostic.highlight_range()),
                    format!("use of possibly-uninitialized `{}`", variable_name),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .build()
}

pub(crate) fn access_unknown_field_error(
    diagnostic: &mun_hir::diagnostics::AccessUnknownField,
    hir_database: &impl HirDatabase,
    parse: &Parse<SourceFile>,
    relative_file_path: &str,
    source_code: &str,
    source_code_len: usize,
    line_index: &Arc<LineIndex>,
) -> Snippet {
    let location = ast::FieldExpr::cast(diagnostic.expr.to_node(&parse.syntax_node()))
        .map(|f| f.field_range())
        .unwrap_or_else(|| diagnostic.highlight_range());

    SnippetBuilder::new()
        .title(
            AnnotationBuilder::new(AnnotationType::Error)
                .label(format!(
                    "no field `{}` on type `{}`",
                    diagnostic.name,
                    diagnostic.receiver_ty.display(hir_database),
                ))
                .build(),
        )
        .slice(
            SliceBuilder::new(true)
                .origin(relative_file_path.to_string())
                .source_annotation(
                    text_range_to_tuple(location),
                    "unknown field".to_string(),
                    AnnotationType::Error,
                )
                .build(&source_code, source_code_len, &line_index),
        )
        .build()
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
