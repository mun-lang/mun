#[macro_use]
extern crate lazy_static;

mod code_gen;
mod db;
mod ir;
mod mock;
pub(crate) mod symbols;

#[cfg(test)]
mod test;

pub use inkwell::{builder, context::Context, module::Module, values, OptimizationLevel};

pub use crate::{
    code_gen::write_module_shared_object,
    db::{IrDatabase, IrDatabaseStorage},
};
