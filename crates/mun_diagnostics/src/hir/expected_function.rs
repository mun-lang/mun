use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use hir::HirDisplay;
use mun_syntax::TextRange;

pub struct ExpectedFunction<'db, 'diag, DB: hir::HirDatabase> {
    db: &'db DB,
    diag: &'diag hir::diagnostics::ExpectedFunction,
}

impl<'db, 'diag, DB: hir::HirDatabase> Diagnostic for ExpectedFunction<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.diag.highlight_range()
    }

    fn label(&self) -> String {
        format!(
            "expected function, found `{}`",
            self.diag.found.display(self.db)
        )
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: self.diag.highlight_range(),
            message: "not a function".to_owned(),
        })
    }
}

impl<'db, 'diag, DB: hir::HirDatabase> ExpectedFunction<'db, 'diag, DB> {
    /// Constructs a new instance of `ExpectedFunction`
    pub fn new(db: &'db DB, diag: &'diag hir::diagnostics::ExpectedFunction) -> Self {
        ExpectedFunction { db, diag }
    }
}
