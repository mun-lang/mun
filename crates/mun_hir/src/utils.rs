use std::sync::Arc;

/// Helper for mutating `Arc<[T]>` (i.e. `Arc::make_mut` for Arc slices).
/// The underlying values are cloned if there are other strong references.
pub(crate) fn make_mut_slice<T: Clone>(a: &mut Arc<[T]>) -> &mut [T] {
    if Arc::get_mut(a).is_none() {
        *a = a.iter().cloned().collect();
    }
    Arc::get_mut(a).unwrap()
}

#[cfg(test)]
pub mod tests {
    use mun_hir_input::WithFixture;

    use crate::{diagnostics::DiagnosticSink, mock::MockDatabase, AstDatabase, Package};

    pub fn diagnostics(content: &str) -> String {
        let (db, _file_id) = MockDatabase::with_single_file(content);

        let mut diags = Vec::new();

        for module in Package::all(&db).iter().flat_map(|pkg| pkg.modules(&db)) {
            if let Some(file_id) = module.file_id(&db) {
                let source_file = db.parse(file_id);
                for err in source_file.errors() {
                    diags.push(format!("{:?}: {err}", err.location()));
                }
            }
        }

        let mut diag_sink = DiagnosticSink::new(|diag| {
            diags.push(format!("{:?}: {}", diag.highlight_range(), diag.message()));
        });

        for module in Package::all(&db).iter().flat_map(|pkg| pkg.modules(&db)) {
            module.diagnostics(&db, &mut diag_sink);
        }

        drop(diag_sink);
        diags.join("\n")
    }
}
