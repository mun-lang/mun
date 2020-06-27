use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use mun_syntax::{AstNode, TextRange};

pub struct UnresolvedType<'db, 'diag, DB: hir::HirDatabase> {
    _db: &'db DB,
    diag: &'diag hir::diagnostics::UnresolvedType,
    value_name: String,
}

impl<'db, 'diag, DB: hir::HirDatabase> Diagnostic for UnresolvedType<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.diag.highlight_range()
    }

    fn label(&self) -> String {
        format!("cannot find type `{}` in this scope", self.value_name)
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: self.diag.highlight_range(),
            message: "not found in this scope".to_owned(),
        })
    }
}

impl<'db, 'diag, DB: hir::HirDatabase> UnresolvedType<'db, 'diag, DB> {
    /// Constructs a new instance of `UnresolvedType`
    pub fn new(db: &'db DB, diag: &'diag hir::diagnostics::UnresolvedType) -> Self {
        let parse = db.parse(diag.file);

        // Get the text of the value as a string
        let value_name = diag
            .type_ref
            .to_node(&parse.syntax_node())
            .syntax()
            .text()
            .to_string();

        UnresolvedType {
            _db: db,
            diag,
            value_name,
        }
    }
}
