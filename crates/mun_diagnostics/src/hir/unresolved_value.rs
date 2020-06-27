use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use mun_syntax::{AstNode, TextRange};

pub struct UnresolvedValue<'db, 'diag, DB: hir::HirDatabase> {
    _db: &'db DB,
    diag: &'diag hir::diagnostics::UnresolvedValue,
    value_name: String,
}

impl<'db, 'diag, DB: hir::HirDatabase> Diagnostic for UnresolvedValue<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.diag.highlight_range()
    }

    fn label(&self) -> String {
        format!("cannot find value `{}` in this scope", self.value_name)
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: self.diag.highlight_range(),
            message: "not found in this scope".to_owned(),
        })
    }
}

impl<'db, 'diag, DB: hir::HirDatabase> UnresolvedValue<'db, 'diag, DB> {
    /// Constructs a new instance of `UnresolvedValue`
    pub fn new(db: &'db DB, diag: &'diag hir::diagnostics::UnresolvedValue) -> Self {
        let parse = db.parse(diag.file);

        // Get the text of the value as a string
        let value_name = diag.expr.to_node(&parse.tree().syntax()).text().to_string();

        UnresolvedValue {
            _db: db,
            diag,
            value_name,
        }
    }
}
