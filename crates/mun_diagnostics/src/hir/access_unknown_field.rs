use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use hir::HirDisplay;
use mun_syntax::{ast, AstNode, TextRange};

pub struct AccessUnknownField<'db, 'diag, DB: hir::HirDatabase> {
    db: &'db DB,
    diag: &'diag hir::diagnostics::AccessUnknownField,
    location: TextRange,
}

impl<'db, 'diag, DB: hir::HirDatabase> Diagnostic for AccessUnknownField<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.location
    }

    fn label(&self) -> String {
        format!(
            "no field `{}` on type `{}`",
            self.diag.name,
            self.diag.receiver_ty.display(self.db),
        )
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: self.location,
            message: "unknown field".to_string(),
        })
    }
}

impl<'db, 'diag, DB: hir::HirDatabase> AccessUnknownField<'db, 'diag, DB> {
    /// Constructs a new instance of `AccessUnknownField`
    pub fn new(db: &'db DB, diag: &'diag hir::diagnostics::AccessUnknownField) -> Self {
        let parse = db.parse(diag.file);

        let location = ast::FieldExpr::cast(diag.expr.to_node(&parse.syntax_node()))
            .map(|f| f.field_range())
            .unwrap_or_else(|| diag.highlight_range());

        AccessUnknownField { db, diag, location }
    }
}
