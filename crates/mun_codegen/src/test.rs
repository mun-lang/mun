use crate::{mock::MockDatabase, IrDatabase};
use mun_hir::diagnostics::DiagnosticSink;
use mun_hir::Module;
use std::cell::RefCell;
use std::path::PathBuf;
use test_utils::{dir_tests, project_dir};

fn test_data_dir() -> PathBuf {
    project_dir().join("crates/mun_codegen/tests/data/")
}

#[test]
fn ir_tests() {
    dir_tests(&test_data_dir(), &["ir"], |text, _path| {
        let (db, file_id) = MockDatabase::with_single_file(text);

        let messages = RefCell::new(Vec::new());
        let mut sink = DiagnosticSink::new(|diag| {
            messages.borrow_mut().push(diag.message());
        });
        if let Some(module) = Module::package_modules(&db)
            .iter()
            .find(|m| m.file_id() == file_id)
        {
            module.diagnostics(&db, &mut sink)
        }
        drop(sink);
        let messages = messages.into_inner();

        if !messages.is_empty() {
            messages.join("\n")
        } else {
            format!(
                "{}",
                db.module_ir(file_id)
                    .llvm_module
                    .print_to_string()
                    .to_string()
            )
        }
    });
}
