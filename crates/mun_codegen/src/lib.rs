pub use inkwell::{builder::Builder, context::Context, module::Module, OptimizationLevel};

pub use crate::{
    assembly::{AssemblyIR, TargetAssembly},
    code_gen::ModuleBuilder,
    db::{CodeGenDatabase, CodeGenDatabaseStorage},
    module_group::ModuleGroup,
};

/// This library generates machine code from HIR using inkwell which is a safe wrapper around LLVM.
mod code_gen;
mod db;
#[macro_use]
mod ir;
mod assembly;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

pub mod value;

pub(crate) mod intrinsics;
mod linker;
mod module_group;
pub(crate) mod type_info;
