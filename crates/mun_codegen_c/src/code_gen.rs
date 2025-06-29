use c_codegen::CFileBuilder;
use mun_codegen::{FileGroupData, ModuleGroup, ModuleGroupId};
use mun_hir::{ModuleDef, StructMemoryKind};
use mun_syntax::ast::StructKind;

use crate::signatures::function_signature;
use crate::structure;
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
    generate_source(db, module_group, &file_group_data).expect("Invalid source code")
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
        })
        .collect::<Vec<_>>();

    // Collect all the structs defined in the module group.
    let local_structs = module_group
        .iter()
        .flat_map(|module| module.declarations(db.upcast()))
        .filter_map(|decl| match decl {
            ModuleDef::Struct(s) => Some(s),
            _ => None,
        })
        .collect::<Vec<_>>();

    let mut builder = CFileBuilder::default();

    for structure in &local_structs {
        match structure.data(db.upcast()).memory_kind {
            StructMemoryKind::Gc => {
                // Generate GC struct declarations that can be used as pointers in function declarations.
                builder.add_statement(structure::declaration(db.upcast(), *structure));
            }
            StructMemoryKind::Value => {
                // Generate value struct definitions that can be used as values in function declarations.
                builder.add_statement(structure::definition(db.upcast(), *structure));
            }
        }
    }

    // Generate function declarations of all the functions in the module group.
    for function in &local_functions {
        builder.add_statement(function_signature(db, *function));
    }

    // Generate GC struct definitions that are required for the function definitions.
    for structure in &local_structs {
        if structure.data(db.upcast()).memory_kind == StructMemoryKind::Gc {
            builder.add_statement(structure::definition(db.upcast(), *structure));
        }
    }

    // Generate function definitions for all the functions in the module group.
    for function in &local_functions {}

    // Generate the dispatch table
    let dispatch_table =
        dispatch_table::generate_initialization(module_group, dispatch_table, db.upcast());
    builder.add_statement(dispatch_table);

    // Generate the type table
    if let Some(type_table) = type_table {
        builder.add_statement(type_table);
    }

    builder.write_to_string().map_err(c_codegen::Error::Io)
}
