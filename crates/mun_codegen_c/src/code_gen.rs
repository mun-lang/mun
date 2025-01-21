use std::sync::Arc;

use mun_codegen::ModuleGroupId;

use crate::{db::CCodegenDatabase, HeaderAndSourceFiles};

/// The context used during C code generation.
pub struct CCodegenContext<'database> {
    /// The Salsa HIR database
    pub db: &'database dyn mun_hir::HirDatabase,
}

pub(crate) fn build_c_files(
    db: &dyn CCodegenDatabase,
    module_group: ModuleGroupId,
) -> Arc<HeaderAndSourceFiles> {
    let module_partition = db.module_partition();

    let module_group = &module_partition[module_group];
}
