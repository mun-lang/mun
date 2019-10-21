use crate::ir::dispatch_table::{DispatchTable, DispatchTableBuilder};
use crate::ir::function;
use crate::IrDatabase;
use inkwell::{module::Module, values::FunctionValue};
use mun_hir::{FileId, ModuleDef};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ModuleIR {
    /// The original source file
    pub file_id: FileId,

    /// The LLVM module that contains the IR
    pub llvm_module: Module,

    /// A mapping from HIR functions to LLVM IR values
    pub functions: HashMap<mun_hir::Function, FunctionValue>,

    /// The dispatch table
    pub dispatch_table: DispatchTable,
}

/// Generates IR for the specified file
pub(crate) fn ir_query(db: &impl IrDatabase, file_id: FileId) -> Arc<ModuleIR> {
    let llvm_module = db
        .context()
        .create_module(db.file_relative_path(file_id).as_str());

    // Generate all the function signatures
    let mut functions = HashMap::new();
    let mut dispatch_table_builder = DispatchTableBuilder::new(db, &llvm_module);
    for def in db.module_data(file_id).definitions() {
        // TODO: Remove once we have more ModuleDef variants
        #[allow(clippy::single_match)]
        match def {
            ModuleDef::Function(f) => {
                // Construct the function signature
                let fun = function::gen_signature(db, *f, &llvm_module);
                functions.insert(*f, fun);

                // Add calls to the dispatch table
                let body = f.body(db);
                let infer = f.infer(db);
                dispatch_table_builder.collect_body(&body, &infer);
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

    Arc::new(ModuleIR {
        file_id,
        llvm_module,
        functions,
        dispatch_table,
    })
}
