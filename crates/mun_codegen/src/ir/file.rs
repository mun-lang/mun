use super::body::ExternalGlobals;
use crate::code_gen::CodeGenContext;
use crate::ir::body::BodyIrGenerator;
use crate::ir::file_group::FileGroupIR;
use crate::ir::{function, type_table::TypeTable};
use crate::value::Global;
use hir::{FileId, ModuleDef};
use inkwell::module::Module;
use std::collections::{BTreeMap, HashMap, HashSet};

/// The IR generated for a single source file.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileIR<'ink> {
    /// The original source file
    pub file_id: FileId,
    /// The LLVM module that contains the IR
    pub llvm_module: Module<'ink>,
    /// The `hir::Function`s that constitute the file's API.
    pub api: HashSet<hir::Function>,
}

/// Generates IR for the specified file.
pub(crate) fn gen_file_ir<'db, 'ink>(
    code_gen: &CodeGenContext<'db, 'ink>,
    group_ir: &FileGroupIR<'ink>,
    file_id: FileId,
) -> FileIR<'ink> {
    let llvm_module = code_gen
        .context
        .create_module(code_gen.db.file_relative_path(file_id).as_str());

    let hir_types = &code_gen.hir_types;

    // Generate all exposed function and wrapper function signatures.
    // Use a `BTreeMap` to guarantee deterministically ordered output.ures
    let mut functions = HashMap::new();
    let mut wrapper_functions = BTreeMap::new();
    for def in code_gen.db.module_data(file_id).definitions() {
        if let ModuleDef::Function(f) = def {
            if !f.is_extern(code_gen.db) {
                let fun = function::gen_prototype(code_gen.db, hir_types, *f, &llvm_module);
                functions.insert(*f, fun);

                let fn_sig = f.ty(code_gen.db).callable_sig(code_gen.db).unwrap();
                if !f.data(code_gen.db).visibility().is_private()
                    && !fn_sig.marshallable(code_gen.db)
                {
                    let wrapper_fun = function::gen_public_prototype(
                        code_gen.db,
                        &code_gen.hir_types,
                        *f,
                        &llvm_module,
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
    let fn_pass_manager = function::create_pass_manager(&llvm_module, code_gen.optimization_level);

    // Generate the function bodies
    for (hir_function, llvm_function) in functions.iter() {
        let mut code_gen = BodyIrGenerator::new(
            code_gen.context,
            code_gen.db,
            (*hir_function, *llvm_function),
            &functions,
            &group_ir.dispatch_table,
            &group_ir.type_table,
            external_globals.clone(),
            &code_gen.hir_types,
        );

        code_gen.gen_fn_body();
        fn_pass_manager.run_on(llvm_function);
    }

    for (hir_function, llvm_function) in wrapper_functions.iter() {
        let mut code_gen = BodyIrGenerator::new(
            code_gen.context,
            code_gen.db,
            (*hir_function, *llvm_function),
            &functions,
            &group_ir.dispatch_table,
            &group_ir.type_table,
            external_globals.clone(),
            &code_gen.hir_types,
        );

        code_gen.gen_fn_wrapper();
        fn_pass_manager.run_on(llvm_function);
    }

    // Filter private methods
    let api: HashSet<hir::Function> = functions
        .keys()
        .filter(|f| f.visibility(code_gen.db) != hir::Visibility::Private)
        .cloned()
        .collect();

    FileIR {
        file_id,
        llvm_module,
        api,
    }
}
