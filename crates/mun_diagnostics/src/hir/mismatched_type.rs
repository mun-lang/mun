use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use hir::HirDisplay;
use mun_syntax::TextRange;

pub struct MismatchedType<'db, 'diag, DB: hir::HirDatabase> {
    db: &'db DB,
    diag: &'diag hir::diagnostics::MismatchedType,
}

impl<'db, 'diag, DB: hir::HirDatabase> Diagnostic for MismatchedType<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.diag.highlight_range()
    }

    fn label(&self) -> String {
        format!(
            "expected `{}`, found `{}`",
            self.diag.expected.display(self.db),
            self.diag.found.display(self.db)
        )
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        None
    }
}

impl<'db, 'diag, DB: hir::HirDatabase> MismatchedType<'db, 'diag, DB> {
    /// Constructs a new instance of `MismatchedType`
    pub fn new(db: &'db DB, diag: &'diag hir::diagnostics::MismatchedType) -> Self {
        MismatchedType { db, diag }
    }
}
