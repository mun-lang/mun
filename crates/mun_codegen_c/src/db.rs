use std::sync::Arc;

use mun_codegen::{CodeGenDatabase, ModuleGroupId};

use crate::HeaderAndSourceFiles;

#[salsa::query_group(CCodegenDatabaseStorage)]
pub trait CCodegenDatabase: CodeGenDatabase {
    #[salsa::invoke(crate::code_gen::build_c_files)]
    fn c_unit(&self, module_group: ModuleGroupId) -> Arc<HeaderAndSourceFiles>;
}
