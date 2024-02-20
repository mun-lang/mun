use mun_hir::HirDisplay;
use mun_syntax::{ast, AstNode, TextRange};

use super::HirDiagnostic;
use crate::{Diagnostic, SourceAnnotation};

/// An error that is emitted when a field is missing from a struct initializer.
///
/// ```mun
/// struct Foo {
///     a: i32,
/// }
///
/// # fn main() {
///     let a = Foo {}; // missing field `a`
/// # }
/// ```
pub struct MissingFields<'db, 'diag, DB: mun_hir::HirDatabase> {
    db: &'db DB,
    diag: &'diag mun_hir::diagnostics::MissingFields,
    location: TextRange,
    missing_fields: String,
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> Diagnostic for MissingFields<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        self.location
    }

    fn title(&self) -> String {
        format!(
            "missing fields {} in initializer of `{}`",
            self.missing_fields,
            self.diag.struct_ty.display(self.db)
        )
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: self.location,
            message: format!("missing {}", self.missing_fields.clone()),
        })
    }
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> MissingFields<'db, 'diag, DB> {
    /// Constructs a new instance of `MissingFields`
    pub fn new(db: &'db DB, diag: &'diag mun_hir::diagnostics::MissingFields) -> Self {
        let parse = db.parse(diag.file);
        let missing_fields = diag
            .field_names
            .iter()
            .map(|n| format!("`{n}`"))
            .collect::<Vec<String>>()
            .join(", ");

        let location = ast::RecordLit::cast(diag.fields.to_node(&parse.syntax_node()))
            .and_then(|f| f.type_ref())
            .map_or_else(|| diag.highlight_range(), |t| t.syntax().text_range());

        MissingFields {
            db,
            diag,
            location,
            missing_fields,
        }
    }
}
