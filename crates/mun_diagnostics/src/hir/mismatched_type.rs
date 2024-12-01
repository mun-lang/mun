use mun_hir::HirDisplay;
use mun_syntax::TextRange;

use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};

/// An error that is emitted when a different type was found than expected.
///
/// ```mun
/// fn add(a: i32, b: i32) -> i32{
///     a+b
/// }
///
/// # fn main() {
///     add(true, false); // type mismatch, expected i32 found bool.
/// # }
/// ```
pub struct MismatchedType<'db, 'diag, DB: mun_hir::HirDatabase> {
    db: &'db DB,
    diag: &'diag mun_hir::diagnostics::MismatchedType,
}

impl<DB: mun_hir::HirDatabase> Diagnostic for MismatchedType<'_, '_, DB> {
    fn range(&self) -> TextRange {
        self.diag.highlight_range()
    }

    fn title(&self) -> String {
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

impl<'db, 'diag, DB: mun_hir::HirDatabase> MismatchedType<'db, 'diag, DB> {
    /// Constructs a new instance of `MismatchedType`
    pub fn new(db: &'db DB, diag: &'diag mun_hir::diagnostics::MismatchedType) -> Self {
        MismatchedType { db, diag }
    }
}
