use crate::db::AnalysisDatabase;
use hir::SourceDatabase;
use mun_diagnostics::DiagnosticForWith;
use mun_syntax::{Location, TextRange};
use std::cell::RefCell;

#[derive(Debug)]
pub struct Diagnostic {
    pub message: String,
    pub range: TextRange,
    // pub fix: Option<SourceChange>,
    // pub severity: Severity,
}

/// Converts a location to a a range for use in diagnostics
fn location_to_range(location: Location) -> TextRange {
    match location {
        Location::Offset(offset) => TextRange::offset_len(offset, 1.into()),
        Location::Range(range) => range,
    }
}

/// Computes all the diagnostics for the specified file.
pub(crate) fn diagnostics(db: &AnalysisDatabase, file_id: hir::FileId) -> Vec<Diagnostic> {
    let mut result = Vec::new();

    // Add all syntax errors
    let parse = db.parse(file_id);
    result.extend(parse.errors().iter().map(|err| Diagnostic {
        message: format!("parse error: {}", err.to_string()),
        range: location_to_range(err.location()),
    }));

    // Add all HIR diagnostics
    let result = RefCell::new(result);
    let mut sink = hir::diagnostics::DiagnosticSink::new(|d| {
        result
            .borrow_mut()
            .push(d.with_diagnostic(db, |d| Diagnostic {
                message: d.label(),
                range: d.range(),
            }));
    });
    hir::Module::from(file_id).diagnostics(db, &mut sink);
    drop(sink);

    // Returns the result
    result.into_inner()
}
