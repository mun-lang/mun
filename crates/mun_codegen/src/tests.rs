use crate::{IrDatabase, OptimizationLevel};
use mun_hir::diagnostics::DiagnosticSink;
use mun_hir::{salsa, FileId, Module, PackageInput, RelativePathBuf, SourceDatabase};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;
use test_utils::{dir_tests, project_dir};

#[salsa::database(
    mun_hir::SourceDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    crate::IrDatabaseStorage
)]
#[derive(Default, Debug)]
struct MockDatabase {
    runtime: salsa::Runtime<MockDatabase>,
}

impl salsa::Database for MockDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<MockDatabase> {
        &self.runtime
    }
}

fn test_data_dir() -> PathBuf {
    project_dir().join("crates/mun_codegen/tests/data/")
}

#[test]
fn ir_tests() {
    dir_tests(&test_data_dir(), &["ir"], |text, path| {
        let mut db = MockDatabase::default();
        let file_id = FileId(0);
        dbg!(path);
        db.set_file_relative_path(file_id, RelativePathBuf::from("main.mun"));
        db.set_file_text(file_id, Arc::new(text.to_string()));
        let mut package_input = PackageInput::default();
        package_input.add_module(file_id);
        db.set_package_input(Arc::new(package_input));
        db.set_optimization_lvl(OptimizationLevel::Default);

        let context = crate::Context::create();
        db.set_context(Arc::new(context));

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
