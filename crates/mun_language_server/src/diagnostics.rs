use crate::db::AnalysisDatabase;
use mun_diagnostics::DiagnosticForWith;
use mun_hir::{AstDatabase, InFile, ModuleId, PackageId, SourceDatabase};
use mun_syntax::{Location, TextRange};
use std::cell::RefCell;

#[derive(Debug)]
pub struct SourceAnnotation {
    pub message: String,
    pub range: InFile<TextRange>,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub message: String,
    pub range: TextRange,
    pub additional_annotations: Vec<SourceAnnotation>,
    // pub fix: Option<SourceChange>,
    // pub severity: Severity,
}

/// Converts a location to a a range for use in diagnostics
fn location_to_range(location: Location) -> TextRange {
    match location {
        Location::Offset(offset) => TextRange::at(offset, 1.into()),
        Location::Range(range) => range,
    }
}

/// Computes all the diagnostics for the specified file.
pub(crate) fn diagnostics(db: &AnalysisDatabase, file_id: mun_hir::FileId) -> Vec<Diagnostic> {
    let mut result = Vec::new();

    // Add all syntax errors
    let parse = db.parse(file_id);
    result.extend(parse.errors().iter().map(|err| Diagnostic {
        message: format!("parse error: {err}"),
        range: location_to_range(err.location()),
        additional_annotations: vec![],
    }));

    // Add all HIR diagnostics
    let result = RefCell::new(result);
    let mut sink = mun_hir::diagnostics::DiagnosticSink::new(|d| {
        result.borrow_mut().push(d.with_diagnostic(db, |d| {
            Diagnostic {
                message: format!("{}\n{}", d.title(), d.footer().join("\n"))
                    .trim()
                    .to_owned(),
                range: d.range(),
                additional_annotations: d
                    .secondary_annotations()
                    .into_iter()
                    .map(|annotation| SourceAnnotation {
                        message: annotation.message,
                        range: annotation.range,
                    })
                    .collect(),
            }
        }));
    });

    let package_id = PackageId(0);
    let module_tree = db.module_tree(package_id);
    if let Some(local_id) = module_tree.module_for_file(file_id) {
        let module_id = ModuleId {
            package: package_id,
            local_id,
        };
        mun_hir::Module::from(module_id).diagnostics(db, &mut sink);
    }
    drop(sink);

    // Returns the result
    result.into_inner()
}
