use std::sync::Arc;

use c_codegen::{identifier, statement::Include, CFileBuilder};
use mun_codegen::{ModuleGroup, ModuleGroupId};
use mun_hir::HirDatabase;

use crate::{db::CCodegenDatabase, dispatch_table, HeaderAndSourceFiles};

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

    let header = generate_header(db.upcast(), module_group);
    let source = generate_source(db.upcast(), module_group).expect("Invalid source code");

    Arc::new(HeaderAndSourceFiles { header, source })
}

fn generate_header(_db: &dyn HirDatabase, _module_group: &ModuleGroup) -> String {
    // for definition in module_group
    //     .iter()
    //     .flat_map(|module| module.declarations(db))
    // {
    //     match definition {
    //         ModuleDef::
    //     }
    // }

    String::new()
}

fn generate_source(
    db: &dyn CCodegenDatabase,
    module_group: &ModuleGroup,
) -> Result<String, identifier::Error> {
    let include = Include::with_quotes("dispatch_table.h");

    let file_group_data = db.file_group(key0)

    let dispatch_table = dispatch_table::generate_initialization(module_group, dispatch_table, db)?;

    CFileBuilder::default()
        .add_statement(include)
        .add_statement(dispatch_table)
        .write_to_string()
}
