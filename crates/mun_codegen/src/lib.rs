#[macro_use]
extern crate lazy_static;

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
