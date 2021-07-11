use crate::Config;
use hir::{salsa, HirDatabase, Upcast};
use mun_codegen::{CodeGenDatabase, CodeGenDatabaseStorage};

/// A compiler database is a salsa database that enables increment compilation.
#[salsa::database(
    hir::SourceDatabaseStorage,
    hir::InternDatabaseStorage,
    hir::AstDatabaseStorage,
    hir::DefDatabaseStorage,
    hir::HirDatabaseStorage,
    CodeGenDatabaseStorage
)]
pub struct CompilerDatabase {
    storage: salsa::Storage<Self>,
}

impl Upcast<dyn hir::AstDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn hir::AstDatabase + 'static) {
        &*self
    }
}

impl Upcast<dyn hir::SourceDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn hir::SourceDatabase + 'static) {
        &*self
    }
}

impl Upcast<dyn hir::DefDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn hir::DefDatabase + 'static) {
        &*self
    }
}

impl Upcast<dyn hir::HirDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn hir::HirDatabase + 'static) {
        &*self
    }
}

impl Upcast<dyn CodeGenDatabase> for CompilerDatabase {
    fn upcast(&self) -> &(dyn CodeGenDatabase + 'static) {
        &*self
    }
}

impl CompilerDatabase {
    /// Constructs a new database
    pub fn new(config: &Config) -> Self {
        let mut db = CompilerDatabase {
            storage: Default::default(),
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
