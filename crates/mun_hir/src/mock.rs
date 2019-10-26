use crate::db::SourceDatabase;
use crate::{FileId, PackageInput, RelativePathBuf};
use std::sync::Arc;

/// A mock implementation of the IR database. It can be used to set up a simple test case.
#[salsa::database(
    crate::SourceDatabaseStorage,
    crate::DefDatabaseStorage,
    crate::HirDatabaseStorage
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
        let file_id = FileId(0);
        db.set_file_relative_path(file_id, RelativePathBuf::from("main.mun"));
        db.set_file_text(file_id, Arc::new(text.to_string()));
        let mut package_input = PackageInput::default();
        package_input.add_module(file_id);
        db.set_package_input(Arc::new(package_input));
        (db, file_id)
    }
}
