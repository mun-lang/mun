use crate::code_gen::CodeGenConfig;
use inkwell::context::Context;
use crate::ir::ty::TypeManager;
use crate::ir::{body::BodyIrGenerator, dispatch_table::DispatchTable, type_table::TypeTable};
use crate::values::FunctionValue;
use crate::{CodeGenParams, Module, OptimizationLevel};
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::types::AnyTypeEnum;

use super::body::ExternalGlobals;
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
    context: &Context,
    db: &impl hir::HirDatabase,
    type_manager: &mut TypeManager,
    f: hir::Function,
    module: &Module,
    params: CodeGenParams,
) -> FunctionValue {
    let name = {
        let name = f.name(db).to_string();
        if params.make_marshallable {
            format!("{}_wrapper", name)
        } else {
            name
        }
    };

    if let AnyTypeEnum::FunctionType(ty) = type_manager.type_ir(context, db, f.ty(db), params) {
        module.add_function(&name, ty, None)
    } else {
        panic!("not a function type")
    }
}

/// Generates the body of a `hir::Function` for an associated `FunctionValue`.
pub(crate) fn gen_body<'a, 'b, D: hir::HirDatabase>(
    context: &Context,
    config: &'a CodeGenConfig,
    db: &'a D,
    type_manager: &'a mut TypeManager,
    function: (hir::Function, FunctionValue),
    llvm_functions: &'a HashMap<hir::Function, FunctionValue>,
    dispatch_table: &'b DispatchTable,
    type_table: &'b TypeTable,
    external_globals: ExternalGlobals,
) {
    let mut code_gen = BodyIrGenerator::new(
        context,
        config,
        db,
        type_manager,
        function,
        llvm_functions,
        dispatch_table,
        type_table,
        CodeGenParams {
            make_marshallable: false,
        },
        external_globals,
    );

    code_gen.gen_fn_body();
}

/// Generates the body of a wrapper around `hir::Function` for its associated
/// `FunctionValue`
pub(crate) fn gen_wrapper_body<'a, 'b, D: hir::HirDatabase>(
    context: &Context,
    config: &'a CodeGenConfig,
    db: &'a D,
    type_manager: &mut TypeManager,
    function: (hir::Function, FunctionValue),
    llvm_functions: &'a HashMap<hir::Function, FunctionValue>,
    dispatch_table: &'b DispatchTable,
    type_table: &'b TypeTable,
    external_globals: ExternalGlobals,
) {
    let mut code_gen = BodyIrGenerator::new(
        context,
        config,
        db,
        type_manager,
        function,
        llvm_functions,
        dispatch_table,
        type_table,
        CodeGenParams {
            make_marshallable: true,
        },
        external_globals,
    );

    code_gen.gen_fn_wrapper();
}
