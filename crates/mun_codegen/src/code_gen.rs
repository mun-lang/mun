pub use assembly_builder::AssemblyBuilder;
pub use context::CodeGenContext;
pub use error::CodeGenerationError;
use inkwell::{
    module::Module, passes::PassBuilderOptions, targets::TargetMachine, OptimizationLevel,
};
pub(crate) use object_file::ObjectFile;

mod assembly_builder;
mod context;
mod error;
mod object_file;
pub mod symbols;

/// Optimizes the specified LLVM `Module` using LLVM's default pipeline.
fn optimize_module(
    module: &Module<'_>,
    target_machine: &TargetMachine,
    optimization_lvl: OptimizationLevel,
) {
    let pipeline = match optimization_lvl {
        OptimizationLevel::None => "default<O0>",
        OptimizationLevel::Less => "default<O1>",
        OptimizationLevel::Default => "default<O2>",
        OptimizationLevel::Aggressive => "default<O3>",
    };
    module
        .run_passes(pipeline, target_machine, PassBuilderOptions::create())
        .expect("LLVM module optimization failed");
}
