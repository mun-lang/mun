mod access_unknown_field;
mod expected_function;
mod mismatched_type;
mod missing_fields;
mod possibly_unitialized_variable;
mod unresolved_type;
mod unresolved_value;

use crate::{Diagnostic, DiagnosticForWith, SourceAnnotation};
use hir::Diagnostic as HirDiagnostic;
use mun_syntax::TextRange;

// Provides conversion of a hir::Diagnostic to a crate::Diagnostic. This requires a database for
// most operations.
impl<DB: hir::HirDatabase> DiagnosticForWith<DB> for dyn hir::Diagnostic {
    fn with_diagnostic<R, F: FnMut(&dyn Diagnostic) -> R>(&self, with: &DB, mut f: F) -> R {
        if let Some(v) = self.downcast_ref::<hir::diagnostics::UnresolvedValue>() {
            f(&unresolved_value::UnresolvedValue::new(with, v))
        } else if let Some(v) = self.downcast_ref::<hir::diagnostics::UnresolvedType>() {
            f(&unresolved_type::UnresolvedType::new(with, v))
        } else if let Some(v) = self.downcast_ref::<hir::diagnostics::ExpectedFunction>() {
            f(&expected_function::ExpectedFunction::new(with, v))
        } else if let Some(v) = self.downcast_ref::<hir::diagnostics::MismatchedType>() {
            f(&mismatched_type::MismatchedType::new(with, v))
        } else if let Some(v) =
            self.downcast_ref::<hir::diagnostics::PossiblyUninitializedVariable>()
        {
            f(&possibly_unitialized_variable::PossiblyUninitializedVariable::new(with, v))
        } else if let Some(v) = self.downcast_ref::<hir::diagnostics::AccessUnknownField>() {
            f(&access_unknown_field::AccessUnknownField::new(with, v))
        } else if let Some(v) = self.downcast_ref::<hir::diagnostics::MissingFields>() {
            f(&missing_fields::MissingFields::new(with, v))
        } else {
            f(&GenericHirDiagnostic { diagnostic: self })
        }
    }
}

/// Diagnostic handler for generic hir diagnostics
struct GenericHirDiagnostic<'diag> {
    diagnostic: &'diag dyn hir::Diagnostic,
}

impl<'diag> Diagnostic for GenericHirDiagnostic<'diag> {
    fn range(&self) -> TextRange {
        self.diagnostic.highlight_range()
    }

    fn label(&self) -> String {
        self.diagnostic.message()
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        None
    }
}
