use c_codegen::{statement::Include, CFileBuilder};
use itertools::Itertools;
use mun_codegen::{FileGroupData, ModuleGroup, ModuleGroupId};
use mun_hir::{HirDatabase, ModuleDef};

use crate::{db::CCodegenDatabase, dispatch_table, type_table};
use crate::signatures::function_signature;

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
        generate_source(db, module_group, &file_group_data).expect("Invalid source code");

    source
}

fn generate_source(
    db: &dyn CCodegenDatabase,
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

    let type_table = type_table::maybe_generate_initialization(type_table);

    // Collect all the functions defined in the module group.
    let local_functions = module_group
        .iter()
        .flat_map(|module| module.declarations(db.upcast()))
        .filter_map(|decl| match decl {
            ModuleDef::Function(fun) => Some(fun),
            _ => None,
        }).collect::<Vec<_>>();

    let mut builder = CFileBuilder::default();

    // Generate function declarations of all the functions in the module group.
    for &function in &local_functions {
        builder.add_statement(function_signature(db, function));
    }

    // Generate the dispatch table
    let dispatch_table = dispatch_table::generate_initialization(module_group, dispatch_table, db.upcast());
    builder.add_statement(dispatch_table);

    // Generate the type table
    if let Some(type_table) = type_table {
        builder.add_statement(type_table);
    }

    builder.write_to_string().map_err(c_codegen::Error::Io)
}
