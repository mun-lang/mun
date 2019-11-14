use crate::{IrDatabase, OptimizationLevel};
use mun_hir::{FileId, RelativePathBuf};
use mun_hir::{SourceDatabase, SourceRoot, SourceRootId};
use std::sync::Arc;

/// A mock implementation of the IR database. It can be used to set up a simple test case.
#[salsa::database(
    mun_hir::SourceDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    crate::IrDatabaseStorage
)]
#[derive(Default, Debug)]
pub(crate) struct MockDatabase {
    runtime: salsa::Runtime<MockDatabase>,
}

impl salsa::Database for MockDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<MockDatabase> {
        &self.runtime
    }
}

impl MockDatabase {
    /// Creates a database from the given text.
    pub fn with_single_file(text: &str) -> (MockDatabase, FileId) {
        let mut db: MockDatabase = Default::default();

        let mut source_root = SourceRoot::default();
        let source_root_id = SourceRootId(0);

        let text = Arc::new(text.to_owned());
        let rel_path = RelativePathBuf::from("main.mun");
        let file_id = FileId(0);
        db.set_file_relative_path(file_id, rel_path.clone());
        db.set_file_text(file_id, Arc::new(text.to_string()));
        db.set_file_source_root(file_id, source_root_id);
        source_root.insert_file(rel_path, file_id);

        db.set_source_root(source_root_id, Arc::new(source_root));
        db.set_optimization_lvl(OptimizationLevel::Default);

        let context = crate::Context::create();
        db.set_context(Arc::new(context));
        (db, file_id)
    }
}
