use crate::ir::{body::BodyIrGenerator, dispatch_table::DispatchTable, type_table::TypeTable};
use crate::values::{FunctionValue, GlobalValue};
use crate::{CodeGenParams, IrDatabase, Module, OptimizationLevel};
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::types::AnyTypeEnum;

use std::collections::HashMap;

/// Constructs a PassManager to optimize functions for the given optimization level.
pub(crate) fn create_pass_manager(
    module: &Module,
    optimization_lvl: OptimizationLevel,
) -> PassManager<FunctionValue> {
    let pass_builder = PassManagerBuilder::create();
    pass_builder.set_optimization_level(optimization_lvl);

    let function_pass_manager = PassManager::create(module);
    pass_builder.populate_function_pass_manager(&function_pass_manager);
    function_pass_manager.initialize();

    function_pass_manager
}

/// Generates a `FunctionValue` for a `hir::Function`. This function does not generate a body for
/// the `hir::Function`. That task is left to the `gen_body` function. The reason this is split
/// between two functions is that first all signatures are generated and then all bodies. This
/// allows bodies to reference `FunctionValue` wherever they are declared in the file.
pub(crate) fn gen_signature(
    db: &impl IrDatabase,
    f: hir::Function,
    module: &Module,
    params: CodeGenParams,
) -> FunctionValue {
    let name = f.name(db).to_string();
    if let AnyTypeEnum::FunctionType(ty) = db.type_ir(f.ty(db), params) {
        module.add_function(&name, ty, None)
    } else {
        panic!("not a function type")
    }
}

/// Generates the body of a `hir::Function` for an associated `FunctionValue`.
pub(crate) fn gen_body<'a, 'b, D: IrDatabase>(
    db: &'a D,
    hir_function: hir::Function,
    llvm_function: FunctionValue,
    module: &'a Module,
    llvm_functions: &'a HashMap<hir::Function, FunctionValue>,
    dispatch_table: &'b DispatchTable,
    type_table: &'b TypeTable,
    allocator_handle_global: Option<GlobalValue>,
) -> FunctionValue {
    let mut code_gen = BodyIrGenerator::new(
        db,
        module,
        hir_function,
        llvm_function,
        llvm_functions,
        dispatch_table,
        type_table,
        CodeGenParams {
            make_marshallable: false,
        },
        allocator_handle_global,
    );

    code_gen.gen_fn_body();

    llvm_function
}

/// Generates the body of a wrapper around `hir::Function` for its associated
/// `FunctionValue`
pub(crate) fn gen_wrapper_body<'a, 'b, D: IrDatabase>(
    db: &'a D,
    hir_function: hir::Function,
    llvm_function: FunctionValue,
    module: &'a Module,
    llvm_functions: &'a HashMap<hir::Function, FunctionValue>,
    dispatch_table: &'b DispatchTable,
    type_table: &'b TypeTable,
    allocator_handle_global: Option<GlobalValue>,
) -> FunctionValue {
    let mut code_gen = BodyIrGenerator::new(
        db,
        module,
        hir_function,
        llvm_function,
        llvm_functions,
        dispatch_table,
        type_table,
        CodeGenParams {
            make_marshallable: true,
        },
        allocator_handle_global,
    );

    code_gen.gen_fn_wrapper();

    llvm_function
}
