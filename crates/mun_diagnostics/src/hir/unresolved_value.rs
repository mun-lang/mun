use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use mun_syntax::{AstNode, TextRange};

/// An error that is emitted when trying to use a value that doesnt exist within the scope.
///
/// ```mun
/// # fn main() {
/// let a = b; // Cannot find `b` in this scope.
/// #}
/// ```
pub struct UnresolvedValue<'db, 'diag, DB: mun_hir::HirDatabase> {
    _db: &'db DB,
    diag: &'diag mun_hir::diagnostics::UnresolvedValue,
    value_name: String,
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> Diagnostic for UnresolvedValue<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.diag.highlight_range()
    }

    fn title(&self) -> String {
        format!("cannot find value `{}` in this scope", self.value_name)
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: self.diag.highlight_range(),
            message: "not found in this scope".to_owned(),
        })
    }
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> UnresolvedValue<'db, 'diag, DB> {
    /// Constructs a new instance of `UnresolvedValue`
    pub fn new(db: &'db DB, diag: &'diag mun_hir::diagnostics::UnresolvedValue) -> Self {
        let parse = db.parse(diag.file);

        // Get the text of the value as a string
        let value_name = diag.expr.to_node(parse.tree().syntax()).text().to_string();

        UnresolvedValue {
            _db: db,
            diag,
            value_name,
        }
    }
}
