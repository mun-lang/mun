use mun_hir::diagnostics::{Diagnostic as HirDiagnostic, DiagnosticSink};
use mun_hir::{FileId, HirDatabase, HirDisplay, Module};
use mun_syntax::{ast, AstNode, SyntaxKind};
use std::cell::RefCell;

mod emit;

pub use emit::Emit;
use mun_errors::{Diagnostic, Level};

/// Constructs diagnostic messages for the given file.
pub fn diagnostics(db: &impl HirDatabase, file_id: FileId) -> Vec<Diagnostic> {
    let parse = db.parse(file_id);
    let mut result = Vec::new();

    result.extend(parse.errors().iter().map(|err| Diagnostic {
        level: Level::Error,
        loc: err.location(),
        message: format!("Syntax Error: {}", err),
    }));

    let result = RefCell::new(result);
    let mut sink = DiagnosticSink::new(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: d.message(),
        });
    })
    .on::<mun_hir::diagnostics::UnresolvedValue, _>(|d| {
        let text = d.expr.to_node(&parse.syntax_node()).text().to_string();
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: format!("could not find value `{}` in this scope", text),
        });
    })
    .on::<mun_hir::diagnostics::UnresolvedType, _>(|d| {
        let text = d
            .type_ref
            .to_node(&parse.syntax_node())
            .syntax()
            .text()
            .to_string();
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: format!("could not find type `{}` in this scope", text),
        });
    })
    .on::<mun_hir::diagnostics::ExpectedFunction, _>(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: format!("expected function, found `{}`", d.found.display(db)),
        });
    })
    .on::<mun_hir::diagnostics::MismatchedType, _>(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: format!(
                "expected `{}`, found `{}`",
                d.expected.display(db),
                d.found.display(db)
            ),
        });
    })
    .on::<mun_hir::diagnostics::DuplicateDefinition, _>(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: match d.definition.kind() {
                SyntaxKind::FUNCTION_DEF => {
                    ast::FunctionDef::cast(d.definition.to_node(&parse.syntax_node()))
                        .map(|f| f.signature_range())
                        .unwrap_or_else(|| d.highlight_range())
                        .into()
                }
                _ => d.highlight_range().into(),
            },
            message: d.message(),
        });
    })
    .on::<mun_hir::diagnostics::PossiblyUninitializedVariable, _>(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: d.highlight_range().into(),
            message: format!(
                "use of possibly-uninitialized variable: `{}`",
                d.pat.to_node(&parse.syntax_node()).text().to_string()
            ),
        })
    })
    .on::<mun_hir::diagnostics::AccessUnknownField, _>(|d| {
        result.borrow_mut().push(Diagnostic {
            level: Level::Error,
            loc: ast::FieldExpr::cast(d.expr.to_node(&parse.syntax_node()))
                .map(|f| f.field_range())
                .unwrap_or_else(|| d.highlight_range())
                .into(),
            message: format!(
                "no field `{}` on type `{}`",
                d.name,
                d.receiver_ty.display(db),
            ),
        })
    });

    Module::from(file_id).diagnostics(db, &mut sink);

    drop(sink);
    result.into_inner()
}
