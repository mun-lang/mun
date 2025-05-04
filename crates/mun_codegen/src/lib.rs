pub use crate::{
    code_gen::OptimizationLevel,
    db::{CodeGenDatabase, CodeGenDatabaseStorage},
    dispatch_table::{DispatchTable, FunctionPrototype},
    // assembly::{AssemblyIr, TargetAssembly},
    // code_gen::AssemblyBuilder,
    file_group::FileGroupData,
    intrinsics::Intrinsic,
    module_group::ModuleGroup,
    module_partition::{ModuleGroupId, ModulePartition},
    type_info::TypeId,
    type_table::TypeTable,
};

/// This library generates machine code from HIR using inkwell which is a safe
/// wrapper around LLVM.
mod code_gen;
mod db;
#[macro_use]
pub(crate) mod dispatch_table;
// #[macro_use]
// mod ir;
// mod assembly;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

mod apple;
pub mod file_group;
pub mod intrinsics;
// mod linker;
mod module_group;
mod module_partition;
mod ty;
pub(crate) mod type_info;
mod type_table;
