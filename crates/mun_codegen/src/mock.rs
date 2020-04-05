use crate::{IrDatabase, OptimizationLevel};
use hir::{FileId, RelativePathBuf, SourceDatabase, SourceRoot, SourceRootId};
use parking_lot::Mutex;
use std::sync::Arc;

/// A mock implementation of the IR database. It can be used to set up a simple test case.
#[salsa::database(
    hir::SourceDatabaseStorage,
    hir::DefDatabaseStorage,
    hir::HirDatabaseStorage,
    crate::IrDatabaseStorage
)]
#[derive(Default, Debug)]
pub(crate) struct MockDatabase {
    runtime: salsa::Runtime<MockDatabase>,
    events: Mutex<Option<Vec<salsa::Event<MockDatabase>>>>,
}

impl salsa::Database for MockDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<MockDatabase> {
        &self.runtime
    }

    fn salsa_runtime_mut(&mut self) -> &mut salsa::Runtime<MockDatabase> {
        &mut self.runtime
    }

    fn salsa_event(&self, event: impl Fn() -> salsa::Event<MockDatabase>) {
        let mut events = self.events.lock();
        if let Some(events) = &mut *events {
            events.push(event());
        }
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
        db.set_optimization_lvl(OptimizationLevel::None);

        let context = crate::Context::create();
        db.set_context(Arc::new(context));
        (db, file_id)
    }

    pub fn log(&self, f: impl FnOnce()) -> Vec<salsa::Event<MockDatabase>> {
        *self.events.lock() = Some(Vec::new());
        f();
        self.events.lock().take().unwrap()
    }

    pub fn log_executed(&self, f: impl FnOnce()) -> Vec<String> {
        let events = self.log(f);
        events
            .into_iter()
            .filter_map(|e| match e.kind {
                // This pretty horrible, but `Debug` is the only way to inspect
                // QueryDescriptor at the moment.
                salsa::EventKind::WillExecute { database_key } => {
                    Some(format!("{:?}", database_key))
                }
                _ => None,
            })
            .collect()
    }
}
