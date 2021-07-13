use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use mun_syntax::{AstNode, TextRange};

/// An error that is emitted when trying to leak a private type
pub struct ExportedPrivate<'db, 'diag, DB: mun_hir::HirDatabase> {
    _db: &'db DB,
    diag: &'diag mun_hir::diagnostics::ExportedPrivate,
    value_name: String,
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> Diagnostic for ExportedPrivate<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.diag.highlight_range()
    }

    fn title(&self) -> String {
        format!("can't leak `{}`", self.value_name)
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: self.diag.highlight_range(),
            message: self.diag.message(),
        })
    }
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> ExportedPrivate<'db, 'diag, DB> {
    /// Constructs a new instance of `ExportedPrivate`
    pub fn new(db: &'db DB, diag: &'diag mun_hir::diagnostics::ExportedPrivate) -> Self {
        let parse = db.parse(diag.file);

        // Get the text of the value as a string
        let value_name = diag
            .type_ref
            .to_node(&parse.syntax_node())
            .syntax()
            .text()
            .to_string();

        ExportedPrivate {
            _db: db,
            diag,
            value_name,
        }
    }
}
