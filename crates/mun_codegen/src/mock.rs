use std::sync::Arc;

use mun_hir::HirDatabase;
use mun_hir_input::{FileId, PackageSet, SourceDatabase, SourceRoot, SourceRootId};
use mun_paths::RelativePathBuf;
use mun_target::spec::Target;
use parking_lot::Mutex;

use crate::{
    db::{CodeGenDatabase, CodeGenDatabaseStorage},
    OptimizationLevel,
};

/// A mock implementation of the IR database. It can be used to set up a simple
/// test case.
#[salsa::database(
    mun_hir_input::SourceDatabaseStorage,
    mun_hir::AstDatabaseStorage,
    mun_hir::InternDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    CodeGenDatabaseStorage
)]
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

impl mun_db::Upcast<dyn mun_hir::AstDatabase> for MockDatabase {
    fn upcast(&self) -> &(dyn mun_hir::AstDatabase + 'static) {
        self
    }
}

impl mun_db::Upcast<dyn SourceDatabase> for MockDatabase {
    fn upcast(&self) -> &(dyn SourceDatabase + 'static) {
        self
    }
}

impl mun_db::Upcast<dyn mun_hir::DefDatabase> for MockDatabase {
    fn upcast(&self) -> &(dyn mun_hir::DefDatabase + 'static) {
        self
    }
}

impl mun_db::Upcast<dyn mun_hir::HirDatabase> for MockDatabase {
    fn upcast(&self) -> &(dyn mun_hir::HirDatabase + 'static) {
        self
    }
}

impl mun_db::Upcast<dyn CodeGenDatabase> for MockDatabase {
    fn upcast(&self) -> &(dyn CodeGenDatabase + 'static) {
        self
    }
}

impl Default for MockDatabase {
    fn default() -> Self {
        let mut db = MockDatabase {
            storage: salsa::Storage::default(),
            events: Mutex::default(),
        };
        db.set_optimization_level(OptimizationLevel::Default);
        db.set_target(Target::host_target().unwrap());
        db
    }
}

impl MockDatabase {
    /// Creates a database from the given text.
    pub fn with_single_file(text: &str) -> (MockDatabase, FileId) {
        let mut db = MockDatabase::default();

        let mut source_root = SourceRoot::default();
        let source_root_id = SourceRootId(0);

        let text = Arc::from(text.to_owned());
        let rel_path = RelativePathBuf::from("mod.mun");
        let file_id = FileId(0);
        db.set_file_text(file_id, text);
        db.set_file_source_root(file_id, source_root_id);
        source_root.insert_file(file_id, rel_path);

        db.set_source_root(source_root_id, Arc::new(source_root));

        let mut packages = PackageSet::default();
        packages.add_package(source_root_id);
        db.set_packages(Arc::new(packages));

        db.set_optimization_level(OptimizationLevel::None);
        (db, file_id)
    }

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
