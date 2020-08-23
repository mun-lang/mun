use inkwell::{
    module::Module,
    passes::{PassManager, PassManagerBuilder},
    OptimizationLevel,
};

mod bitcode_file;
mod context;
mod error;
mod module_builder;
pub mod symbols;

pub use context::CodeGenContext;
pub use error::CodeGenerationError;
pub use module_builder::ModuleBuilder;

/// Optimizes the specified LLVM `Module` using the default passes for the given
/// `OptimizationLevel`.
fn optimize_module(module: &Module, optimization_lvl: OptimizationLevel) {
    let pass_builder = PassManagerBuilder::create();
    pass_builder.set_optimization_level(optimization_lvl);

    let module_pass_manager = PassManager::create(());
    pass_builder.populate_module_pass_manager(&module_pass_manager);
    module_pass_manager.run_on(module);
}
