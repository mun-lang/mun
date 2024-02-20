pub use assembly_builder::AssemblyBuilder;
pub use context::CodeGenContext;
pub use error::CodeGenerationError;
use inkwell::{
    module::Module,
    passes::{PassManager, PassManagerBuilder},
    OptimizationLevel,
};
pub(crate) use object_file::ObjectFile;

mod assembly_builder;
mod context;
mod error;
mod object_file;
pub mod symbols;

/// Optimizes the specified LLVM `Module` using the default passes for the given
/// `OptimizationLevel`.
fn optimize_module(module: &Module<'_>, optimization_lvl: OptimizationLevel) {
    let pass_builder = PassManagerBuilder::create();
    pass_builder.set_optimization_level(optimization_lvl);

    let module_pass_manager = PassManager::create(());
    pass_builder.populate_module_pass_manager(&module_pass_manager);
    module_pass_manager.run_on(module);
}
