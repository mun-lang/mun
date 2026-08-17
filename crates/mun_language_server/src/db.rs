#![allow(clippy::enum_variant_names)] // This is a HACK because we use salsa

use std::panic;

use mun_hir::HirDatabase;
use mun_target::spec::Target;
use ra_salsa::{Database, Durability, Snapshot};

use crate::cancelation::Canceled;

/// The `AnalysisDatabase` provides the database for all analyses. A database is
/// given input and produces output based on these inputs through the use of
/// queries. These queries are memoized which enables us to not have to
/// recompute everything if certain inputs change. `salsa` powers this.
///
/// Internally this `AnalysisDatabase` is composed of the `Analysis` struct. You
/// can apply a set of changes which will be applied to the database. You can
/// take snapshots of the `Analysis` struct which can be used to perform
/// analysis.
///
/// With this struct we can reuse a lot of functionality from the compiler which
/// should provide a better user experience.
#[ra_salsa::database(
    mun_hir_input::SourceDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    mun_hir::AstDatabaseStorage,
    mun_hir::InternDatabaseStorage
)]
pub(crate) struct AnalysisDatabase {
    storage: ra_salsa::Storage<Self>,
}

impl Default for AnalysisDatabase {
    fn default() -> Self {
        let mut db = AnalysisDatabase {
            storage: ra_salsa::Storage::default(),
        };
        db.set_target(Target::host_target().expect("could not determine host target spec"));
        db
    }
}

impl AnalysisDatabase {
    /// Triggers a simple write on the database which will cancell all
    /// outstanding snapshots.
    pub fn request_cancelation(&mut self) {
        self.synthetic_write(Durability::LOW);
    }
}

impl ra_salsa::Database for AnalysisDatabase {}

impl AnalysisDatabase {
    pub fn catch_canceled<F, T>(&self, f: F) -> Result<T, Canceled>
    where
        Self: Sized + panic::RefUnwindSafe,
        F: FnOnce(&Self) -> T + panic::UnwindSafe,
    {
        panic::catch_unwind(|| f(self)).map_err(|err| match err.downcast::<Canceled>() {
            Ok(canceled) => *canceled,
            Err(payload) => match payload.downcast::<ra_salsa::Cancelled>() {
                Ok(_) => Canceled::new(),
                Err(payload) => panic::resume_unwind(payload),
            },
        })
    }
}

impl ra_salsa::ParallelDatabase for AnalysisDatabase {
    fn snapshot(&self) -> Snapshot<Self> {
        Snapshot::new(AnalysisDatabase {
            storage: self.storage.snapshot(),
        })
    }
}
