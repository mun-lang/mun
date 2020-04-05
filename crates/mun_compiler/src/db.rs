use mun_hir::salsa;

#[salsa::database(
    mun_hir::SourceDatabaseStorage,
    mun_hir::DefDatabaseStorage,
    mun_hir::HirDatabaseStorage,
    mun_codegen::IrDatabaseStorage
)]
#[derive(Debug)]
pub(crate) struct CompilerDatabase {
    runtime: salsa::Runtime<CompilerDatabase>,
}

impl CompilerDatabase {
    pub fn new() -> Self {
        CompilerDatabase {
            runtime: salsa::Runtime::default(),
        }
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
