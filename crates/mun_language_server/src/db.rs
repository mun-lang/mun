#![allow(clippy::enum_variant_names)] // This is a HACK because we use salsa

use crate::cancelation::Canceled;
use hir::{DefDatabaseStorage, HirDatabase, HirDatabaseStorage, SourceDatabaseStorage};
use mun_target::spec::Target;
use salsa::{Database, Runtime, Snapshot};
use std::panic;

/// The `AnalysisDatabase` provides the database for all analysis. A database is given input and
/// produces output based on these inputs through the use of queries. These queries are memoized
/// which enables us to not have to recompute everything if certain inputs change. `salsa` powers
/// this.
///
/// Internally this `AnalysisDatabase` is composed in the `Analysis` struct. You can apply a set of
/// changes which will be applied to the database. You can take snapshots of the `Analysis` struct
/// which can be used to perform analysis.
///
/// With this struct we can reuse a lot of functionality from the compiler which should provide a
/// better user experience.
#[salsa::database(SourceDatabaseStorage, DefDatabaseStorage, HirDatabaseStorage)]
#[derive(Debug)]
pub(crate) struct AnalysisDatabase {
    runtime: salsa::Runtime<AnalysisDatabase>,
}

impl AnalysisDatabase {
    pub fn new() -> Self {
        let mut db = AnalysisDatabase {
            runtime: Default::default(),
        };

        db.set_target(Target::host_target().expect("could not determine host target spec"));

        db
    }
}

impl salsa::Database for AnalysisDatabase {
    fn salsa_runtime(&self) -> &Runtime<Self> {
        &self.runtime
    }
    fn salsa_runtime_mut(&mut self) -> &mut salsa::Runtime<AnalysisDatabase> {
        &mut self.runtime
    }
    fn salsa_event(&self, event: impl Fn() -> salsa::Event<AnalysisDatabase>) {
        match event().kind {
            salsa::EventKind::DidValidateMemoizedValue { .. }
            | salsa::EventKind::WillExecute { .. } => {
                self.check_canceled();
            }
            _ => (),
        }
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
            runtime: self.runtime.snapshot(self),
        })
    }
}
