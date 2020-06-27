use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};
use hir::HirDisplay;
use mun_syntax::{ast, AstNode, TextRange};

pub struct MissingFields<'db, 'diag, DB: hir::HirDatabase> {
    db: &'db DB,
    diag: &'diag hir::diagnostics::MissingFields,
    location: TextRange,
    missing_fields: String,
}

impl<'db, 'diag, DB: hir::HirDatabase> Diagnostic for MissingFields<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.location
    }

    fn label(&self) -> String {
        format!(
            "missing fields {} in initializer of `{}`",
            self.missing_fields,
            self.diag.struct_ty.display(self.db)
        )
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: self.location,
            message: self.missing_fields.clone(),
        })
    }
}

impl<'db, 'diag, DB: hir::HirDatabase> MissingFields<'db, 'diag, DB> {
    /// Constructs a new instance of `MissingFields`
    pub fn new(db: &'db DB, diag: &'diag hir::diagnostics::MissingFields) -> Self {
        let parse = db.parse(diag.file);
        let missing_fields = diag
            .field_names
            .iter()
            .map(|n| format!("`{}`", n))
            .collect::<Vec<String>>()
            .join(", ");
        let location = ast::RecordLit::cast(diag.fields.to_node(&parse.syntax_node()))
            .and_then(|f| f.type_ref())
            .map(|t| t.syntax().text_range())
            .unwrap_or_else(|| diag.highlight_range());

        MissingFields {
            db,
            diag,
            location,
            missing_fields,
        }
    }
}
