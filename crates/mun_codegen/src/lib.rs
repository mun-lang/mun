/// This library generates machine code from HIR using inkwell which is a safe wrapper around LLVM.

#[macro_use]
extern crate lazy_static;

#[macro_use] extern crate failure;

mod code_gen;
mod db;
mod ir;
pub(crate) mod symbols;
mod mock;

#[cfg(test)]
mod test;

pub use inkwell::{builder, context::Context, module::Module, values, OptimizationLevel};

pub use crate::{
    code_gen::write_module_shared_object,
    db::{IrDatabase, IrDatabaseStorage},
};
