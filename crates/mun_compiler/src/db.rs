use mun_codegen::{CodeGenDatabase, CodeGenDatabaseStorage};
use mun_hir::{salsa, HirDatabase};

use crate::Config;

/// A compiler database is a salsa database that enables increment compilation.
#[salsa::database(
    mun_hir_input::SourceDatabaseStorage,
    mun_hir::InternDatabaseStorage,
    mun_hir::AstDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    CodeGenDatabaseStorage
)]
pub struct CompilerDatabase {
    storage: salsa::Storage<Self>,
}

impl CompilerDatabase {
    /// Constructs a new database
    pub fn new(config: &Config) -> Self {
        let mut db = CompilerDatabase {
            storage: salsa::Storage::default(),
        };

        // Set the initial configuration
        db.set_config(config);

        db
    }

    /// Applies the given configuration to the database
    pub fn set_config(&mut self, config: &Config) {
        self.set_target(config.target.clone());
        self.set_optimization_level(config.optimization_lvl);
    }
}

impl salsa::Database for CompilerDatabase {}
