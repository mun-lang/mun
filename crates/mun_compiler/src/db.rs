use crate::Config;
use mun_codegen::IrDatabase;
use mun_hir::{salsa, HirDatabase};
use std::sync::Arc;

/// A compiler database is a salsa database that enables increment compilation.
#[salsa::database(
    mun_hir::SourceDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    mun_codegen::IrDatabaseStorage
)]
#[derive(Debug)]
pub struct CompilerDatabase {
    runtime: salsa::Runtime<CompilerDatabase>,
}

impl CompilerDatabase {
    /// Constructs a new database
    pub fn new(config: &Config) -> Self {
        let mut db = CompilerDatabase {
            runtime: salsa::Runtime::default(),
        };

        // Set the initial configuration
        db.set_context(Arc::new(mun_codegen::Context::create()));
        db.set_config(config);

        db
    }

    /// Applies the given configuration to the database
    pub fn set_config(&mut self, config: &Config) {
        self.set_target(config.target.clone());
        self.set_optimization_lvl(config.optimization_lvl);
    }
}

impl salsa::Database for CompilerDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime<CompilerDatabase> {
        &self.runtime
    }
    fn salsa_runtime_mut(&mut self) -> &mut salsa::Runtime<CompilerDatabase> {
        &mut self.runtime
    }
}
