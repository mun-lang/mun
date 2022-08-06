use crate::{ir::ty::HirTypeCache, Module, OptimizationLevel};
use inkwell::{
    passes::{PassManager, PassManagerBuilder},
    values::FunctionValue,
};

use mun_hir::HirDatabase;

/// Constructs a PassManager to optimize functions for the given optimization level.
pub(crate) fn create_pass_manager<'ink>(
    module: &Module<'ink>,
    optimization_lvl: OptimizationLevel,
) -> PassManager<FunctionValue<'ink>> {
    let pass_builder = PassManagerBuilder::create();
    pass_builder.set_optimization_level(optimization_lvl);

    let function_pass_manager = PassManager::create(module);
    pass_builder.populate_function_pass_manager(&function_pass_manager);
    function_pass_manager.initialize();

    function_pass_manager
}

/// Generates a `FunctionValue` for a `mun_hir::Function`. This function does not generate a body for
/// the `mun_hir::Function`. That task is left to the `gen_body` function. The reason this is split
/// between two functions is that first all signatures are generated and then all bodies. This
/// allows bodies to reference `FunctionValue` wherever they are declared in the file.
pub(crate) fn gen_prototype<'db, 'ink>(
    db: &'db dyn HirDatabase,
    types: &HirTypeCache<'db, 'ink>,
    func: mun_hir::Function,
    module: &Module<'ink>,
) -> FunctionValue<'ink> {
    let name = func.name(db).to_string();
    let ir_ty = types.get_function_type(func);
    module.add_function(&name, ir_ty, None)
}

/// Generates a `FunctionValue` for a `mun_hir::Function` that is usable from the public API. This
/// function does not generate a body for the `mun_hir::Function`. That task is left to the `gen_body`
/// function. The reason this is split between two functions is that first all signatures are
/// generated and then all bodies. This allows bodies to reference `FunctionValue` wherever they
/// are declared in the file.
pub(crate) fn gen_public_prototype<'db, 'ink>(
    db: &'db dyn HirDatabase,
    types: &HirTypeCache<'db, 'ink>,
    func: mun_hir::Function,
    module: &Module<'ink>,
) -> FunctionValue<'ink> {
    let name = format!("{}_wrapper", func.name(db));
    let ir_ty = types.get_public_function_type(func);
    module.add_function(&name, ir_ty, None)
}
