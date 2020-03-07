use super::adt;
use crate::ir::dispatch_table::{DispatchTable, DispatchTableBuilder};
use crate::ir::function;
use crate::type_info::TypeInfo;
use crate::{CodeGenParams, IrDatabase};
use hir::{FileId, ModuleDef};
use inkwell::{module::Module, values::FunctionValue};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ModuleIR {
    /// The original source file
    pub file_id: FileId,

    /// The LLVM module that contains the IR
    pub llvm_module: Module,

    /// A mapping from HIR functions to LLVM IR values
    pub functions: HashMap<hir::Function, FunctionValue>,

    /// A set of unique TypeInfo values
    pub types: HashSet<TypeInfo>,

    /// The dispatch table
    pub dispatch_table: DispatchTable,
}

/// Generates IR for the specified file
pub(crate) fn ir_query(db: &impl IrDatabase, file_id: FileId) -> Arc<ModuleIR> {
    let llvm_module = db
        .context()
        .create_module(db.file_relative_path(file_id).as_str());

    // Collect type definitions for all used types
    let mut types = HashSet::new();

    for def in db.module_data(file_id).definitions() {
        match def {
            ModuleDef::Struct(s) => {
                let _t = adt::gen_struct_decl(db, *s);
                types.insert(db.type_info(s.ty(db)));
            }
            ModuleDef::BuiltinType(_) | ModuleDef::Function(_) => (),
        }
    }

    // Generate all the function signatures
    let mut functions = HashMap::new();
    let mut wrappers = HashMap::new();
    let mut dispatch_table_builder = DispatchTableBuilder::new(db, &llvm_module);
    for def in db.module_data(file_id).definitions() {
        // TODO: Remove once we have more ModuleDef variants
        #[allow(clippy::single_match)]
        match def {
            ModuleDef::Function(f) if !f.is_extern(db) => {
                // Collect argument types
                let fn_sig = f.ty(db).callable_sig(db).unwrap();
                for ty in fn_sig.params().iter() {
                    types.insert(db.type_info(ty.clone()));
                }
                // Collect return type
                let ret_ty = fn_sig.ret();
                if !ret_ty.is_empty() {
                    types.insert(db.type_info(ret_ty.clone()));
                }

                // Construct the function signature
                let fun = function::gen_signature(
                    db,
                    *f,
                    &llvm_module,
                    CodeGenParams { is_extern: false },
                );
                functions.insert(*f, fun);

                // Add calls to the dispatch table
                let body = f.body(db);
                let infer = f.infer(db);
                dispatch_table_builder.collect_body(&body, &infer);

                if f.data(db).visibility() != hir::Visibility::Private && !fn_sig.marshallable(db) {
                    let wrapper_fun = function::gen_signature(
                        db,
                        *f,
                        &llvm_module,
                        CodeGenParams { is_extern: true },
                    );
                    wrappers.insert(*f, wrapper_fun);

                    // Add calls from the function's wrapper to the dispatch table
                    dispatch_table_builder.collect_wrapper_body(*f);
                }
            }
            _ => {}
        }
    }

    // Construct requirements for generating the bodies
    let dispatch_table = dispatch_table_builder.finalize(&functions);
    let fn_pass_manager = function::create_pass_manager(&llvm_module, db.optimization_lvl());

    // Generate the function bodies
    for (hir_function, llvm_function) in functions.iter() {
        function::gen_body(
            db,
            *hir_function,
            *llvm_function,
            &llvm_module,
            &functions,
            &dispatch_table,
        );
        fn_pass_manager.run_on(llvm_function);
    }

    for (hir_function, llvm_function) in wrappers.iter() {
        function::gen_wrapper_body(
            db,
            *hir_function,
            *llvm_function,
            &llvm_module,
            &functions,
            &dispatch_table,
        );
        fn_pass_manager.run_on(llvm_function);
    }

    // Dispatch entries can include previously unchecked intrinsics
    for entry in dispatch_table.entries().iter() {
        // Collect argument types
        for ty in entry.prototype.arg_types.iter() {
            types.insert(ty.clone());
        }
        // Collect return type
        if let Some(ret_ty) = entry.prototype.ret_type.as_ref() {
            types.insert(ret_ty.clone());
        }
    }

    // Filter private methods
    let mut api: HashMap<hir::Function, FunctionValue> = functions
        .into_iter()
        .filter(|(f, _)| f.visibility(db) != hir::Visibility::Private)
        .collect();

    // Replace non-marshallable functions with their marshallable wrappers
    for (hir_function, llvm_function) in wrappers {
        api.insert(hir_function, llvm_function);
    }

    Arc::new(ModuleIR {
        file_id,
        llvm_module,
        functions: api,
        types,
        dispatch_table,
    })
}
