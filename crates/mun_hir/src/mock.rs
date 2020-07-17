use crate::db::SourceDatabase;
use crate::db::{HirDatabase, Upcast};
use crate::input::{SourceRoot, SourceRootId};
use crate::{DefDatabase, FileId, RelativePathBuf};
use mun_target::spec::Target;
use parking_lot::Mutex;
use std::sync::Arc;

/// A mock implementation of the IR database. It can be used to set up a simple test case.
#[salsa::database(
    crate::SourceDatabaseStorage,
    crate::DefDatabaseStorage,
    crate::HirDatabaseStorage
)]
#[derive(Default)]
pub(crate) struct MockDatabase {
    storage: salsa::Storage<Self>,
    events: Mutex<Option<Vec<salsa::Event>>>,
}

impl salsa::Database for MockDatabase {
    fn salsa_event(&self, event: salsa::Event) {
        let mut events = self.events.lock();
        if let Some(events) = &mut *events {
            events.push(event);
        }
    }
}

impl Upcast<dyn DefDatabase> for MockDatabase {
    fn upcast(&self) -> &dyn DefDatabase {
        &*self
    }
}

impl Upcast<dyn SourceDatabase> for MockDatabase {
    fn upcast(&self) -> &dyn SourceDatabase {
        &*self
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
        db.set_target(Target::host_target().unwrap());
        db.set_file_relative_path(file_id, rel_path.clone());
        db.set_file_text(file_id, Arc::new(text.to_string()));
        db.set_file_source_root(file_id, source_root_id);
        source_root.insert_file(file_id);

        db.set_source_root(source_root_id, Arc::new(source_root));
        (db, file_id)
    }
}

impl MockDatabase {
    pub fn log(&self, f: impl FnOnce()) -> Vec<salsa::Event> {
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
                    Some(format!("{:?}", database_key.debug(self)))
                }
                _ => None,
            })
            .collect()
    }
}
