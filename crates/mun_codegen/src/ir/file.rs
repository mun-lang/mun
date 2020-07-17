use super::body::ExternalGlobals;
use crate::ir::{function, type_table::TypeTable};
use crate::value::Global;
use crate::{CodeGenParams, IrDatabase};
use hir::{FileId, ModuleDef};
use inkwell::module::Module;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

/// The IR generated for a single source file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileIR {
    /// The original source file
    pub file_id: FileId,
    /// The LLVM module that contains the IR
    pub llvm_module: Module,
    /// The `hir::Function`s that constitute the file's API.
    pub api: HashSet<hir::Function>,
}

/// Generates IR for the specified file.
pub(crate) fn ir_query(db: &dyn IrDatabase, file_id: FileId) -> Arc<FileIR> {
    let llvm_module = db
        .context()
        .create_module(db.file_relative_path(file_id).as_str());

    let group_ir = db.group_ir(file_id);

    // Generate all exposed function and wrapper function signatures.
    // Use a `BTreeMap` to guarantee deterministically ordered output.ures
    let mut functions = HashMap::new();
    let mut wrapper_functions = BTreeMap::new();
    for def in db.module_data(file_id).definitions() {
        if let ModuleDef::Function(f) = def {
            if !f.is_extern(db.upcast()) {
                let fun = function::gen_signature(
                    db,
                    *f,
                    &llvm_module,
                    CodeGenParams {
                        make_marshallable: false,
                    },
                );
                functions.insert(*f, fun);

                let fn_sig = f.ty(db.upcast()).callable_sig(db.upcast()).unwrap();
                if !f.data(db.upcast()).visibility().is_private()
                    && !fn_sig.marshallable(db.upcast())
                {
                    let wrapper_fun = function::gen_signature(
                        db,
                        *f,
                        &llvm_module,
                        CodeGenParams {
                            make_marshallable: true,
                        },
                    );
                    wrapper_functions.insert(*f, wrapper_fun);
                }
            }
        }
    }

    let external_globals = {
        let alloc_handle = group_ir
            .allocator_handle_type
            .map(|ty| llvm_module.add_global(ty, None, "allocatorHandle"));
        let dispatch_table = group_ir
            .dispatch_table
            .ty()
            .map(|ty| llvm_module.add_global(ty, None, "dispatchTable"));
        let type_table = if group_ir.type_table.is_empty() {
            None
        } else {
            Some(llvm_module.add_global(group_ir.type_table.ty(), None, TypeTable::NAME))
        };
        ExternalGlobals {
            alloc_handle,
            dispatch_table,
            type_table: type_table.map(|g| unsafe { Global::from_raw(g) }),
        }
    };

    // Construct requirements for generating the bodies
    let fn_pass_manager = function::create_pass_manager(&llvm_module, db.optimization_lvl());

    // Generate the function bodies
    for (hir_function, llvm_function) in functions.iter() {
        function::gen_body(
            db,
            (*hir_function, *llvm_function),
            &functions,
            &group_ir.dispatch_table,
            &group_ir.type_table,
            external_globals.clone(),
        );
        fn_pass_manager.run_on(llvm_function);
    }

    for (hir_function, llvm_function) in wrapper_functions.iter() {
        function::gen_wrapper_body(
            db,
            (*hir_function, *llvm_function),
            &functions,
            &group_ir.dispatch_table,
            &group_ir.type_table,
            external_globals.clone(),
        );
        fn_pass_manager.run_on(llvm_function);
    }

    // Filter private methods
    let api: HashSet<hir::Function> = functions
        .keys()
        .filter(|f| f.visibility(db.upcast()) != hir::Visibility::Private)
        .cloned()
        .collect();

    Arc::new(FileIR {
        file_id,
        llvm_module,
        api,
    })
}
