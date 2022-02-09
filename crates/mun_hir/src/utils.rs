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
    use crate::{
        code_model::r#struct::validator::StructValidator,
        diagnostics::DiagnosticSink,
        expr::validator::{ExprValidator, TypeAliasValidator},
        mock::MockDatabase,
        with_fixture::WithFixture,
        FileId, ModuleDef, Package,
    };

    pub fn diagnostics(content: &str) -> String {
        let (db, _file_id) = MockDatabase::with_single_file(content);

        let mut diags = Vec::new();

        let mut diag_sink = DiagnosticSink::new(|diag| {
            diags.push(format!("{:?}: {}", diag.highlight_range(), diag.message()));
        });

        for item in Package::all(&db)
            .iter()
            .flat_map(|pkg| pkg.modules(&db))
            .flat_map(|module| module.declarations(&db))
        {
            match item {
                ModuleDef::Function(item) => {
                    ExprValidator::new(item, &db).validate_body(&mut diag_sink);
                }
                ModuleDef::TypeAlias(item) => {
                    TypeAliasValidator::new(item, &db)
                        .validate_target_type_existence(&mut diag_sink);
                }
                ModuleDef::Struct(item) => {
                    StructValidator::new(item, &db, FileId(0)).validate_privacy(&mut diag_sink);
                }
                _ => {}
            }
        }

        drop(diag_sink);
        diags.join("\n")
    }
}
