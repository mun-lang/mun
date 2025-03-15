use std::sync::Arc;

use mun_hir::{HasVisibility as _, ModuleDef};
use rustc_hash::FxHashSet;

use crate::{
    ir::{
        dispatch_table::DispatchTableBuilder,
        intrinsics::{self, IntrinsicsSet},
        ty::HirTypeCache,
    },
    CodeGenDatabase, DispatchTable, ModuleGroupId,
};

pub type Data = FileGroupData;

#[derive(Debug, PartialEq, Eq)]
pub struct FileGroupData {
    pub dispatch_table: DispatchTable,
    pub intrinsics: IntrinsicsSet,
    pub referenced_modules: FxHashSet<mun_hir::Module>,
}

pub(super) fn build_file_group(
    db: &dyn CodeGenDatabase,
    module_group_id: ModuleGroupId,
) -> Arc<FileGroupData> {
    let module_partition = db.module_partition();
    let module_group = &module_partition[module_group_id];

    let mut intrinsics = IntrinsicsSet::new();
    let mut needs_alloc = false;

    let db = db.upcast();

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
                    &mut needs_alloc,
                    &f.body(db),
                    &f.infer(db),
                );

                let fn_sig = f.ty(db).callable_sig(db).unwrap();
                if f.visibility(db).is_externally_visible() && !fn_sig.marshallable(db) {
                    intrinsics::collect_wrapper_body(&mut intrinsics, &mut needs_alloc);
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

    let hir_types = HirTypeCache::new(db);

    // Collect all exposed functions' bodies.
    let mut dispatch_table_builder =
        DispatchTableBuilder::new(db, &intrinsics, &hir_types, module_group);
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

    let (dispatch_table, referenced_modules) = dispatch_table_builder.build();

    Arc::new(FileGroupData {
        dispatch_table,
        intrinsics,
        referenced_modules,
    })
}
