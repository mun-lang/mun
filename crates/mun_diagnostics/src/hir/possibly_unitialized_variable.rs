use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use mun_syntax::TextRange;

pub struct PossiblyUninitializedVariable<'db, 'diag, DB: hir::HirDatabase> {
    _db: &'db DB,
    diag: &'diag hir::diagnostics::PossiblyUninitializedVariable,
    value_name: String,
}

impl<'db, 'diag, DB: hir::HirDatabase> Diagnostic
    for PossiblyUninitializedVariable<'db, 'diag, DB>
{
    fn range(&self) -> TextRange {
        self.diag.highlight_range()
    }

    fn label(&self) -> String {
        format!("use of possibly-uninitialized `{}`", self.value_name)
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        None
    }
}

impl<'db, 'diag, DB: hir::HirDatabase> PossiblyUninitializedVariable<'db, 'diag, DB> {
    /// Constructs a new instance of `PossiblyUninitializedVariable`
    pub fn new(db: &'db DB, diag: &'diag hir::diagnostics::PossiblyUninitializedVariable) -> Self {
        let parse = db.parse(diag.file);

        // Get the text of the value as a string
        let value_name = diag.pat.to_node(&parse.syntax_node()).text().to_string();

        PossiblyUninitializedVariable {
            _db: db,
            diag,
            value_name,
        }
    }
}
