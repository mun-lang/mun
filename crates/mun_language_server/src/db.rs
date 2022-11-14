#![allow(clippy::enum_variant_names)] // This is a HACK because we use salsa

use crate::cancelation::Canceled;
use mun_hir::{HirDatabase, Upcast};
use mun_target::spec::Target;
use salsa::{Database, Durability, Snapshot};
use std::panic;

/// The `AnalysisDatabase` provides the database for all analyses. A database is given input and
/// produces output based on these inputs through the use of queries. These queries are memoized
/// which enables us to not have to recompute everything if certain inputs change. `salsa` powers
/// this.
///
/// Internally this `AnalysisDatabase` is composed of the `Analysis` struct. You can apply a set of
/// changes which will be applied to the database. You can take snapshots of the `Analysis` struct
/// which can be used to perform analysis.
///
/// With this struct we can reuse a lot of functionality from the compiler which should provide a
/// better user experience.
#[salsa::database(
    mun_hir::SourceDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    mun_hir::AstDatabaseStorage,
    mun_hir::InternDatabaseStorage
)]
pub(crate) struct AnalysisDatabase {
    storage: salsa::Storage<Self>,
}

impl Default for AnalysisDatabase {
    fn default() -> Self {
        let mut db = AnalysisDatabase {
            storage: Default::default(),
        };
        db.set_target(Target::host_target().expect("could not determine host target spec"));
        db
    }
}

impl AnalysisDatabase {
    /// Triggers a simple write on the database which will cancell all outstanding snapshots.
    pub fn request_cancelation(&mut self) {
        self.salsa_runtime_mut().synthetic_write(Durability::LOW);
    }
}

impl salsa::Database for AnalysisDatabase {
    fn on_propagated_panic(&self) -> ! {
        Canceled::throw()
    }
    fn salsa_event(&self, event: salsa::Event) {
        match event.kind {
            salsa::EventKind::DidValidateMemoizedValue { .. }
            | salsa::EventKind::WillExecute { .. } => {
                self.check_canceled();
            }
            _ => (),
        }
    }
}

impl Upcast<dyn mun_hir::AstDatabase> for AnalysisDatabase {
    fn upcast(&self) -> &(dyn mun_hir::AstDatabase + 'static) {
        self
    }
}

impl Upcast<dyn mun_hir::SourceDatabase> for AnalysisDatabase {
    fn upcast(&self) -> &(dyn mun_hir::SourceDatabase + 'static) {
        self
    }
}

impl Upcast<dyn mun_hir::DefDatabase> for AnalysisDatabase {
    fn upcast(&self) -> &(dyn mun_hir::DefDatabase + 'static) {
        self
    }
}

impl Upcast<dyn mun_hir::HirDatabase> for AnalysisDatabase {
    fn upcast(&self) -> &(dyn mun_hir::HirDatabase + 'static) {
        self
    }
}

impl AnalysisDatabase {
    fn check_canceled(&self) {
        if self.salsa_runtime().is_current_revision_canceled() {
            Canceled::throw()
        }
    }

    pub fn catch_canceled<F, T>(&self, f: F) -> Result<T, Canceled>
    where
        Self: Sized + panic::RefUnwindSafe,
        F: FnOnce(&Self) -> T + panic::UnwindSafe,
    {
        panic::catch_unwind(|| f(self)).map_err(|err| match err.downcast::<Canceled>() {
            Ok(canceled) => *canceled,
            Err(payload) => panic::resume_unwind(payload),
        })
    }
}

impl salsa::ParallelDatabase for AnalysisDatabase {
    fn snapshot(&self) -> Snapshot<Self> {
        Snapshot::new(AnalysisDatabase {
            storage: self.storage.snapshot(),
        })
    }
}
