use std::sync::Arc;

use mun_hir::{HasVisibility as _, HirDatabase, ModuleDef};
use rustc_hash::FxHashSet;

use crate::{
    dispatch_table::{DispatchTableBuildOutput, DispatchTableBuilder},
    intrinsics::{self, IntrinsicsSet},
    ty::HirTypeCache,
    type_table::{TypeTable, TypeTableBuilder},
    CodeGenDatabase, DispatchTable, ModuleGroup, ModuleGroupId,
};

pub type Data = FileGroupData;

#[derive(Debug, PartialEq, Eq)]
pub struct FileGroupData {
    pub dispatch_table: DispatchTable,
    pub intrinsics: IntrinsicsSet,
    /// Whether the module group needs an allocator.
    pub needs_allocator: bool,
    pub referenced_modules: FxHashSet<mun_hir::Module>,
    pub type_table: TypeTable,
}

pub(super) fn build_file_group(
    db: &dyn CodeGenDatabase,
    module_group_id: ModuleGroupId,
) -> Arc<FileGroupData> {
    let module_partition = db.module_partition();
    let module_group = &module_partition[module_group_id];

    let db = db.upcast();

    let IntrinsicsData {
        intrinsics,
        needs_allocator,
    } = collect_intrinsics(db, module_group);

    let DispatchTableBuildOutput {
        dispatch_table,
        referenced_modules,
    } = collect_dispatch_table(db, module_group, &intrinsics);

    let type_table = collect_type_table(db, &dispatch_table, &HirTypeCache::new(db), module_group);

    Arc::new(FileGroupData {
        dispatch_table,
        intrinsics,
        needs_allocator,
        referenced_modules,
        type_table,
    })
}

fn collect_dispatch_table(
    db: &dyn HirDatabase,
    module_group: &ModuleGroup,
    intrinsics: &IntrinsicsSet,
) -> DispatchTableBuildOutput {
    let hir_types = HirTypeCache::new(db);

    // Collect all exposed functions' bodies.
    let mut dispatch_table_builder =
        DispatchTableBuilder::new(db, intrinsics, &hir_types, module_group);
    for def in module_group
        .iter()
        .flat_map(|module| module.declarations(db))
    {
        if let ModuleDef::Function(f) = def {
            // Find all functions that must be present in the dispatch table
            if !f.is_extern(db) {
                let body = f.body(db);
                let infer = f.infer(db);
                dispatch_table_builder.collect_body(&body, &infer);
            }
        }
    }

    dispatch_table_builder.build()
}

struct IntrinsicsData {
    intrinsics: IntrinsicsSet,
    needs_allocator: bool,
}

fn collect_intrinsics(db: &dyn HirDatabase, module_group: &ModuleGroup) -> IntrinsicsData {
    let mut intrinsics = IntrinsicsSet::new();
    let mut needs_allocator = false;

    // Collect all intrinsic functions, wrapper function, and generate struct
    // declarations.
    for def in module_group
        .iter()
        .flat_map(|module| module.declarations(db))
    {
        match def {
            ModuleDef::Function(f) if !f.is_extern(db) => {
                intrinsics::collect_fn_body(
                    db,
                    &mut intrinsics,
                    &mut needs_allocator,
                    &f.body(db),
                    &f.infer(db),
                );

                let fn_sig = f.ty(db).callable_sig(db).unwrap();
                if f.visibility(db).is_externally_visible() && !fn_sig.marshallable(db) {
                    intrinsics::collect_wrapper_body(&mut intrinsics, &mut needs_allocator);
                }
            }
            // TODO: Extern types for functions?
            ModuleDef::Module(_)
            | ModuleDef::Struct(_)
            | ModuleDef::PrimitiveType(_)
            | ModuleDef::TypeAlias(_)
            | ModuleDef::Function(_) => (),
        }
    }

    IntrinsicsData {
        intrinsics,
        needs_allocator,
    }
}

fn collect_type_table(
    db: &dyn HirDatabase,
    dispatch_table: &DispatchTable,
    hir_types: &HirTypeCache<'_>,
    module_group: &ModuleGroup,
) -> TypeTable {
    let mut type_table_builder = TypeTableBuilder::new(db, dispatch_table, hir_types, module_group);

    // Collect all used types
    for def in module_group
        .iter()
        .flat_map(|module| module.declarations(db))
    {
        match def {
            ModuleDef::Struct(s) => {
                type_table_builder.collect_struct(s);
            }
            ModuleDef::Function(f) => {
                type_table_builder.collect_fn(f);
            }
            ModuleDef::PrimitiveType(_) | ModuleDef::TypeAlias(_) | ModuleDef::Module(_) => (),
        }
    }

    type_table_builder.build()
}
