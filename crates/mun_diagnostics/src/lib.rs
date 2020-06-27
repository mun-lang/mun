mod hir;

use mun_syntax::TextRange;

///! This crate provides in depth human readable diagnostic information and fixes for compiler
///! errors that can be shared between the compiler and the language server.
///!
///! The processing of diagnostics into human readable is separated from the machine readable
///! diagnostics in for instance the HIR crate for performance reasons. This enables lazily querying
///! the system for more information only when required.

/// An annotation within the source code
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SourceAnnotation {
    /// The location in the source
    pub range: TextRange,

    /// The message
    pub message: String,
}

/// The base trait for all diagnostics in this crate.
pub trait Diagnostic {
    /// Returns the location of this diagnostic.
    fn range(&self) -> TextRange;

    /// Returns the primary message of the diagnostic.
    fn label(&self) -> String;

    /// Returns a source annotation that acts as the primary annotation for this Diagnostic. If this
    /// function returns `None` use the values returned from [`range`] and [`label`].
    fn primary_annotation(&self) -> Option<SourceAnnotation>;
}

pub trait DiagnosticFor {
    /// Calls the specified function `f` with an instance of a [`Diagnostic`]. This can be used
    /// to perform lazy diagnostic evaluation.
    fn with_diagnostic<R, F: FnMut(&dyn Diagnostic) -> R>(&self, f: F) -> R;
}

pub trait DiagnosticForWith<With> {
    /// Calls the specified function `f` with an instance of a [`Diagnostic`]. This can be used
    /// to perform lazy diagnostic evaluation.
    fn with_diagnostic<R, F: FnMut(&dyn Diagnostic) -> R>(&self, with: &With, f: F) -> R;
}
