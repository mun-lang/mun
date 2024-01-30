use mun_codegen::{CodeGenDatabase, CodeGenDatabaseStorage};
use mun_hir::{salsa, HirDatabase, Upcast};

use crate::Config;

/// A compiler database is a salsa database that enables increment compilation.
#[salsa::database(
    mun_hir::SourceDatabaseStorage,
    mun_hir::InternDatabaseStorage,
    mun_hir::AstDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    CodeGenDatabaseStorage
)]
pub struct CompilerDatabase {
    storage: salsa::Storage<Self>,
}

impl Upcast<dyn mun_hir::AstDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn mun_hir::AstDatabase + 'static) {
        self
    }
}

impl Upcast<dyn mun_hir::SourceDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn mun_hir::SourceDatabase + 'static) {
        self
    }
}

impl Upcast<dyn mun_hir::DefDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn mun_hir::DefDatabase + 'static) {
        self
    }
}

impl Upcast<dyn mun_hir::HirDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn mun_hir::HirDatabase + 'static) {
        self
    }
}

impl Upcast<dyn CodeGenDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn CodeGenDatabase + 'static) {
        self
    }
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
