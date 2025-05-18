use c_codegen::{statement::Include, CFileBuilder};
use mun_codegen::{FileGroupData, ModuleGroup, ModuleGroupId};
use mun_hir::HirDatabase;

use crate::{db::CCodegenDatabase, dispatch_table, type_table};

/// The context used during C code generation.
pub struct CCodegenContext<'database> {
    /// The Salsa HIR database
    pub db: &'database dyn mun_hir::HirDatabase,
}

pub(crate) fn build_c_files(db: &dyn CCodegenDatabase, module_group_id: ModuleGroupId) -> String {
    let module_partition = db.module_partition();
    let module_group = &module_partition[module_group_id];

    let file_group_data = db.file_group(module_group_id);

    let source =
        generate_source(db.upcast(), module_group, &file_group_data).expect("Invalid source code");

    source
}

fn generate_source(
    db: &dyn HirDatabase,
    module_group: &ModuleGroup,
    file_group_data: &FileGroupData,
) -> c_codegen::Result<String> {
    let FileGroupData {
        dispatch_table,
        intrinsics,
        needs_allocator,
        referenced_modules,
        type_table,
    } = file_group_data;

    let dispatch_table = dispatch_table::generate_initialization(module_group, dispatch_table, db);
    let type_table = type_table::maybe_generate_initialization(type_table);

    let mut builder = CFileBuilder::default();
    builder.add_statement(dispatch_table);

    if let Some(type_table) = type_table {
        builder.add_statement(type_table);
    }

    builder.write_to_string().map_err(c_codegen::Error::Io)
}
